#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::ViewportBuilder;
use quark_sextant::ui::view::launcher::launcher_app::LauncherApp;

/// The launcher: your projects and themes. Opening a project starts the viewer in its own window.
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Quark-Sextant Launcher")
            .with_inner_size([980.0, 600.0])
            .with_min_inner_size([760.0, 460.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native("Quark-Sextant Launcher", options, Box::new(|cc| Ok(Box::new(LauncherApp::new(cc)))))
}
