use shared::protocol::{WsClientMessage, WsServerMessage};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn start(url: String, token: String, out: Arc<Mutex<Vec<shared::protocol::EncryptedMessage>>>) {
    thread::spawn(move || {
        loop {
            use tungstenite::{connect, Message};
            if let Ok((mut ws, _)) = connect(&url) {
                let _ = ws.send(Message::Text(serde_json::to_string(&WsClientMessage::Auth { token: token.clone() }).unwrap()));
                loop {
                    match ws.read() {
                        Ok(Message::Text(t)) => {
                            if let Ok(WsServerMessage::Message(m)) = serde_json::from_str(&t) {
                                out.lock().unwrap().push(m);
                            }
                        }
                        Ok(Message::Close(_)) => break,
                        Err(_) => break,
                        _ => {}
                    }
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    });
}
