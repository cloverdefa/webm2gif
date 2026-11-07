//! 「來源」卡片：選擇檔案／資料夾。

use super::{MUTED, card, section_label};
use crate::app::App;
use eframe::egui;

impl App {
    pub(super) fn source_card(&mut self, ui: &mut egui::Ui, narrow: bool) {
        card(ui, |ui| {
            section_label(ui, self.tr("來源", "SOURCE"));
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
                    ui.label(egui::RichText::new(if chosen { source_ready } else { source_hint }).size(12.).color(MUTED));
                    ui.add(egui::Label::new(egui::RichText::new(folder_text).size(14.).strong().color(egui::Color32::from_rgb(38, 49, 63))).wrap());
                    ui.add_space(8.);
                    picker(ui, true);
                });
            } else {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(if chosen { source_ready } else { source_hint }).size(12.).color(MUTED));
                        ui.add_space(3.);
                        ui.add(egui::Label::new(egui::RichText::new(folder_text).size(14.).strong().color(egui::Color32::from_rgb(38, 49, 63))).truncate());
                    });
                    ui.add_space(6.);
                    picker(ui, false);
                });
            }
        });
    }
}
