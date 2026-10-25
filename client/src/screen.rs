use scrap::{Capturer, Display};
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct ScreenSession {
    pub running: Arc<Mutex<bool>>,
    pub handle: Option<thread::JoinHandle<()>>,
}

impl ScreenSession {
    pub fn start(peer: std::net::SocketAddr, key: [u8; 32]) -> Result<Self, String> {
        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();
        let handle = thread::spawn(move || {
            if let Err(e) = run_screen(peer, key, running_clone.clone()) {
                eprintln!("screen error: {}", e);
            }
            *running_clone.lock().unwrap() = false;
        });
        Ok(Self { running, handle: Some(handle) })
    }

    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
        if let Some(h) = self.handle.take() { let _ = h.join(); }
    }
}

fn run_screen(peer: std::net::SocketAddr, key: [u8; 32], running: Arc<Mutex<bool>>) -> Result<(), String> {
    let display = Display::primary().map_err(|e| e.to_string())?;
    let w = display.width();
    let h = display.height();
    let mut capturer = Capturer::new(display).map_err(|e| e.to_string())?;
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    // send a header packet with dimensions
    let header = format!("SCREEN:{}:{}", w, h);
    let _ = socket.send_to(header.as_bytes(), peer);

    while *running.lock().unwrap() {
        if let Ok(frame) = capturer.frame() {
            // frame is BGRA; encrypt and send in chunks? Send whole (may be large)
            if let Ok((nonce, ct)) = shared::crypto::xchacha::encrypt(&key, &*frame, &[]) {
                let mut pkt = Vec::with_capacity(24 + ct.len());
                pkt.extend_from_slice(&nonce);
                pkt.extend_from_slice(&ct);
                // UDP may fail for large; ignore
                let _ = socket.send_to(&pkt, peer);
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}
