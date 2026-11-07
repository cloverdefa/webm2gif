//! 操作按鈕列：開始轉換、開啟輸出、清除紀錄。

use crate::{app::App, platform};
use eframe::egui;

impl App {
    pub(super) fn action_row(&mut self, ui: &mut egui::Ui, narrow: bool) {
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
                    platform::open(p);
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
    }
}
