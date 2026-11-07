//! 「轉換進度」卡片：進度條與活動紀錄。

use super::{ACCENT, ACCENT_DARK, HINT, card, section_label};
use crate::app::App;
use eframe::egui;

impl App {
    pub(super) fn progress_card(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, self.tr("轉換進度", "PROGRESS"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(format!("{}/{}", self.done, self.total)).strong().color(ACCENT_DARK)); });
            });
            ui.add_space(7.);
            ui.add(egui::ProgressBar::new(if self.total == 0 { 0. } else { self.done as f32 / self.total as f32 }).desired_height(10.).fill(ACCENT).text(self.status.clone()));
            ui.add_space(14.);
            ui.horizontal(|ui| {
                section_label(ui, self.tr("活動紀錄", "ACTIVITY"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label(egui::RichText::new(format!("{} {}", self.logs.len(), self.tr("筆", "items"))).size(11.).color(HINT)); });
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
    }
}
