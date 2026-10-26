use axum::{
    extract::{State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use shared::protocol::{WsClientMessage, WsServerMessage, EncryptedMessage, KeyAnnouncement};
use chrono;
use uuid::Uuid;
use sqlx;

use crate::state::AppState;
use crate::auth::device_from_token;

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut device_id: Option<String> = None;
    let mut rx = state.broadcast.subscribe();

    // authentication first
    // send loop
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(cm) = serde_json::from_str::<WsClientMessage>(&text) {
                    match cm {
                        WsClientMessage::Auth { token } => {
                            if let Some(dev) = device_from_token(&state.pool, &token).await {
                                let _ = state.broadcast.send(serde_json::to_string(&WsServerMessage::Presence { device_id: dev.clone(), online: true }).unwrap());
                                device_id = Some(dev);
                            } else {
                                let _ = state.broadcast.send(serde_json::to_string(&WsServerMessage::Error { message: "auth failed".into() }).unwrap());
                            }
                        }
                        WsClientMessage::SendMessage { channel_id, payload } => {
                            if let Some(dev) = &device_id {
                                let id = Uuid::new_v4().to_string();
                                let created = chrono::Utc::now().timestamp();
                                let _ = sqlx::query("INSERT INTO messages (id, channel_id, sender_device_id, created_at, payload) VALUES (?1,?2,?3,?4,?5)")
                                    .bind(&id).bind(&channel_id).bind(dev).bind(created).bind(&payload)
                                    .execute(&state.pool).await;
                                let em = EncryptedMessage {
                                    id, channel_id, sender_device_id: dev.clone(), created_at: created, payload,
                                };
                                let _ = state.broadcast.send(serde_json::to_string(&WsServerMessage::Message(em)).unwrap());
                            }
                        }
                        WsClientMessage::KeyAnnounce { announcement } => {
                            if let Some(dev) = &device_id {
                                if dev == &announcement.from_device {
                                    let _ = sqlx::query("INSERT INTO key_announcements (from_device, to_device, channel_id, kem_ciphertext, nonce, wrapped_key, signature, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)")
                                        .bind(&announcement.from_device).bind(&announcement.to_device).bind(&announcement.channel_id)
                                        .bind(&announcement.kem_ciphertext).bind(&announcement.nonce).bind(&announcement.wrapped_key).bind(&announcement.signature).bind(chrono::Utc::now().timestamp())
                                        .execute(&state.pool).await;
                                    let _ = state.broadcast.send(serde_json::to_string(&WsServerMessage::KeyAnnounce(announcement)).unwrap());
                                }
                            }
                        }
                        WsClientMessage::Ping => {
                            let _ = state.broadcast.send(serde_json::to_string(&WsServerMessage::Pong).unwrap());
                        }
                        WsClientMessage::Typing { channel_id } => {
                            if let Some(dev) = &device_id {
                                let _ = state.broadcast.send(serde_json::to_string(&WsServerMessage::Typing { channel_id, device_id: dev.clone() }).unwrap());
                            }
                        }
                        WsClientMessage::Presence => {
                            if let Some(dev) = &device_id {
                                let _ = state.broadcast.send(serde_json::to_string(&WsServerMessage::Presence { device_id: dev.clone(), online: true }).unwrap());
                            }
                        }
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    if let Some(dev) = &device_id {
        let _ = state.broadcast.send(serde_json::to_string(&WsServerMessage::Presence { device_id: dev.clone(), online: false }).unwrap());
    }
    send_task.abort();
}
