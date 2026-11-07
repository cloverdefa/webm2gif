#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod convert;
mod model;
mod platform;
mod theme;
mod tools;
mod ui;
mod updater;

use app::App;
use eframe::egui;

fn main() -> eframe::Result {
    eframe::run_native(
        "Webm2Gif",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([640., 780.])
                .with_min_inner_size([520., 640.])
                .with_drag_and_drop(true)
                .with_icon(
                    eframe::icon_data::from_png_bytes(include_bytes!("../assets/app-icon.png"))
                        .expect("invalid app icon PNG"),
                ),
            ..Default::default()
        },
        Box::new(|cc| {
            let has_cjk_font = theme::configure_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx, has_cjk_font)))
        }),
    )
}
