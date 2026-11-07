//! 字型與整體配色。

use eframe::egui;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// 支援繁體中文的字型檔名，由高到低依序優先。
const PREFERRED_FONTS: &[&str] = &[
    // Windows：微軟正黑體
    "msjh.ttc",
    "msjh.ttf",
    "msjhl.ttc",
    // Noto（Debian/Ubuntu、Arch/Omarchy、Fedora 等）
    "notosanstc-regular.otf",
    "notosanstc-regular.ttf",
    "notosanscjktc-regular.otf",
    "notosanscjk-regular.ttc",
    // macOS
    "pingfang.ttc",
    "stheiti medium.ttc",
    "stheiti light.ttc",
    "songti.ttc",
    // 其他常見 CJK 字型
    "wqy-zenhei.ttc",
    "wqy-microhei.ttc",
    "droidsansfallbackfull.ttf",
    "droidsansfallback.ttf",
    "msyh.ttc",
    "msyh.ttf",
    "simhei.ttf",
    "simsun.ttc",
    "arial unicode.ttf",
];

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = if cfg!(windows) {
        let root = std::env::var_os("WINDIR")
            .or_else(|| std::env::var_os("SystemRoot"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        vec![root.join("Fonts")]
    } else if cfg!(target_os = "macos") {
        vec![
            "/System/Library/Fonts".into(),
            "/System/Library/Fonts/Supplemental".into(),
            "/Library/Fonts".into(),
        ]
    } else {
        vec![
            "/usr/share/fonts".into(),
            "/usr/local/share/fonts".into(),
            // WSL 可以直接借用 Windows 的字型
            "/mnt/c/Windows/Fonts".into(),
        ]
    };
    if !cfg!(windows) {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            dirs.push(home.join(".local/share/fonts"));
            dirs.push(home.join(".fonts"));
            dirs.push(home.join("Library/Fonts"));
        }
    }
    dirs
}

fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
}

/// 掃描字型目錄，依 PREFERRED_FONTS 的順序挑出第一個存在的字型。
fn find_in_dirs() -> Option<PathBuf> {
    let mut found: HashMap<String, PathBuf> = HashMap::new();
    for dir in font_dirs() {
        for entry in WalkDir::new(dir)
            .max_depth(6)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if entry.file_type().is_file() && is_font_file(path) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    found
                        .entry(name.to_ascii_lowercase())
                        .or_insert_with(|| path.to_path_buf());
                }
            }
        }
    }
    PREFERRED_FONTS.iter().find_map(|name| found.remove(*name))
}

/// 向 fontconfig 詢問支援繁體中文的字型（Linux，字型不在上述清單時的後備）。
#[cfg(unix)]
fn find_with_fontconfig() -> Option<PathBuf> {
    let out = std::process::Command::new("fc-list")
        .args([":lang=zh-tw", "-f", "%{file}\n"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let files: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| PathBuf::from(l.trim()))
        .filter(|p| is_font_file(p) && p.is_file())
        .collect();
    files
        .iter()
        .find(|p| p.to_string_lossy().to_ascii_lowercase().contains("regular"))
        .or_else(|| files.first())
        .cloned()
}

#[cfg(not(unix))]
fn find_with_fontconfig() -> Option<PathBuf> {
    None
}

/// Noto Sans CJK 的 .ttc 依序為 JP、KR、SC、TC、HK，繁體中文是第 3 個（從 0 起算）。
fn collection_index(path: &Path, bytes: &[u8]) -> u32 {
    let is_noto_cjk = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| {
            let n = n.to_ascii_lowercase();
            n.starts_with("notosanscjk") && n.ends_with(".ttc")
        });
    if is_noto_cjk && bytes.len() >= 12 && &bytes[..4] == b"ttcf" {
        let count = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        if count > 3 {
            return 3;
        }
    }
    0
}

/// 載入 CJK 字型；找不到時回傳 false（介面會退回英文，避免顯示成方塊）。
pub fn configure_fonts(ctx: &egui::Context) -> bool {
    let Some(path) = find_in_dirs().or_else(find_with_fontconfig) else {
        eprintln!(
            "[webm2gif] CJK font not found; the UI falls back to English.\n  \
             Ubuntu/WSL:   sudo apt install fonts-noto-cjk\n  \
             Arch/Omarchy: sudo pacman -S noto-fonts-cjk"
        );
        return false;
    };
    let Ok(bytes) = std::fs::read(&path) else {
        return false;
    };
    let index = collection_index(&path, &bytes);
    let mut data = egui::FontData::from_owned(bytes);
    data.index = index;

    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("cjk".into(), data.into());
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "cjk".into());
    }
    ctx.set_fonts(fonts);
    true
}

pub fn apply_visuals(ctx: &egui::Context) {
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
}
