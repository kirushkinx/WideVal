#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod launcher;
mod process;
mod resolution;
mod startup;
mod ui;

use eframe::egui;
use ui::WideValApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, 400.0])
            .with_resizable(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        &format!("WideVal {}", env!("CARGO_PKG_VERSION")),
        options,
        Box::new(|cc| Ok(Box::new(WideValApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../icon.png");
    let image = image::load_from_memory(icon_bytes).expect("Failed to load icon");
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    egui::IconData {
        rgba: rgba.into_raw(),
        width,
        height,
    }
}