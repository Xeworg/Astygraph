//! Astynex desktop entry point.

use eframe::run_native;

fn main() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(astynex::APPLICATION_NAME)
            .with_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    run_native(
        astynex::APPLICATION_NAME,
        options,
        Box::new(|_cc| Ok(Box::new(astynex::app::AstynexApp::default()))),
    )
    .expect("eframe native run failed");
}
