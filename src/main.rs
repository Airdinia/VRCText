#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod download;
mod osc;
mod theme;
mod tts;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([460.0, 340.0])
            .with_min_inner_size([460.0, 240.0])
            .with_title("VRCText")
            .with_app_id("vrctext"),
        ..Default::default()
    };
    eframe::run_native(
        "VRCText",
        options,
        Box::new(|cc| Ok(Box::new(app::VRCTextApp::new(cc)))),
    )
}
