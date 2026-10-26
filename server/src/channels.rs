use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use shared::protocol::*;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::state::AppState;
use crate::auth::device_from_token;

fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn create_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<Channel>, StatusCode> {
    let token = token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let id = Uuid::new_v4().to_string();
    let members = serde_json::to_string(&req.members).unwrap();
    sqlx::query("INSERT INTO channels (id, name, kind, members) VALUES (?1,?2,?3,?4)")
        .bind(&id)
        .bind(&req.name)
        .bind(format!("{:?}", req.kind))
        .bind(&members)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Channel { id, name: req.name, kind: req.kind, members: req.members }))
}

pub async fn list_channels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Channel>>, StatusCode> {
    let token = token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let rows: Vec<(String,String,String,String)> = sqlx::query_as("SELECT id,name,kind,members FROM channels")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::new();
    for (id,name,kind,members) in rows {
        let members: Vec<Id> = serde_json::from_str(&members).unwrap_or_default();
        let kind = match kind.as_str() {
            "Direct" => ChannelKind::Direct,
            "Group" => ChannelKind::Group,
            "Text" => ChannelKind::Text,
            "Voice" => ChannelKind::Voice,
            _ => ChannelKind::Text,
        };
        out.push(Channel { id, name, kind, members });
    }
    Ok(Json(out))
}

pub async fn add_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(channel_id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<Channel>, StatusCode> {
    let token = headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = crate::auth::device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let account_id = body.get("account_id").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?.to_string();
    // load channel
    let row: Option<(String,String,String,String)> = sqlx::query_as("SELECT id,name,kind,members FROM channels WHERE id=?1")
        .bind(&channel_id).fetch_optional(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some((id,name,kind,members)) = row else { return Err(StatusCode::NOT_FOUND); };
    let mut mems: Vec<String> = serde_json::from_str(&members).unwrap_or_default();
    if !mems.contains(&account_id) { mems.push(account_id); }
    let mems_json = serde_json::to_string(&mems).unwrap();
    sqlx::query("UPDATE channels SET members=?1 WHERE id=?2").bind(&mems_json).bind(&channel_id).execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let kind = match kind.as_str() {
        "Direct" => shared::protocol::ChannelKind::Direct,
        "Group" => shared::protocol::ChannelKind::Group,
        "Text" => shared::protocol::ChannelKind::Text,
        "Voice" => shared::protocol::ChannelKind::Voice,
        _ => shared::protocol::ChannelKind::Text,
    };
    Ok(Json(shared::protocol::Channel { id, name, kind, members: mems }))
}

pub async fn device_account(pool: &sqlx::SqlitePool, device_id: &str) -> Option<String> {
    sqlx::query_scalar("SELECT account_id FROM devices WHERE id=?1")
        .bind(device_id).fetch_optional(pool).await.ok().flatten()
}

pub async fn is_channel_member(pool: &sqlx::SqlitePool, channel_id: &str, account_id: &str) -> bool {
    let row: Option<(String,)> = sqlx::query_as("SELECT members FROM channels WHERE id=?1")
        .bind(channel_id).fetch_optional(pool).await.ok().flatten();
    if let Some((members,)) = row {
        if let Ok(mems) = serde_json::from_str::<Vec<String>>(&members) {
            return mems.contains(&account_id.to_string());
        }
    }
    false
}

pub async fn delete_channel(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(channel_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token = headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = crate::auth::device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    sqlx::query("DELETE FROM messages WHERE channel_id=?1").bind(&channel_id).execute(&state.pool).await.ok();
    sqlx::query("DELETE FROM channels WHERE id=?1").bind(&channel_id).execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}
