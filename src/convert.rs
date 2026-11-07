//! 轉換核心：畫質參數、ImageMagick / FFmpeg 呼叫，以及批次轉換執行緒。

use crate::model::{Entry, Event};
#[cfg(windows)]
use crate::platform::CREATE_NO_WINDOW;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Sender,
    thread,
};

/// 單次轉換的畫質參數；「目標大小」模式會逐輪調低這些值。
#[derive(Clone, Copy)]
pub struct Quality {
    /// 使用者指定的輸出高度（None = 保持原始尺寸）
    pub height: Option<u32>,
    /// 額外縮放係數（1.0 = 不縮），壓縮迴圈會逐輪縮小
    pub scale: f32,
    pub fps: u32,
    pub colors: u32,
}

impl Quality {
    fn target_height(&self) -> Option<u32> {
        self.height
            .map(|h| ((h as f32 * self.scale).round() as u32).max(16))
    }
}

pub fn is_supported(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "webm" | "webp" | "mp4" | "mov"
    )
}

pub fn is_video_ext(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "mp4" | "mov"
    )
}

/// 取得動態圖每一格的延遲（單位 1/100 秒）；單格或失敗時回傳 None。
fn frame_delays(magick: &Path, input: &Path) -> Option<Vec<u32>> {
    let is_convert = magick
        .file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("convert"));
    let mut cmd = if is_convert {
        // ImageMagick 6：identify 是獨立執行檔
        Command::new(magick.with_file_name(if cfg!(windows) {
            "identify.exe"
        } else {
            "identify"
        }))
    } else {
        let mut c = Command::new(magick);
        c.arg("identify");
        c
    };
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let out = cmd.args(["-format", "%T\n"]).arg(input).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let delays: Vec<u32> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect();
    (delays.len() > 1).then_some(delays)
}

/// 依目標 FPS 規劃要刪除的影格；回傳 (要刪除的索引, 新的每格延遲)。
/// 目標 FPS 不低於原始 FPS 時回傳 None（不抽格）。
fn plan_frame_drop(delays: &[u32], target_fps: u32) -> Option<(Vec<usize>, u32)> {
    let delays: Vec<f64> = delays
        .iter()
        .map(|&d| if d == 0 { 10.0 } else { d as f64 })
        .collect();
    let total: f64 = delays.iter().sum();
    let orig_fps = 100.0 * delays.len() as f64 / total;
    if (target_fps as f64) >= orig_fps * 0.95 {
        return None;
    }
    let step = 100.0 / target_fps as f64;
    let (mut t, mut next) = (0.0_f64, 0.0_f64);
    let mut drops = vec![];
    for (i, d) in delays.iter().enumerate() {
        if t + 1e-9 >= next {
            next += step;
        } else {
            drops.push(i);
        }
        t += d;
    }
    Some((drops, step.round().max(2.0) as u32))
}

fn run_convert(
    input: &Path,
    output: &Path,
    q: Quality,
    clip_range: Option<(f64, f64)>,
    ffmpeg: Option<&Path>,
    magick: Option<&Path>,
) -> std::io::Result<std::process::Output> {
    if clip_range.is_some() || is_video_ext(input) {
        let scale_filter = match (q.target_height(), q.scale < 0.999) {
            (Some(h), _) => format!("scale=-1:{h}:flags=lanczos"),
            (None, true) => format!("scale=trunc(iw*{:.3}):-1:flags=lanczos", q.scale),
            (None, false) => "scale=iw:ih".into(),
        };
        let filter = format!(
            "fps={fps},{scale_filter},split[s0][s1];[s0]palettegen=stats_mode=diff:max_colors={colors}[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle[out]",
            fps = q.fps,
            colors = q.colors
        );
        let mut cmd = Command::new(ffmpeg.unwrap());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
        if let Some((start, _)) = clip_range {
            cmd.args(["-ss", &start.to_string()]);
        }
        cmd.arg("-i").arg(input);
        if let Some((start, end)) = clip_range {
            cmd.args(["-t", &(end - start).to_string()]);
        }
        cmd.args(["-filter_complex", &filter, "-map", "[out]", "-loop", "0"])
            .arg(output)
            .output()
    } else {
        let magick = magick.unwrap();
        let mut cmd = Command::new(magick);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd.arg(input).arg("-coalesce");
        // 抽格：依目標 FPS 刪除部分影格，並把剩餘影格的延遲拉長以維持播放速度
        if let Some(delays) = frame_delays(magick, input) {
            if let Some((drops, delay)) = plan_frame_drop(&delays, q.fps) {
                let list = drops
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                cmd.args(["-delete", &list, "-set", "delay", &delay.to_string()]);
            }
        }
        if let Some(h) = q.target_height() {
            cmd.args(["-resize", &format!("x{h}")]);
        } else if q.scale < 0.999 {
            cmd.args(["-resize", &format!("{:.1}%", q.scale * 100.0)]);
        }
        cmd.args([
            "-colors",
            &q.colors.to_string(),
            "-layers",
            "Optimize",
            "-loop",
            "0",
        ]);
        cmd.arg(output).output()
    }
}

