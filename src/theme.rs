//! 字型與整體配色。

use eframe::egui;

pub fn configure_fonts(ctx: &egui::Context) {
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
