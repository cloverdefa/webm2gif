//! 更新橫幅：有新版／下載中／已安裝／失敗時顯示在標題列下方。

use super::{ACCENT, ACCENT_DARK, HINT, MUTED, card, section_label};
use crate::{
    app::App,
    platform,
    updater::{CURRENT_VERSION, RELEASES_PAGE, State},
};
use eframe::egui;

enum Act {
    Install,
    Dismiss,
    Retry,
    Restart,
    OpenPage(String),
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

impl App {
    pub(super) fn update_banner(&mut self, ui: &mut egui::Ui) {
        let state = match &self.updater.state {
            State::Idle | State::Checking | State::UpToDate => return,
            s => s.clone(),
        };
        let converting = self.rx.is_some();
        let mut act: Option<Act> = None;

        card(ui, |ui| {
            section_label(ui, self.tr("軟體更新", "UPDATE"));
            ui.add_space(6.);
            match &state {
                State::Available(info) => {
                    let title = if self.zh {
                        format!("發現新版本 v{}（目前 v{CURRENT_VERSION}）", info.version)
                    } else {
                        format!("Version v{} is available (current v{CURRENT_VERSION})", info.version)
                    };
                    ui.label(egui::RichText::new(title).size(15.).strong().color(ACCENT_DARK));
                    if !info.notes.trim().is_empty() {
                        ui.add_space(4.);
                        egui::CollapsingHeader::new(self.tr("更新內容", "Release notes"))
                            .id_salt("update_notes")
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical().max_height(120.).show(ui, |ui| {
                                    ui.add(egui::Label::new(egui::RichText::new(info.notes.trim()).size(12.).color(MUTED)).wrap());
                                });
                            });
                    }
                    ui.add_space(8.);
                    ui.horizontal_wrapped(|ui| {
                        if info.asset.is_some() {
                            let update = egui::Button::new(egui::RichText::new(self.tr("立即更新", "Update now")).strong().color(egui::Color32::WHITE))
                                .fill(ACCENT)
                                .min_size(egui::vec2(130., 36.))
                                .corner_radius(egui::CornerRadius::same(8));
                            if ui.add_enabled(!converting, update).clicked() {
                                act = Some(Act::Install);
                            }
                        }
                        if ui.button(self.tr("前往下載頁", "Open release page")).clicked() {
                            act = Some(Act::OpenPage(info.page.clone()));
                        }
                        if ui.button(self.tr("稍後", "Later")).clicked() {
                            act = Some(Act::Dismiss);
                        }
                    });
                    let hint = if info.asset.is_none() {
                        Some(self.tr("此平台沒有可自動安裝的檔案，請至下載頁手動更新。", "No automatic installer for this platform; please update from the release page."))
                    } else if converting {
                        Some(self.tr("轉換進行中，完成後才能更新。", "A conversion is running; update after it finishes."))
                    } else {
                        None
                    };
                    if let Some(hint) = hint {
                        ui.add_space(4.);
                        ui.label(egui::RichText::new(hint).size(11.).color(HINT));
                    }
                }
                State::Installing { done, total } => {
                    let (fraction, text) = if *total > 0 {
                        let fraction = (*done as f32 / *total as f32).clamp(0., 1.);
                        let text = if done >= total {
                            self.tr("安裝中…", "Installing…").to_string()
                        } else {
                            format!("{} {:.1} / {:.1} MB", self.tr("下載中", "Downloading"), mb(*done), mb(*total))
                        };
                        (fraction, text)
                    } else {
                        (0., self.tr("下載中…", "Downloading…").to_string())
                    };
                    ui.add(egui::ProgressBar::new(fraction).desired_height(10.).fill(ACCENT).text(text));
                    ui.add_space(4.);
                    ui.label(egui::RichText::new(self.tr("更新期間請勿關閉程式。", "Please keep the app open during the update.")).size(11.).color(HINT));
                }
                State::Installed(version) => {
                    let text = if self.zh {
                        format!("已更新至 v{version}，重新啟動後生效。")
                    } else {
                        format!("Updated to v{version}. Restart to use the new version.")
                    };
                    ui.label(egui::RichText::new(text).size(15.).strong().color(ACCENT_DARK));
                    ui.add_space(8.);
                    ui.horizontal_wrapped(|ui| {
                        let restart = egui::Button::new(egui::RichText::new(self.tr("重新啟動", "Restart now")).strong().color(egui::Color32::WHITE))
                            .fill(ACCENT)
                            .min_size(egui::vec2(130., 36.))
                            .corner_radius(egui::CornerRadius::same(8));
                        if ui.add_enabled(!converting, restart).clicked() {
                            act = Some(Act::Restart);
                        }
                        if ui.button(self.tr("稍後", "Later")).clicked() {
                            act = Some(Act::Dismiss);
                        }
                    });
                }
                State::Failed(message) => {
                    let text = if self.zh {
                        format!("更新失敗：{message}")
                    } else {
                        format!("Update failed: {message}")
                    };
                    ui.add(egui::Label::new(egui::RichText::new(text).color(egui::Color32::from_rgb(180, 54, 54))).wrap());
                    ui.add_space(8.);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(self.tr("重新檢查", "Check again")).clicked() {
                            act = Some(Act::Retry);
                        }
                        if ui.button(self.tr("前往下載頁", "Open release page")).clicked() {
                            act = Some(Act::OpenPage(RELEASES_PAGE.to_string()));
                        }
                        if ui.button(self.tr("關閉", "Dismiss")).clicked() {
                            act = Some(Act::Dismiss);
                        }
                    });
                }
                State::Idle | State::Checking | State::UpToDate => {}
            }
        });
        ui.add_space(12.);

        let ctx = ui.ctx().clone();
        match act {
            Some(Act::Install) => self.updater.install(&ctx),
            Some(Act::Dismiss) => self.updater.dismiss(),
            Some(Act::Retry) => {
                self.updater.dismiss();
                self.updater.check(&ctx, false);
            }
            Some(Act::Restart) => {
                if self.updater.restart() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Some(Act::OpenPage(url)) => platform::open(url),
            None => {}
        }
    }
}
