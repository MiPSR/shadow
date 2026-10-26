use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use crate::state::AppState;
use crate::auth::device_from_token;
use crate::channels;

fn token(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn join_stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let channel_id = body.get("channel_id").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?.to_string();
    let acct = channels::device_account(&state.pool, &dev).await.ok_or(StatusCode::FORBIDDEN)?;
    if !channels::is_channel_member(&state.pool, &channel_id, &acct).await {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(Json(serde_json::json!({"ok": true, "channel_id": channel_id})))
}
