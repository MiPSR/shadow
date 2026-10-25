use shared::protocol::{WsClientMessage, WsServerMessage};
use std::sync::{Arc, Mutex};
use std::thread;

pub fn start(url: String, token: String, out: Arc<Mutex<Vec<shared::protocol::EncryptedMessage>>>) {
    thread::spawn(move || {
        use tungstenite::{connect, Message};
        let (mut ws, _) = match connect(url) {
            Ok(x) => x,
            Err(_) => return,
        };
        let _ = ws.send(Message::Text(serde_json::to_string(&WsClientMessage::Auth { token }).unwrap()));
        loop {
            if let Ok(msg) = ws.read() {
                if let Message::Text(t) = msg {
                    if let Ok(WsServerMessage::Message(m)) = serde_json::from_str(&t) {
                        out.lock().unwrap().push(m);
                    }
                }
            }
        }
    });
}
