//! 標題列：App 名稱、版本／檢查更新按鈕、語言切換。

use super::{ACCENT_DARK, ACCENT_SOFT, MUTED};
use crate::{
    app::App,
    updater::{CURRENT_VERSION, State},
};
use eframe::egui;

impl App {
    pub(super) fn header(&mut self, ui: &mut egui::Ui, narrow: bool) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(egui::RichText::new("Webm2Gif").size(27.).strong().color(egui::Color32::from_rgb(30, 41, 55)));
                ui.label(egui::RichText::new(self.tr("影片轉 GIF 工具", "Video to GIF Converter")).color(MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 語言切換
                let lang_label = if self.zh { "EN" } else if self.cjk_font { "中文" } else { "ZH" };
                let lang_button = egui::Button::new(lang_label).small();
                if ui
                    .add_enabled(self.zh || self.cjk_font, lang_button)
                    .on_disabled_hover_text("No CJK font found. Install fonts-noto-cjk to enable Chinese.")
                    .clicked()
                {
                    self.zh = !self.zh;
                }
                self.version_button(ui);
                if !narrow {
                    egui::Frame::new().fill(ACCENT_SOFT).corner_radius(egui::CornerRadius::same(16)).inner_margin(egui::Margin::symmetric(11, 6)).show(ui, |ui| {
                        ui.label(egui::RichText::new("WEBM / WEBP / MP4 / MOV  →  GIF").size(10.).strong().color(ACCENT_DARK));
                    });
                }
            });
        });
    }

    /// 顯示目前版本；點擊可手動檢查 GitHub 上是否有新版。
    fn version_button(&mut self, ui: &mut egui::Ui) {
        let (label, enabled, highlight) = match &self.updater.state {
            State::Checking => (self.tr("檢查中…", "Checking…").to_string(), false, false),
            State::Installing { .. } | State::Installed(_) => (format!("v{CURRENT_VERSION}"), false, false),
            State::UpToDate => (format!("v{CURRENT_VERSION}  ✓ {}", self.tr("已是最新", "Up to date")), true, false),
            State::Available(info) => (format!("⬆ v{}  {}", info.version, self.tr("可更新", "available")), true, true),
            State::Idle | State::Failed(_) => (format!("v{CURRENT_VERSION}  ⟳ {}", self.tr("檢查更新", "Check for updates")), true, false),
        };
        let mut text = egui::RichText::new(label).size(12.);
        if highlight {
            text = text.strong().color(ACCENT_DARK);
        }
        let mut button = egui::Button::new(text).small();
        if highlight {
            button = button.fill(ACCENT_SOFT);
        }
        let can_check = matches!(self.updater.state, State::Idle | State::UpToDate | State::Failed(_));
        if ui.add_enabled(enabled, button).clicked() && can_check {
            let ctx = ui.ctx().clone();
            self.updater.check(&ctx, false);
        }
    }
}
