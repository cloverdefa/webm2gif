#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
use eframe::egui;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        OnceLock,
        mpsc::{self, Receiver},
    },
    thread,
};
use walkdir::WalkDir;

#[cfg(embedded_tools)]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_tools.rs"));
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone)]
struct Entry {
    text: String,
    kind: &'static str,
}
enum Event {
    Log(Entry, Option<PathBuf>),
    Done,
}

/// 單次轉換的畫質參數；「目標大小」模式會逐輪調低這些值。
#[derive(Clone, Copy)]
struct Quality {
    /// 使用者指定的輸出高度（None = 保持原始尺寸）
    height: Option<u32>,
    /// 額外縮放係數（1.0 = 不縮），壓縮迴圈會逐輪縮小
    scale: f32,
    fps: u32,
    colors: u32,
}

impl Quality {
    fn target_height(&self) -> Option<u32> {
        self.height
            .map(|h| ((h as f32 * self.scale).round() as u32).max(16))
    }
}

fn is_supported(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase()
            .as_str(),
        "webm" | "webp" | "mp4" | "mov"
    )
}

fn is_video_ext(path: &Path) -> bool {
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

struct App {
    zh: bool,
    files: Vec<PathBuf>,
    /// 最近一次是以「單一資料夾」選擇來源時記錄，僅用於顯示路徑
    source_folder: Option<PathBuf>,
    resize: bool,
    height: String,
    fps: u32,
    colors: u32,
    target_on: bool,
    target_mb: String,
    clip_mode: bool,
    clip_start: String,
    clip_end: String,
    status: String,
    logs: Vec<Entry>,
    outputs: BTreeSet<PathBuf>,
    rx: Option<Receiver<Event>>,
    total: usize,
    done: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            // Windows 不會設定 LANG，以環境變數偵測會永遠落到英文，
            // 所以預設直接用中文，標題列右上角可切換。
            zh: true,
            files: vec![],
            source_folder: None,
            resize: false,
            height: "600".into(),
            fps: 15,
            colors: 256,
            target_on: false,
            target_mb: "5".into(),
            clip_mode: false,
            clip_start: "0".into(),
            clip_end: "5".into(),
            status: String::new(),
            logs: vec![],
            outputs: BTreeSet::new(),
            rx: None,
            total: 0,
            done: 0,
        }
    }
}

