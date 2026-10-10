mod app;
mod calc;
mod commands;

use app::RustExelApp;
use eframe::egui;

fn load_app_icon() -> Option<egui::IconData> {
    let png_bytes = include_bytes!("../../../assets/icons/logo_64.png");
    if let Ok(img) = image::load_from_memory(png_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        })
    } else {
        None
    }
}

fn main() -> Result<(), eframe::Error> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Rust EXEL native desktop application with wgpu renderer...");

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Rust EXEL - Professional Spreadsheet")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([640.0, 480.0]);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        // Explicitly use the wgpu rendering backend as requested
        renderer: eframe::Renderer::Wgpu,
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Rust EXEL",
        native_options,
        Box::new(|cc| Ok(Box::new(RustExelApp::new(cc)))),
    )
}
