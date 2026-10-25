mod api;
mod app;
mod config;
mod storage;

use app::ShadowApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "shadow",
        options,
        Box::new(|_cc| Ok(Box::new(ShadowApp::new()))),
    )
}
