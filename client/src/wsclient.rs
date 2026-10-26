use shared::protocol::{WsClientMessage, WsServerMessage};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn start(url: String, token: String, out: Arc<Mutex<Vec<shared::protocol::EncryptedMessage>>>, typing: Arc<Mutex<Vec<(String, String)>>>, presence: Arc<Mutex<HashSet<String>>>) {
    thread::spawn(move || {
        loop {
            use tungstenite::{connect, Message};
            if let Ok((mut ws, _)) = connect(&url) {
                let _ = ws.send(Message::Text(serde_json::to_string(&WsClientMessage::Auth { token: token.clone() }).unwrap()));
                let _ = ws.send(Message::Text(serde_json::to_string(&WsClientMessage::Presence).unwrap()));
                loop {
                    match ws.read() {
                        Ok(Message::Text(t)) => {
                            if let Ok(WsServerMessage::Message(m)) = serde_json::from_str(&t) {
                                out.lock().unwrap().push(m);
                            } else if let Ok(WsServerMessage::Typing { channel_id, device_id }) = serde_json::from_str(&t) {
                                let mut v = typing.lock().unwrap();
                                if !v.iter().any(|(c,d)| c==&channel_id && d==&device_id) {
                                    v.push((channel_id, device_id));
                                }
                            } else if let Ok(WsServerMessage::Presence { device_id, online }) = serde_json::from_str(&t) {
                                let mut p = presence.lock().unwrap();
                                if online { p.insert(device_id); } else { p.remove(&device_id); }
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
