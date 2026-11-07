//! 「轉換設定」卡片：尺寸、影格率、色彩數、目標大小、片段模式。

use super::{HINT, MUTED, card, section_label};
use crate::app::App;
use eframe::egui;

impl App {
    pub(super) fn settings_card(&mut self, ui: &mut egui::Ui, narrow: bool) {
        card(ui, |ui| {
            section_label(ui, self.tr("轉換設定", "CONVERSION SETTINGS"));
            ui.add_space(8.);
            if narrow {
                ui.vertical(|ui| {
                    let resize_label = self.tr("調整輸出高度", "Resize output height");
                    ui.checkbox(&mut self.resize, resize_label);
                    ui.horizontal(|ui| {
                        ui.add_enabled(self.resize, egui::TextEdit::singleline(&mut self.height).desired_width(78.).margin(egui::Margin::symmetric(8, 7)));
                        ui.label(egui::RichText::new("px  ·  100–2000").color(MUTED));
                    });
                });
            } else {
                ui.horizontal(|ui| {
                    let resize_label = self.tr("調整輸出高度", "Resize output height");
                    ui.checkbox(&mut self.resize, resize_label);
                    ui.add_enabled(self.resize, egui::TextEdit::singleline(&mut self.height).desired_width(68.).margin(egui::Margin::symmetric(8, 7)));
                    ui.label(egui::RichText::new("px  ·  100–2000").color(MUTED));
                });
            }
            ui.label(egui::RichText::new(self.tr("關閉時保留原始尺寸，開啟時維持比例縮放。", "Leave off to keep the original size; resizing preserves the aspect ratio.")).size(11.).color(HINT));

            // ── 檔案大小控制：影格率 / 色彩數 / 目標大小 ──
            ui.add_space(8.);
            ui.separator();
            ui.add_space(4.);
            ui.horizontal_wrapped(|ui| {
                ui.label(self.tr("影格率", "Frame rate"));
                ui.add(egui::Slider::new(&mut self.fps, 5..=30).suffix(" fps"));
                ui.add_space(10.);
                ui.label(self.tr("色彩數", "Colors"));
                egui::ComboBox::from_id_salt("colors")
                    .selected_text(self.colors.to_string())
                    .show_ui(ui, |ui| {
                        for c in [256u32, 128, 64, 32] {
                            ui.selectable_value(&mut self.colors, c, c.to_string());
                        }
                    });
            });
            let target_label = self.tr("限制輸出檔案大小", "Limit output file size");
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.target_on, target_label);
                ui.add_enabled(self.target_on, egui::TextEdit::singleline(&mut self.target_mb).desired_width(68.).margin(egui::Margin::symmetric(8, 7)));
                ui.label(egui::RichText::new("MB").color(MUTED));
            });
            ui.label(egui::RichText::new(self.tr("降低影格率、縮小尺寸或減少色彩數都能縮小檔案；啟用大小限制後會自動逐步壓縮到低於目標（較耗時）。", "Lower frame rate, smaller size, or fewer colors all shrink the file. With a size limit, files are re-encoded with progressively lower settings until under the target (slower).")).size(11.).color(HINT));

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
                ui.label(egui::RichText::new(self.tr("片段模式支援 WebM、MP4 與 MOV，並需要 FFmpeg。", "Clip mode supports WebM, MP4, and MOV and requires FFmpeg.")).size(11.).color(HINT));
            }
        });
    }
}
