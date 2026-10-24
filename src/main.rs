use eframe::egui;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
};
use walkdir::WalkDir;

#[derive(Clone)]
struct Entry {
    text: String,
    kind: &'static str,
}
enum Event {
    Log(Entry, Option<PathBuf>),
    Done,
}

struct App {
    zh: bool,
    folder: Option<PathBuf>,
    resize: bool,
    height: String,
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
        let zh = std::env::var("LANG")
            .unwrap_or_default()
            .to_lowercase()
            .starts_with("zh");
        Self {
            zh,
            folder: None,
            resize: false,
            height: "600".into(),
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
    fn tr(&self, zh: &'static str, en: &'static str) -> &'static str {
        if self.zh { zh } else { en }
    }
    fn scan(folder: &Path) -> Vec<PathBuf> {
        WalkDir::new(folder)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                matches!(
                    e.path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase()
                        .as_str(),
                    "webm" | "webp" | "mp4"
                )
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    }
    fn magick() -> Option<PathBuf> {
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
        None
    }
    fn ffmpeg() -> Option<PathBuf> {
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
        let Some(folder) = self.folder.clone() else {
            self.logs.push(Entry {
                text: self
                    .tr(
                        "請先選擇來源資料夾。",
                        "Please select a source folder first.",
                    )
                    .into(),
                kind: "warn",
            });
            return;
        };
        let mut files = Self::scan(&folder);
        if self.clip_mode {
            files.retain(|path| {
                path.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "webm" | "mp4"))
            });
        }
        if files.is_empty() {
            self.logs.push(Entry {
                text: self
                    .tr(
                        if self.clip_mode {
                            "未找到 WebM 或 MP4 影片。"
                        } else {
                            "未找到可轉換的檔案。"
                        },
                        if self.clip_mode {
                            "No WebM or MP4 videos found."
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
        let needs_magick = clip_range.is_none()
            && files.iter().any(|path| {
                path.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|ext| !ext.eq_ignore_ascii_case("mp4"))
            });
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
        let needs_ffmpeg = clip_range.is_some()
            || files.iter().any(|path| {
                path.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
            });
        let ffmpeg = if needs_ffmpeg {
            let Some(path) = Self::ffmpeg() else {
                self.logs.push(Entry {
                    text: self
                        .tr(
                            "MP4 與影片片段轉換需要 FFmpeg，請先安裝並加入 PATH。",
                            "MP4 and clip conversion require FFmpeg. Install it and add it to PATH.",
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
                let is_mp4 = input
                    .extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"));
                let result = if clip_range.is_some() || is_mp4 {
                    let height_filter = height
                        .map(|h| format!("scale=-1:{h}:flags=lanczos"))
                        .unwrap_or_else(|| "scale=iw:ih".into());
                    let filter = format!(
                        "fps=15,{height_filter},split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer[out]"
                    );
                    let mut cmd = Command::new(ffmpeg.as_ref().unwrap());
                    cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
                    if let Some((start, _)) = clip_range {
                        cmd.args(["-ss", &start.to_string()]);
                    }
                    cmd.arg("-i").arg(&input);
                    if let Some((start, end)) = clip_range {
                        cmd.args(["-t", &(end - start).to_string()]);
                    }
                    cmd.args(["-filter_complex", &filter, "-map", "[out]", "-loop", "0"])
                        .arg(&output)
                        .output()
                } else {
                    let mut cmd = Command::new(magick.as_ref().unwrap());
                    cmd.arg(&input);
                    if let Some(h) = height {
                        cmd.args(["-resize", &format!("x{h}")]);
                    }
                    cmd.arg(&output).output()
                };
                let (text, kind, out) = match result {
                    Ok(r) if r.status.success() => (
                        format!(
                            "✓  {}",
                            output.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        "ok",
                        Some(dir),
                    ),
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
        &[r"C:\Windows\Fonts\msyh.ttc", r"C:\Windows\Fonts\simhei.ttf"]
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
                    if !narrow {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::Frame::new().fill(egui::Color32::from_rgb(225, 245, 239)).corner_radius(egui::CornerRadius::same(16)).inner_margin(egui::Margin::symmetric(11, 6)).show(ui, |ui| {
                                ui.label(egui::RichText::new("WEBM / MP4 / WEBP  →  GIF").size(11.).strong().color(egui::Color32::from_rgb(0, 105, 82)));
                            });
                        });
                    }
                });
                ui.add_space(18.);

                egui::Frame::group(ui.style()).fill(egui::Color32::WHITE).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(229, 233, 238))).corner_radius(egui::CornerRadius::same(12)).inner_margin(egui::Margin::same(16)).show(ui, |ui| {
                    ui.label(egui::RichText::new(self.tr("來源資料夾", "SOURCE FOLDER")).size(11.).strong().color(egui::Color32::from_rgb(105, 117, 132)));
                    ui.add_space(8.);
                    let chosen = self.folder.is_some();
                    let source_ready = self.tr("已選擇來源", "SOURCE READY");
                    let source_hint = self.tr("選擇影片與動畫所在資料夾", "Select a folder with videos or animations");
                    let folder_text = self.folder.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| self.tr("尚未選擇資料夾", "Choose a folder to begin").into());
                    let browse_text = self.tr("瀏覽資料夾…", "Browse folders…");
                    let mut picker = |ui: &mut egui::Ui, full_width: bool| {
                        ui.add_enabled_ui(self.rx.is_none(), |ui| {
                            let button = egui::Button::new(egui::RichText::new(browse_text).strong())
                                .min_size(egui::vec2(if full_width { ui.available_width() } else { 118. }, 38.));
                            if ui.add(button).clicked() {
                                if let Some(p) = rfd::FileDialog::new().pick_folder() {
                                    self.folder = Some(p);
                                    let count = Self::scan(self.folder.as_ref().unwrap()).len();
                                    self.status = if self.zh { format!("找到 {count} 個檔案") } else { format!("{count} files found") };
                                }
                            }
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
                        ui.label(egui::RichText::new(self.tr("片段模式支援 WebM 與 MP4，並需要 FFmpeg。", "Clip mode supports WebM and MP4 and requires FFmpeg.")).size(11.).color(egui::Color32::from_rgb(125, 135, 147)));
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
                        let path = if self.outputs.len() == 1 { self.outputs.iter().next().cloned() } else { self.folder.clone() };
                        if let Some(p) = path {
                            #[cfg(target_os = "macos")] let _ = Command::new("open").arg(p).spawn();
                            #[cfg(target_os = "windows")] let _ = Command::new("explorer").arg(p).spawn();
                            #[cfg(all(unix, not(target_os = "macos")))] let _ = Command::new("xdg-open").arg(p).spawn();
                        }
                    }
                    if ui.button(egui::RichText::new(self.tr("清除紀錄", "Clear log")).color(egui::Color32::from_rgb(105, 117, 132))).clicked() {
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
                                let color = match e.kind { "ok" => egui::Color32::from_rgb(0, 115, 82), "err" => egui::Color32::from_rgb(180, 54, 54), _ => egui::Color32::from_rgb(105, 117, 132) };
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
                .with_inner_size([640., 760.])
                .with_min_inner_size([520., 620.]),
            ..Default::default()
        },
        Box::new(|cc| {
            configure_fonts(&cc.egui_ctx);
            Ok(Box::<App>::default())
        }),
    )
}
