// RustView — Lightweight multi-format image viewer
// Entry point: parse CLI args, launch eframe window

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console on Windows release

mod app;
mod loader;
mod nav;
mod types;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    // Accept a file path as first CLI argument (file association)
    let initial_path: Option<PathBuf> = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|p| p.exists());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("RustView")
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "RustView",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::RustViewApp::new(cc, initial_path)))),
    )
}
