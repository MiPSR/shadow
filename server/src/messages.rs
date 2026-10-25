use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode}, Json};
use shared::protocol::*;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::state::AppState;
use crate::auth::device_from_token;

fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn send_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<EncryptedMessage>, StatusCode> {
    let token = token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let id = Uuid::new_v4().to_string();
    let created = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO messages (id, channel_id, sender_device_id, created_at, payload) VALUES (?1,?2,?3,?4,?5)")
        .bind(&id)
        .bind(&req.channel_id)
        .bind(&dev)
        .bind(created)
        .bind(&req.payload)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(EncryptedMessage {
        id,
        channel_id: req.channel_id,
        sender_device_id: dev,
        created_at: created,
        payload: req.payload,
    }))
}

pub async fn list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<EncryptedMessage>>, StatusCode> {
    let token = token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let rows: Vec<(String,String,String,i64,String)> = sqlx::query_as(
        "SELECT id, channel_id, sender_device_id, created_at, payload FROM messages WHERE channel_id=?1 ORDER BY created_at ASC"
    )
        .bind(&channel_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out = rows.into_iter().map(|(id, channel_id, sender_device_id, created_at, payload)| {
        EncryptedMessage { id, channel_id, sender_device_id, created_at, payload }
    }).collect();
    Ok(Json(out))
}
