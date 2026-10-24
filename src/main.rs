use eframe::egui;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
};
use walkdir::WalkDir;

#[cfg(windows)]
const EXTRACT_DIR: &str = "ImageMagick-7.1.2-29-portable-Q16-x64";

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
                    "webm" | "webp"
                )
            })
            .map(|e| e.path().to_path_buf())
            .collect()
    }
    fn magick() -> Option<PathBuf> {
        if let Ok(path) = which("magick") {
            return Some(path);
        }
        #[cfg(windows)]
        {
            let base = std::env::current_exe().ok()?.parent()?.join(EXTRACT_DIR);
            return WalkDir::new(base)
                .into_iter()
                .filter_map(Result::ok)
                .find(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case("magick.exe")
                })
                .map(|e| e.path().to_path_buf());
        }
        #[cfg(not(windows))]
        {
            None
        }
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
        let files = Self::scan(&folder);
        if files.is_empty() {
            self.logs.push(Entry {
                text: self
                    .tr("未找到可轉換的檔案。", "No convertible files found.")
                    .into(),
                kind: "warn",
            });
            return;
        }
        let Some(magick) = Self::magick() else {
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
        let height = if self.resize {
            self.height
                .parse::<u32>()
                .ok()
                .filter(|h| (100..=2000).contains(h))
        } else {
            None
        };
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
                let mut cmd = Command::new(&magick);
                cmd.arg(&input);
                if let Some(h) = height {
                    cmd.args(["-resize", &format!("x{h}")]);
                }
                let result = cmd.arg(&output).output();
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

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            ui.heading("Webm2Gif");
            ui.label(self.tr("影片轉 GIF 工具", "Video to GIF Converter"));
            ui.add_space(14.);
            ui.label(self.tr("來源資料夾", "Source Folder"));
            ui.horizontal(|ui| {
                ui.add_enabled_ui(self.rx.is_none(), |ui| {
                    if ui.button(self.tr("選擇資料夾", "Browse")).clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            self.folder = Some(p);
                            let count = Self::scan(self.folder.as_ref().unwrap()).len();
                            self.status = if self.zh {
                                format!("找到 {count} 個檔案")
                            } else {
                                format!("{count} files found")
                            };
                        }
                    }
                });
                ui.label(
                    self.folder
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| self.tr("尚未選擇資料夾", "No folder selected").into()),
                );
            });
            ui.add_space(8.);
            ui.horizontal(|ui| {
                let resize_label = self.tr("縮放高度", "Resize height");
                ui.checkbox(&mut self.resize, resize_label);
                ui.add_enabled(
                    self.resize,
                    egui::TextEdit::singleline(&mut self.height).desired_width(65.),
                );
                ui.label("px (100–2000)");
            });
            ui.add_space(10.);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.rx.is_none(),
                        egui::Button::new(self.tr("開始轉換", "Convert")),
                    )
                    .clicked()
                {
                    self.start();
                }
                if ui.button(self.tr("清除", "Clear")).clicked() {
                    self.logs.clear();
                    self.outputs.clear();
                    self.done = 0;
                    self.status = self.tr("就緒", "Ready").into();
                }
                if ui
                    .add_enabled(
                        !self.outputs.is_empty(),
                        egui::Button::new(self.tr("開啟輸出資料夾", "Open Output Folder")),
                    )
                    .clicked()
                {
                    let path = if self.outputs.len() == 1 {
                        self.outputs.iter().next().cloned()
                    } else {
                        self.folder.clone()
                    };
                    if let Some(p) = path {
                        #[cfg(target_os = "macos")]
                        let _ = Command::new("open").arg(p).spawn();
                        #[cfg(target_os = "windows")]
                        let _ = Command::new("explorer").arg(p).spawn();
                        #[cfg(all(unix, not(target_os = "macos")))]
                        let _ = Command::new("xdg-open").arg(p).spawn();
                    }
                }
            });
            ui.add(
                egui::ProgressBar::new(if self.total == 0 {
                    0.
                } else {
                    self.done as f32 / self.total as f32
                })
                .text(format!("{}  {}/{}", self.status, self.done, self.total)),
            );
            ui.separator();
            ui.label(self.tr("轉換紀錄", "Conversion Log"));
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for e in &self.logs {
                        let color = match e.kind {
                            "ok" => egui::Color32::from_rgb(59, 109, 17),
                            "err" => egui::Color32::from_rgb(163, 45, 45),
                            _ => egui::Color32::GRAY,
                        };
                        ui.colored_label(color, &e.text);
                    }
                });
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Webm2Gif",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::<App>::default())),
    )
}
