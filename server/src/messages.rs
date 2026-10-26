use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode}, Json};
use shared::protocol::*;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::state::AppState;
use crate::channels;
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
    if req.payload.len() as u64 > state.config.max_upload_bytes { return Err(StatusCode::PAYLOAD_TOO_LARGE); }
    // channel access check
    if let Some(acct) = channels::device_account(&state.pool, &dev).await {
        if !channels::is_channel_member(&state.pool, &req.channel_id, &acct).await {
            return Err(StatusCode::FORBIDDEN);
        }
    } else { return Err(StatusCode::FORBIDDEN); }
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
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<EncryptedMessage>>, StatusCode> {
    let token = token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    if let Some(acct) = channels::device_account(&state.pool, &dev).await {
        if !channels::is_channel_member(&state.pool, &channel_id, &acct).await {
            return Err(StatusCode::FORBIDDEN);
        }
    } else { return Err(StatusCode::FORBIDDEN); }
    let limit: i64 = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100);
    let offset: i64 = params.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);
    let rows: Vec<(String,String,String,i64,String)> = sqlx::query_as(
        "SELECT id, channel_id, sender_device_id, created_at, payload FROM messages WHERE channel_id=?1 ORDER BY created_at ASC LIMIT ?2 OFFSET ?3"
    )
        .bind(&channel_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out = rows.into_iter().map(|(id, channel_id, sender_device_id, created_at, payload)| {
        EncryptedMessage { id, channel_id, sender_device_id, created_at, payload }
    }).collect();
    Ok(Json(out))
}

pub async fn delete_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token = token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    // allow sender or admin (admin via token? reuse device account? check admin accounts)
    let sender: Option<String> = sqlx::query_scalar("SELECT sender_device_id FROM messages WHERE id=?1")
        .bind(&id).fetch_optional(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.flatten();
    let Some(sender_dev) = sender else { return Err(StatusCode::NOT_FOUND); };
    if sender_dev != dev {
        // check if device's account is admin
        let acct = channels::device_account(&state.pool, &dev).await;
        let is_admin: bool = if let Some(a) = acct {
            sqlx::query_scalar("SELECT is_admin FROM accounts WHERE id=?1").bind(&a).fetch_optional(&state.pool).await.ok().flatten().unwrap_or(0) != 0
        } else { false };
        if !is_admin { return Err(StatusCode::FORBIDDEN); }
    }
    sqlx::query("DELETE FROM messages WHERE id=?1").bind(&id).execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}
