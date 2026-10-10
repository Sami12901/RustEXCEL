use eframe::egui;
use rust_exel::RustExelApp;

fn load_app_icon() -> Option<egui::IconData> {
    let png_bytes = include_bytes!("../../../assets/icons/logo_64.png");
    if let Ok(img) = image::load_from_memory(png_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let raw = rgba.into_raw();
        // Ensure image dimensions are non-zero and exactly match the buffer size
        if width > 0
            && height > 0
            && (width as usize)
                .saturating_mul(height as usize)
                .saturating_mul(4)
                == raw.len()
        {
            Some(egui::IconData {
                rgba: raw,
                width,
                height,
            })
        } else {
            None
        }
    } else {
        None
    }
}

fn create_viewport() -> egui::ViewportBuilder {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Rust EXEL - Professional Spreadsheet")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([640.0, 480.0]);

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    viewport
}

fn run_with_renderer(renderer: eframe::Renderer) -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        renderer,
        viewport: create_viewport(),
        ..Default::default()
    };

    eframe::run_native(
        "Rust EXEL",
        native_options,
        Box::new(|cc| Ok(Box::new(RustExelApp::new(cc)))),
    )
}

fn main() -> Result<(), eframe::Error> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Rust EXEL native desktop application...");

    // 1. Attempt primary WGPU rendering backend
    tracing::info!("Attempting startup with WGPU hardware-accelerated renderer...");
    let wgpu_result = run_with_renderer(eframe::Renderer::Wgpu);

    match wgpu_result {
        Ok(()) => {
            tracing::info!("Rust EXEL exited cleanly (WGPU renderer).");
            Ok(())
        }
        Err(wgpu_err) => {
            tracing::warn!(
                "WGPU renderer initialization failed ({wgpu_err}). Falling back to Glow (OpenGL) renderer..."
            );

            // 2. Fall back to Glow renderer so app never hard-crashes on systems without WGPU adapters
            let glow_result = run_with_renderer(eframe::Renderer::Glow);

            match glow_result {
                Ok(()) => {
                    tracing::info!("Rust EXEL exited cleanly (Glow fallback renderer).");
                    Ok(())
                }
                Err(glow_err) => {
                    let crash_message = format!(
                        "Rust EXEL Fatal Startup Crash Report\n\
                         Timestamp: {:?}\n\
                         Primary WGPU Error: {}\n\
                         Fallback Glow Error: {}\n\
                         Environment: OS Windows\n",
                        std::time::SystemTime::now(),
                        wgpu_err,
                        glow_err
                    );
                    tracing::error!("{crash_message}");

                    // Log crash to crash.log file
                    if let Err(write_err) = std::fs::write("crash.log", &crash_message) {
                        tracing::error!("Failed to write crash.log: {write_err}");
                    }

                    Err(glow_err)
                }
            }
        }
    }
}
