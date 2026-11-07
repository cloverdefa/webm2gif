//! 介面：eframe::App 入口與各區塊共用的樣式。

mod actions;
mod header;
mod progress;
mod settings;
mod source;
mod update;

use crate::{app::App, theme};
use eframe::egui;

pub const MUTED: egui::Color32 = egui::Color32::from_rgb(105, 117, 132);
pub const HINT: egui::Color32 = egui::Color32::from_rgb(125, 135, 147);
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0, 137, 105);
pub const ACCENT_DARK: egui::Color32 = egui::Color32::from_rgb(0, 105, 82);
pub const ACCENT_SOFT: egui::Color32 = egui::Color32::from_rgb(225, 245, 239);
const BORDER: egui::Color32 = egui::Color32::from_rgb(229, 233, 238);

/// 白底圓角卡片。
pub fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::group(ui.style())
        .fill(egui::Color32::WHITE)
        .stroke(egui::Stroke::new(1.0_f32, BORDER))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(16))
        .show(ui, add_contents)
}

/// 卡片內的小標題（全大寫灰字）。
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(11.).strong().color(MUTED));
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        theme::apply_visuals(ctx);
        self.handle_drop(ctx);
        self.poll_convert(ctx);
        self.updater.poll();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let narrow = ui.available_width() < 570.0;
                ui.add_space(6.);
                self.header(ui, narrow);
                ui.add_space(18.);
                self.update_banner(ui);
                self.source_card(ui, narrow);
                ui.add_space(12.);
                self.settings_card(ui, narrow);
                ui.add_space(14.);
                self.action_row(ui, narrow);
                ui.add_space(12.);
                self.progress_card(ui);
                ui.add_space(8.);
            });
        });
    }
}
