use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode}, Json};
use crate::state::AppState;
use crate::auth::device_from_token;

fn token(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let media_type = body.get("media_type").and_then(|v| v.as_str()).unwrap_or("application/octet-stream").to_string();
    let payload = body.get("payload").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
    if payload.len() as u64 > state.config.max_upload_bytes { return Err(StatusCode::PAYLOAD_TOO_LARGE); }
    let id = uuid::Uuid::new_v4().to_string();
    let size = payload.len() as i64;
    let msg_id = body.get("message_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    sqlx::query("INSERT INTO attachments (id, message_id, size, media_type, payload, created_at) VALUES (?1,?2,?3,?4,?5,?6)")
        .bind(&id).bind(&msg_id).bind(size).bind(&media_type).bind(payload).bind(chrono::Utc::now().timestamp())
        .execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"id": id})))
}

pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let row: Option<(String,String,i64)> = sqlx::query_as("SELECT payload, media_type, size FROM attachments WHERE id=?1")
        .bind(&id).fetch_optional(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some((payload, media_type, size)) = row else { return Err(StatusCode::NOT_FOUND); };
    Ok(Json(serde_json::json!({"id": id, "payload": payload, "media_type": media_type, "size": size})))
}

pub async fn delete_attachment(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    sqlx::query("DELETE FROM attachments WHERE id=?1").bind(&id).execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}