impl App {
    #[cfg(embedded_tools)]
    fn extract_embedded_tools(root: &Path) -> Option<()> {
        for (relative, bytes) in embedded::EMBEDDED_TOOLS {
            let destination = root.join(*relative);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).ok()?;
            }
            std::fs::write(destination, *bytes).ok()?;
        }
        Some(())
    }

    fn embedded_tools_dir() -> Option<PathBuf> {
        static TOOLS: OnceLock<Option<PathBuf>> = OnceLock::new();
        TOOLS
            .get_or_init(|| {
                #[cfg(embedded_tools)]
                {
                    let root = std::env::temp_dir()
                        .join(format!("webm2gif-tools-{}", env!("CARGO_PKG_VERSION")));
                    let magick = root.join("ImageMagick/magick.exe");
                    let ffmpeg = root.join("ffmpeg/ffmpeg.exe");
                    if magick.is_file() && ffmpeg.is_file() {
                        return Some(root);
                    }
                    if std::fs::create_dir_all(&root).is_err() {
                        return None;
                    }
                    Self::extract_embedded_tools(&root)?;
                    if magick.is_file() && ffmpeg.is_file() {
                        Some(root)
                    } else {
                        None
                    }
                }
                #[cfg(not(embedded_tools))]
                {
                    None
                }
            })
            .clone()
    }

    fn tr(&self, zh: &'static str, en: &'static str) -> &'static str {
        if self.zh { zh } else { en }
    }
    fn scan(folder: &Path) -> Vec<PathBuf> {
        WalkDir::new(folder)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_supported(e.path()))
            .map(|e| e.path().to_path_buf())
            .collect()
    }

    /// 加入來源：檔案直接加入，資料夾則遞迴掃描；replace = true 時先清空既有清單。
    fn add_paths(&mut self, paths: Vec<PathBuf>, replace: bool) {
        if replace {
            self.files.clear();
        }
        self.source_folder = if replace && paths.len() == 1 && paths[0].is_dir() {
            Some(paths[0].clone())
        } else {
            None
        };
        for p in paths {
            let found = if p.is_dir() {
                Self::scan(&p)
            } else if is_supported(&p) {
                vec![p]
            } else {
                vec![]
            };
            for f in found {
                if !self.files.contains(&f) {
                    self.files.push(f);
                }
            }
        }
        let count = self.files.len();
        self.status = if self.zh {
            format!("共 {count} 個檔案")
        } else {
            format!("{count} files found")
        };
    }

    fn source_text(&self) -> String {
        if let Some(f) = &self.source_folder {
            return f.display().to_string();
        }
        match self.files.len() {
            0 => self
                .tr("尚未選擇來源", "Choose files or a folder to begin")
                .into(),
            1 => self.files[0].display().to_string(),
            n => {
                if self.zh {
                    format!("已選擇 {n} 個檔案")
                } else {
                    format!("{n} files selected")
                }
            }
        }
    }

    fn magick() -> Option<PathBuf> {
        if let Some(root) = Self::embedded_tools_dir() {
            let path = root.join("ImageMagick/magick.exe");
            if path.is_file() {
                return Some(path);
            }
        }
        if let Some(exe_dir) = std::env::current_exe()
            .ok()?
            .parent()
            .map(Path::to_path_buf)
        {
            for root in [
                exe_dir.join("tools/ImageMagick"),
                exe_dir.join("ImageMagick-7.1.2-29-portable-Q16-x64"),
            ] {
                if let Some(path) = WalkDir::new(root)
                    .into_iter()
                    .filter_map(Result::ok)
                    .find(|e| {
                        e.file_type().is_file()
                            && e.file_name().to_string_lossy().eq_ignore_ascii_case(
                                if cfg!(windows) {
                                    "magick.exe"
                                } else {
                                    "magick"
                                },
                            )
                    })
                    .map(|e| e.path().to_path_buf())
                {
                    return Some(path);
                }
            }
        }
        if let Ok(path) = which("magick") {
            return Some(path);
        }
        // ImageMagick 6, still commonly packaged by Ubuntu releases used in WSL,
        // exposes the converter as `convert` instead of `magick`.
        if let Ok(path) = which("convert") {
            return Some(path);
        }
        None
    }
    fn ffmpeg() -> Option<PathBuf> {
        if let Some(root) = Self::embedded_tools_dir() {
            let path = root.join("ffmpeg/ffmpeg.exe");
            if path.is_file() {
                return Some(path);
            }
        }
        if let Some(exe_dir) = std::env::current_exe().ok()?.parent() {
            let bundled = exe_dir.join("tools/ffmpeg").join(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            });
            if bundled.is_file() {
                return Some(bundled);
            }
            let bundled_bin = exe_dir.join("tools/ffmpeg/bin").join(if cfg!(windows) {
                "ffmpeg.exe"
            } else {
                "ffmpeg"
            });
            if bundled_bin.is_file() {
                return Some(bundled_bin);
            }
        }
        which("ffmpeg").ok()
    }
    fn start(&mut self) {
        if self.files.is_empty() {
            self.logs.push(Entry {
                text: self
                    .tr(
                        "請先選擇來源檔案或資料夾。",
                        "Please select source files or a folder first.",
                    )
                    .into(),
                kind: "warn",
            });
            return;
        }
        let mut files = self.files.clone();
        if self.clip_mode {
            files.retain(|path| {
                path.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|ext| {
                        matches!(ext.to_ascii_lowercase().as_str(), "webm" | "mp4" | "mov")
                    })
            });
        }
        if files.is_empty() {
            self.logs.push(Entry {
                text: self
                    .tr(
                        if self.clip_mode {
                            "未找到 WebM、MP4 或 MOV 影片。"
                        } else {
                            "未找到可轉換的檔案。"
                        },
                        if self.clip_mode {
                            "No WebM, MP4, or MOV videos found."
                        } else {
                            "No convertible files found."
                        },
                    )
                    .into(),
                kind: "warn",
            });
            return;
        }
        let clip_range = if self.clip_mode {
            match (self.clip_start.parse::<f64>(), self.clip_end.parse::<f64>()) {
                (Ok(start), Ok(end)) if start >= 0.0 && end > start => Some((start, end)),
                _ => {
                    self.logs.push(Entry { text: self.tr("片段時間無效：結束時間需大於起始時間，且不可為負數。", "Invalid clip range: end time must be greater than start time, and neither value can be negative.").into(), kind: "warn" });
                    return;
                }
            }
        } else {
            None
        };
        let needs_magick = clip_range.is_none() && files.iter().any(|path| !is_video_ext(path));
        let magick = if needs_magick {
            let Some(path) = Self::magick() else {
                self.logs.push(Entry {
                    text: self
                        .tr(
                            "找不到 ImageMagick，請先安裝並加入 PATH。",
                            "ImageMagick was not found. Install it and add it to PATH.",
                        )
                        .into(),
                    kind: "err",
                });
                return;
            };
            Some(path)
        } else {
            None
        };
        let needs_ffmpeg = clip_range.is_some() || files.iter().any(|path| is_video_ext(path));
        let ffmpeg = if needs_ffmpeg {
            let Some(path) = Self::ffmpeg() else {
                self.logs.push(Entry {
                    text: self
                        .tr(
                            "MP4、MOV 與影片片段轉換需要 FFmpeg，請先安裝並加入 PATH。",
                            "MP4, MOV, and clip conversion require FFmpeg. Install it and add it to PATH.",
                        )
                        .into(),
                    kind: "err",
                });
                return;
            };
            Some(path)
        } else {
            None
        };
        let height = if self.resize {
            self.height
                .parse::<u32>()
                .ok()
                .filter(|h| (100..=2000).contains(h))
        } else {
            None
        };
        if self.resize && height.is_none() {
            self.logs.push(Entry {
                text: self
                    .tr(
                        "高度需為 100 到 2000 之間的整數。",
                        "Height must be a whole number from 100 to 2000.",
                    )
                    .into(),
                kind: "warn",
            });
            return;
        }
        let target_bytes = if self.target_on {
            match self.target_mb.trim().parse::<f64>() {
                Ok(mb) if mb >= 0.1 => Some((mb * 1_048_576.0) as u64),
                _ => {
                    self.logs.push(Entry {
                        text: self
                            .tr(
                                "目標大小需為不小於 0.1 的數字（MB）。",
                                "Target size must be a number of at least 0.1 (MB).",
                            )
                            .into(),
                        kind: "warn",
                    });
                    return;
                }
            }
        } else {
            None
        };
        let (zh, fps, colors) = (self.zh, self.fps, self.colors);
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.outputs.clear();
        self.total = files.len();
        self.done = 0;
        self.status = self.tr("轉換中…", "Converting…").into();
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
                        let name = output.file_name().unwrap_or_default().to_string_lossy().to_string();
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
}

