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
