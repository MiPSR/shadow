use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use crate::state::AppState;
use crate::auth::device_from_token;

fn token(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn server_key(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"public_key": B64.encode(&state.server_mldsa_pk)}))
}

pub async fn authorize(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let action = body.get("action").and_then(|v| v.as_str()).unwrap_or("p2p").to_string();
    let peer = body.get("peer_device").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let channel = body.get("channel_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    let expires = chrono::Utc::now().timestamp() + 300;
    let msg = format!("p2p:{}:{}:{}:{}:{}", dev, action, peer, channel.clone().unwrap_or_default(), expires);
    let sig = shared::crypto::mldsa::sign(&state.server_mldsa_sk, msg.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({
        "from_device": dev,
        "action": action,
        "peer_device": peer,
        "channel_id": channel,
        "expires_at": expires,
        "signature": B64.encode(sig),
    })))
}
