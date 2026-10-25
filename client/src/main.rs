#[cfg(not(target_arch = "wasm32"))]
mod api;
#[cfg(not(target_arch = "wasm32"))]
mod app;
#[cfg(not(target_arch = "wasm32"))]
mod backup;
#[cfg(not(target_arch = "wasm32"))]
mod compat;
#[cfg(not(target_arch = "wasm32"))]
mod config;
#[cfg(not(target_arch = "wasm32"))]
mod media;
#[cfg(not(target_arch = "wasm32"))]
mod screen;
#[cfg(not(target_arch = "wasm32"))]
mod storage;
#[cfg(not(target_arch = "wasm32"))]
mod p2p;
#[cfg(not(target_arch = "wasm32"))]
mod voice;
#[cfg(not(target_arch = "wasm32"))]
mod wsclient;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "shadow",
        options,
        Box::new(|_cc| Ok(Box::new(app::ShadowApp::new()))),
    )
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
    let window = web_sys::window().ok_or_else(|| wasm_bindgen::JsValue::from_str("no window"))?;
    let document = window.document().ok_or_else(|| wasm_bindgen::JsValue::from_str("no document"))?;
    let canvas = document.get_element_by_id(canvas_id).ok_or_else(|| wasm_bindgen::JsValue::from_str("canvas not found"))?;
    if canvas.dyn_ref::<web_sys::HtmlCanvasElement>().is_none() {
        return Err(wasm_bindgen::JsValue::from_str("invalid canvas"));
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn main() {}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct WebApp {}
#[cfg(target_arch = "wasm32")]
impl eframe::App for WebApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("shadow (web client not fully implemented)");
        });
    }
}