fn which(name: &str) -> Result<PathBuf, ()> {
    let key = if cfg!(windows) { "PATH" } else { "PATH" };
    let paths: Vec<PathBuf> = std::env::var_os(key)
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    for p in paths {
        let candidate = p.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        #[cfg(windows)]
        {
            let exe = p.join(format!("{name}.exe"));
            if exe.is_file() {
                return Ok(exe);
            }
        }
    }
    Err(())
}

fn configure_fonts(ctx: &egui::Context) {
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/PingFang.ttc",
        ]
    } else if cfg!(windows) {
        // 微軟正黑體（繁體）優先，其次微軟雅黑、黑體
        &[
            r"C:\Windows\Fonts\msjh.ttc",
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\simhei.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        ]
    };

    let Some(bytes) = candidates.iter().find_map(|path| std::fs::read(path).ok()) else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".into(), egui::FontData::from_owned(bytes).into());
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "cjk".into());
    }
    ctx.set_fonts(fonts);
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = egui::Color32::from_rgb(247, 248, 250);
        visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(247, 248, 250);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(241, 244, 247);
        visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(54, 65, 78);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(225, 245, 239);
        visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(0, 105, 82);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 105, 82);
        visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;
        visuals.selection.bg_fill = egui::Color32::from_rgb(0, 137, 105);
        visuals.selection.stroke.color = egui::Color32::WHITE;
        ctx.set_visuals(visuals);

        // 拖曳檔案 / 資料夾到視窗（轉換中不處理）
        if self.rx.is_none() {
            let dropped: Vec<PathBuf> = ctx.input(|i| {
                i.raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect()
            });
            if !dropped.is_empty() {
                let replace = self.files.is_empty();
                self.add_paths(dropped, replace);
            }
        }

        let mut finished = false;
        if let Some(rx) = &self.rx {
            while let Ok(ev) = rx.try_recv() {
                match ev {
                    Event::Log(e, p) => {
                        self.logs.push(e);
                        if let Some(p) = p {
                            self.outputs.insert(p);
                        }
                        self.done += 1;
                    }
                    Event::Done => {
                        self.status = if self.zh { "完成" } else { "Done" }.into();
                        finished = true;
                        break;
                    }
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        if finished {
            self.rx = None;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let narrow = ui.available_width() < 570.0;
                ui.add_space(6.);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(egui::RichText::new("Webm2Gif").size(27.).strong().color(egui::Color32::from_rgb(30, 41, 55)));
                        ui.label(egui::RichText::new(self.tr("影片轉 GIF 工具", "Video to GIF Converter")).color(egui::Color32::from_rgb(105, 117, 132)));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 語言切換
                        if ui.small_button(if self.zh { "EN" } else { "中文" }).clicked() {
                            self.zh = !self.zh;
                        }
                        if !narrow {
                            egui::Frame::new().fill(egui::Color32::from_rgb(225, 245, 239)).corner_radius(egui::CornerRadius::same(16)).inner_margin(egui::Margin::symmetric(11, 6)).show(ui, |ui| {
                                ui.label(egui::RichText::new("WEBM / WEBP / MP4 / MOV  →  GIF").size(10.).strong().color(egui::Color32::from_rgb(0, 105, 82)));
                            });
                        }
                    });
                });
                ui.add_space(18.);

                egui::Frame::group(ui.style()).fill(egui::Color32::WHITE).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(229, 233, 238))).corner_radius(egui::CornerRadius::same(12)).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                    ui.label(egui::RichText::new(self.tr("來源", "SOURCE")).size(11.).strong().color(egui::Color32::from_rgb(105, 117, 132)));
                    ui.add_space(8.);
                    let chosen = self.source_folder.is_some() || !self.files.is_empty();
                    let source_ready = self.tr("已選擇來源", "SOURCE READY");
                    let source_hint = self.tr("選擇影片／動畫檔案或資料夾，也可直接拖曳到視窗", "Select files or a folder, or drag and drop here");
                    let folder_text = self.source_text();
                    let pick_files_text = self.tr("選擇檔案…", "Choose files…");
                    let pick_folder_text = self.tr("選擇資料夾…", "Choose folder…");
                    let filter_name = self.tr("影片 / 動畫", "Videos / animations");
                    let mut picker = |ui: &mut egui::Ui, full_width: bool| {
                        ui.add_enabled_ui(self.rx.is_none(), |ui| {
                            let width = if full_width { ui.available_width() } else { 128. };
                            ui.vertical(|ui| {
                                let files_btn = egui::Button::new(egui::RichText::new(pick_files_text).strong())
                                    .min_size(egui::vec2(width, 38.));
                                if ui.add(files_btn).clicked() {
                                    if let Some(paths) = rfd::FileDialog::new()
                                        .add_filter(filter_name, &["webm", "webp", "mp4", "mov"])
                                        .pick_files()
                                    {
                                        self.add_paths(paths, true);
                                    }
                                }
                                let folder_btn = egui::Button::new(egui::RichText::new(pick_folder_text).strong())
                                    .min_size(egui::vec2(width, 38.));
                                if ui.add(folder_btn).clicked() {
                                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                        self.add_paths(vec![p], true);
                                    }
                                }
                            });
                        });
                    };
                    if narrow {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(if chosen { source_ready } else { source_hint }).size(12.).color(egui::Color32::from_rgb(105, 117, 132)));
                            ui.add(egui::Label::new(egui::RichText::new(folder_text).size(14.).strong().color(egui::Color32::from_rgb(38, 49, 63))).wrap());
                            ui.add_space(8.);
                            picker(ui, true);
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(if chosen { source_ready } else { source_hint }).size(12.).color(egui::Color32::from_rgb(105, 117, 132)));
                                ui.add_space(3.);
                                ui.add(egui::Label::new(egui::RichText::new(folder_text).size(14.).strong().color(egui::Color32::from_rgb(38, 49, 63))).truncate());
                            });
                            ui.add_space(6.);
                            picker(ui, false);
                        });
                    }
                });

                ui.add_space(12.);
                egui::Frame::group(ui.style()).fill(egui::Color32::WHITE).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(229, 233, 238))).corner_radius(egui::CornerRadius::same(12)).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                    ui.label(egui::RichText::new(self.tr("轉換設定", "CONVERSION SETTINGS")).size(11.).strong().color(egui::Color32::from_rgb(105, 117, 132)));
                    ui.add_space(8.);
                    if narrow {
                        ui.vertical(|ui| {
                            let resize_label = self.tr("調整輸出高度", "Resize output height");
                            ui.checkbox(&mut self.resize, resize_label);
                            ui.horizontal(|ui| {
                                ui.add_enabled(self.resize, egui::TextEdit::singleline(&mut self.height).desired_width(78.).margin(egui::Margin::symmetric(8, 7)));
                                ui.label(egui::RichText::new("px  ·  100–2000").color(egui::Color32::from_rgb(105, 117, 132)));
                            });
                        });
                    } else {
                        ui.horizontal(|ui| {
                            let resize_label = self.tr("調整輸出高度", "Resize output height");
                            ui.checkbox(&mut self.resize, resize_label);
                            ui.add_enabled(self.resize, egui::TextEdit::singleline(&mut self.height).desired_width(68.).margin(egui::Margin::symmetric(8, 7)));
                            ui.label(egui::RichText::new("px  ·  100–2000").color(egui::Color32::from_rgb(105, 117, 132)));
                        });
                    }
                    ui.label(egui::RichText::new(self.tr("關閉時保留原始尺寸，開啟時維持比例縮放。", "Leave off to keep the original size; resizing preserves the aspect ratio.")).size(11.).color(egui::Color32::from_rgb(125, 135, 147)));

                    // ── 檔案大小控制：影格率 / 色彩數 / 目標大小 ──
                    ui.add_space(8.);
                    ui.separator();
                    ui.add_space(4.);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(self.tr("影格率", "Frame rate"));
                        ui.add(egui::Slider::new(&mut self.fps, 5..=30).suffix(" fps"));
                        ui.add_space(10.);
                        ui.label(self.tr("色彩數", "Colors"));
                        egui::ComboBox::from_id_salt("colors")
                            .selected_text(self.colors.to_string())
                            .show_ui(ui, |ui| {
                                for c in [256u32, 128, 64, 32] {
                                    ui.selectable_value(&mut self.colors, c, c.to_string());
                                }
                            });
                    });
                    let target_label = self.tr("限制輸出檔案大小", "Limit output file size");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.target_on, target_label);
                        ui.add_enabled(self.target_on, egui::TextEdit::singleline(&mut self.target_mb).desired_width(68.).margin(egui::Margin::symmetric(8, 7)));
                        ui.label(egui::RichText::new("MB").color(egui::Color32::from_rgb(105, 117, 132)));
                    });
                    ui.label(egui::RichText::new(self.tr("降低影格率、縮小尺寸或減少色彩數都能縮小檔案；啟用大小限制後會自動逐步壓縮到低於目標（較耗時）。", "Lower frame rate, smaller size, or fewer colors all shrink the file. With a size limit, files are re-encoded with progressively lower settings until under the target (slower).")).size(11.).color(egui::Color32::from_rgb(125, 135, 147)));

                    ui.add_space(8.);
                    ui.separator();
                    let clip_label = self.tr("只轉換影片片段", "Convert a video clip only");
                    ui.checkbox(&mut self.clip_mode, clip_label);
                    if self.clip_mode {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(self.tr("開始（秒）", "Start (sec)"));
                            ui.add(egui::TextEdit::singleline(&mut self.clip_start).desired_width(72.));
                            ui.label(self.tr("結束（秒）", "End (sec)"));
                            ui.add(egui::TextEdit::singleline(&mut self.clip_end).desired_width(72.));
                        });
                        ui.label(egui::RichText::new(self.tr("片段模式支援 WebM、MP4 與 MOV，並需要 FFmpeg。", "Clip mode supports WebM, MP4, and MOV and requires FFmpeg.")).size(11.).color(egui::Color32::from_rgb(125, 135, 147)));
                    }
                });

                ui.add_space(14.);
                let mut action_buttons = |ui: &mut egui::Ui, stacked: bool| {
                    let converting = self.rx.is_some();
                    let convert = egui::Button::new(egui::RichText::new(if converting { self.tr("正在轉換…", "Converting…") } else { self.tr("▶   開始轉換", "▶   Start converting") }).size(16.).strong().color(egui::Color32::WHITE))
                        .fill(if converting { egui::Color32::from_rgb(112, 170, 153) } else { egui::Color32::from_rgb(0, 137, 105) })
                        .min_size(egui::vec2(if stacked { ui.available_width() } else { 220. }, 50.))
                        .corner_radius(egui::CornerRadius::same(10));
                    if ui.add_enabled(!converting, convert).clicked() { self.start(); }
                    let output_button = egui::Button::new(egui::RichText::new(self.tr("開啟輸出", "Open output")).strong()).min_size(egui::vec2(if stacked { ui.available_width() } else { 120. }, 42.));
                    if ui.add_enabled(!self.outputs.is_empty(), output_button).clicked() {
                        if let Some(p) = self.outputs.iter().next().cloned() {
                            #[cfg(target_os = "macos")] let _ = Command::new("open").arg(p).spawn();
                            #[cfg(target_os = "windows")] let _ = Command::new("explorer").arg(p).spawn();
                            #[cfg(all(unix, not(target_os = "macos")))] let _ = Command::new("xdg-open").arg(p).spawn();
                        }
                    }
                    let clear = egui::Button::new(egui::RichText::new(self.tr("清除紀錄", "Clear log")).strong().color(egui::Color32::from_rgb(156, 48, 48)))
                        .fill(egui::Color32::from_rgb(255, 235, 235))
                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(239, 185, 185)));
                    if ui.add(clear).clicked() {
                        self.logs.clear(); self.outputs.clear(); self.done = 0; self.status = self.tr("就緒", "Ready").into();
                    }
                };
                if narrow {
                    ui.vertical(|ui| action_buttons(ui, true));
                } else {
                    ui.horizontal_wrapped(|ui| action_buttons(ui, false));
                }

                ui.add_space(12.);
                egui::Frame::group(ui.style()).fill(egui::Color32::WHITE).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(229, 233, 238))).corner_radius(egui::CornerRadius::same(12)).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(self.tr("轉換進度", "PROGRESS")).size(11.).strong().color(egui::Color32::from_rgb(105, 117, 132)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(format!("{}/{}", self.done, self.total)).strong().color(egui::Color32::from_rgb(0, 105, 82))); });
                    });
                    ui.add_space(7.);
                    ui.add(egui::ProgressBar::new(if self.total == 0 { 0. } else { self.done as f32 / self.total as f32 }).desired_height(10.).fill(egui::Color32::from_rgb(0, 137, 105)).text(self.status.clone()));
                    ui.add_space(14.);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(self.tr("活動紀錄", "ACTIVITY")).size(11.).strong().color(egui::Color32::from_rgb(105, 117, 132)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(format!("{} {}", self.logs.len(), self.tr("筆", "items"))).size(11.).color(egui::Color32::from_rgb(125, 135, 147))); });
                    });
                    ui.add_space(7.);
                    egui::Frame::new().fill(egui::Color32::from_rgb(248, 249, 251)).corner_radius(egui::CornerRadius::same(8)).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(170.).stick_to_bottom(true).show(ui, |ui| {
                            if self.logs.is_empty() { ui.label(egui::RichText::new(self.tr("轉換結果會顯示在這裡。", "Conversion results will appear here.")).color(egui::Color32::from_rgb(145, 153, 163))); }
                            for e in &self.logs {
                                let color = match e.kind { "ok" => egui::Color32::from_rgb(0, 115, 82), "err" => egui::Color32::from_rgb(180, 54, 54), "warn" => egui::Color32::from_rgb(176, 108, 0), _ => egui::Color32::from_rgb(105, 117, 132) };
                                ui.colored_label(color, &e.text);
                            }
                        });
                    });
                });
                ui.add_space(8.);
            });
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Webm2Gif",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([640., 780.])
                .with_min_inner_size([520., 640.])
                .with_drag_and_drop(true)
                .with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!("../assets/app-icon.png"))
                        .expect("invalid app icon PNG"),
                ),
            ..Default::default()
        },
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::<App>::default())
        }),
    )
}
