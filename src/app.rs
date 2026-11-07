//! 應用程式狀態與商業邏輯（與繪圖無關的部分）。

use crate::{
    convert::{self, BatchParams, is_supported, is_video_ext},
    model::{Entry, Event},
    tools,
    updater::Updater,
};
use eframe::egui;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
};
use walkdir::WalkDir;

pub struct App {
    pub zh: bool,
    pub files: Vec<PathBuf>,
    /// 最近一次是以「單一資料夾」選擇來源時記錄，僅用於顯示路徑
    pub source_folder: Option<PathBuf>,
    pub resize: bool,
    pub height: String,
    pub fps: u32,
    pub colors: u32,
    pub target_on: bool,
    pub target_mb: String,
    pub clip_mode: bool,
    pub clip_start: String,
    pub clip_end: String,
    pub status: String,
    pub logs: Vec<Entry>,
    pub outputs: BTreeSet<PathBuf>,
    pub rx: Option<Receiver<Event>>,
    pub total: usize,
    pub done: usize,
    pub updater: Updater,
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
            updater: Updater::new(),
        }
    }
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        let mut app = Self::default();
        // 啟動時靜默檢查一次；失敗或已是最新都不打擾使用者。
        app.updater.check(ctx, true);
        app
    }

    pub fn tr(&self, zh: &'static str, en: &'static str) -> &'static str {
        if self.zh { zh } else { en }
    }

    fn push(&mut self, kind: &'static str, zh: &'static str, en: &'static str) {
        let text = self.tr(zh, en).into();
        self.logs.push(Entry { text, kind });
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
    pub fn add_paths(&mut self, paths: Vec<PathBuf>, replace: bool) {
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

    pub fn source_text(&self) -> String {
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

    /// 拖曳檔案 / 資料夾到視窗（轉換中不處理）
    pub fn handle_drop(&mut self, ctx: &egui::Context) {
        if self.rx.is_some() {
            return;
        }
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

    /// 收取背景轉換執行緒回報的事件。
    pub fn poll_convert(&mut self, ctx: &egui::Context) {
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
    }

    pub fn start(&mut self) {
        if self.files.is_empty() {
            self.push(
                "warn",
                "請先選擇來源檔案或資料夾。",
                "Please select source files or a folder first.",
            );
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
            if self.clip_mode {
                self.push(
                    "warn",
                    "未找到 WebM、MP4 或 MOV 影片。",
                    "No WebM, MP4, or MOV videos found.",
                );
            } else {
                self.push("warn", "未找到可轉換的檔案。", "No convertible files found.");
            }
            return;
        }
        let clip_range = if self.clip_mode {
            match (self.clip_start.parse::<f64>(), self.clip_end.parse::<f64>()) {
                (Ok(start), Ok(end)) if start >= 0.0 && end > start => Some((start, end)),
                _ => {
                    self.push(
                        "warn",
                        "片段時間無效：結束時間需大於起始時間，且不可為負數。",
                        "Invalid clip range: end time must be greater than start time, and neither value can be negative.",
                    );
                    return;
                }
            }
        } else {
            None
        };
        let needs_magick = clip_range.is_none() && files.iter().any(|path| !is_video_ext(path));
        let magick = if needs_magick {
            let Some(path) = tools::magick() else {
                self.push(
                    "err",
                    "找不到 ImageMagick，請先安裝並加入 PATH。",
                    "ImageMagick was not found. Install it and add it to PATH.",
                );
                return;
            };
            Some(path)
        } else {
            None
        };
        let needs_ffmpeg = clip_range.is_some() || files.iter().any(|path| is_video_ext(path));
        let ffmpeg = if needs_ffmpeg {
            let Some(path) = tools::ffmpeg() else {
                self.push(
                    "err",
                    "MP4、MOV 與影片片段轉換需要 FFmpeg，請先安裝並加入 PATH。",
                    "MP4, MOV, and clip conversion require FFmpeg. Install it and add it to PATH.",
                );
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
            self.push(
                "warn",
                "高度需為 100 到 2000 之間的整數。",
                "Height must be a whole number from 100 to 2000.",
            );
            return;
        }
        let target_bytes = if self.target_on {
            match self.target_mb.trim().parse::<f64>() {
                Ok(mb) if mb >= 0.1 => Some((mb * 1_048_576.0) as u64),
                _ => {
                    self.push(
                        "warn",
                        "目標大小需為不小於 0.1 的數字（MB）。",
                        "Target size must be a number of at least 0.1 (MB).",
                    );
                    return;
                }
            }
        } else {
            None
        };
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.outputs.clear();
        self.total = files.len();
        self.done = 0;
        self.status = self.tr("轉換中…", "Converting…").into();
        convert::spawn_batch(
            BatchParams {
                files,
                height,
                fps: self.fps,
                colors: self.colors,
                target_bytes,
                clip_range,
                ffmpeg,
                magick,
                zh: self.zh,
            },
            tx,
        );
    }
}
