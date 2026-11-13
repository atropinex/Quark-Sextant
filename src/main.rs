#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::ViewportBuilder;
use quark_sextant::ui::view::viewer::viewer_app::ViewerApp;
use std::path::PathBuf;

/// The viewer. The launcher starts it as `quark-sextant --project <folder>`, or run it alone and pick a folder.
fn main() -> eframe::Result<()> {
    let project = std::env::args().skip_while(|arg| arg != "--project").nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title("Quark-Sextant")
            .with_inner_size([1360.0, 820.0])
            .with_min_inner_size([900.0, 560.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native("Quark-Sextant", options, Box::new(move |cc| Ok(Box::new(ViewerApp::new(cc, project)))))
}
