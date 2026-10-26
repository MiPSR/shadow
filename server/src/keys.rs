use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use shared::protocol::*;
use crate::state::AppState;
use crate::auth::device_from_token;

fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn announce_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(ann): Json<KeyAnnouncement>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token = token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    if dev != ann.from_device { return Err(StatusCode::FORBIDDEN); }
    // verify from_device not revoked and signature
    let from_row: Option<(String, i64)> = sqlx::query_as("SELECT mldsa_pk, revoked FROM devices WHERE id=?1")
        .bind(&ann.from_device).fetch_optional(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some((pk_b64, revoked)) = from_row else { return Err(StatusCode::NOT_FOUND); };
    if revoked != 0 { return Err(StatusCode::FORBIDDEN); }
    let to_exists: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM devices WHERE id=?1")
        .bind(&ann.to_device).fetch_optional(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if to_exists.is_none() { return Err(StatusCode::NOT_FOUND); }
    let pk = shared::crypto::b64_decode(&pk_b64).map_err(|_| StatusCode::BAD_REQUEST)?;
    let sig = shared::crypto::b64_decode(&ann.signature).map_err(|_| StatusCode::BAD_REQUEST)?;
    let channel = ann.channel_id.clone().unwrap_or_default();
    let msg = format!("key:{}:{}:{}", ann.from_device, ann.to_device, channel);
    if !shared::crypto::mldsa::verify(&pk, msg.as_bytes(), &sig) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    sqlx::query("INSERT INTO key_announcements (from_device, to_device, channel_id, kem_ciphertext, nonce, wrapped_key, signature, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)")
        .bind(&ann.from_device)
        .bind(&ann.to_device)
        .bind(&ann.channel_id)
        .bind(&ann.kem_ciphertext)
        .bind(&ann.nonce)
        .bind(&ann.wrapped_key)
        .bind(&ann.signature)
        .bind(chrono::Utc::now().timestamp())
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // broadcast via ws? HTTP only; client polls
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn fetch_key_announcements(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<KeyAnnouncement>>, StatusCode> {
    let token = token_from_headers(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let rows: Vec<(String,String,Option<String>,String,String,String,String)> = sqlx::query_as(
        "SELECT from_device, to_device, channel_id, kem_ciphertext, nonce, wrapped_key, signature FROM key_announcements WHERE to_device=?1"
    )
        .bind(&dev)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out = rows.into_iter().map(|(f,t,c,k,n,w,s)| KeyAnnouncement {
        from_device: f,
        to_device: t,
        channel_id: c,
        kem_ciphertext: k,
        nonce: n,
        wrapped_key: w,
        signature: s,
    }).collect();
    Ok(Json(out))
}