/// 檔案仍超過目標大小時，產生下一輪更小的參數；到達下限回傳 None。
fn shrink(q: Quality, round: u32) -> Option<Quality> {
    let scale = q.scale * 0.85;
    if round >= 8 || scale < 0.25 {
        return None;
    }
    Some(Quality {
        scale,
        fps: ((q.fps as f32 * 0.85).round() as u32).max(6).min(q.fps),
        colors: if round >= 2 && q.colors > 64 {
            q.colors / 2
        } else {
            q.colors
        },
        ..q
    })
}

/// 一次批次轉換所需的全部參數（已驗證）。
pub struct BatchParams {
    pub files: Vec<PathBuf>,
    pub height: Option<u32>,
    pub fps: u32,
    pub colors: u32,
    pub target_bytes: Option<u64>,
    pub clip_range: Option<(f64, f64)>,
    pub ffmpeg: Option<PathBuf>,
    pub magick: Option<PathBuf>,
    pub zh: bool,
}

/// 在背景執行緒逐檔轉換，進度以 `Event` 回報。
pub fn spawn_batch(params: BatchParams, tx: Sender<Event>) {
    let BatchParams {
        files,
        height,
        fps,
        colors,
        target_bytes,
        clip_range,
        ffmpeg,
        magick,
        zh,
    } = params;
    thread::spawn(move || {
        for input in files {
            let dir = input.parent().unwrap_or(Path::new(".")).join("gif");
            let _ = std::fs::create_dir_all(&dir);
            let output = dir
                .join(input.file_stem().unwrap_or_default())
                .with_extension("gif");
            let mut q = Quality {
                height,
                scale: 1.0,
                fps,
                colors,
            };
            let mut result = run_convert(
                &input,
                &output,
                q,
                clip_range,
                ffmpeg.as_deref(),
                magick.as_deref(),
            );
            // 指定目標大小：超過就逐輪降低尺寸 / FPS / 色彩數重新轉換
            let mut over_limit = false;
            if let Some(limit) = target_bytes {
                let mut round = 0;
                while matches!(&result, Ok(r) if r.status.success()) {
                    let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                    if size <= limit {
                        break;
                    }
                    match shrink(q, round) {
                        Some(next) => {
                            q = next;
                            round += 1;
                            result = run_convert(
                                &input,
                                &output,
                                q,
                                clip_range,
                                ffmpeg.as_deref(),
                                magick.as_deref(),
                            );
                        }
                        None => {
                            over_limit = true;
                            break;
                        }
                    }
                }
            }
            let (text, kind, out) = match result {
                Ok(r) if r.status.success() => {
                    let name = output
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let mb = std::fs::metadata(&output)
                        .map(|m| m.len() as f64 / 1_048_576.0)
                        .unwrap_or(0.0);
                    if over_limit {
                        (
                            format!(
                                "⚠  {name} ({mb:.2} MB) — {}",
                                if zh {
                                    "已達壓縮下限，仍超過目標大小"
                                } else {
                                    "reached minimum quality, still above target"
                                }
                            ),
                            "warn",
                            Some(dir),
                        )
                    } else {
                        (format!("✓  {name} ({mb:.2} MB)"), "ok", Some(dir))
                    }
                }
                Ok(r) => (
                    format!(
                        "✗  {}: {}",
                        input.display(),
                        String::from_utf8_lossy(&r.stderr).trim()
                    ),
                    "err",
                    None,
                ),
                Err(e) => (format!("✗  {}: {e}", input.display()), "err", None),
            };
            let _ = tx.send(Event::Log(Entry { text, kind }, out));
        }
        let _ = tx.send(Event::Done);
    });
}
