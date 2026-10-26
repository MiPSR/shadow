use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use crate::state::AppState;

fn token(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn admin_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = crate::auth::device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let acct: Option<String> = sqlx::query_scalar("SELECT account_id FROM devices WHERE id=?1").bind(&dev).fetch_optional(&state.pool).await.ok().flatten();
    let is_admin = if let Some(a) = acct { sqlx::query_scalar::<_, i64>("SELECT is_admin FROM accounts WHERE id=?1").bind(&a).fetch_optional(&state.pool).await.ok().flatten().unwrap_or(0) != 0 } else { false };
    if !is_admin { return Err(StatusCode::FORBIDDEN); }
    let accounts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts").fetch_one(&state.pool).await.unwrap_or(0);
    let devices: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM devices").fetch_one(&state.pool).await.unwrap_or(0);
    let channels: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channels").fetch_one(&state.pool).await.unwrap_or(0);
    let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages").fetch_one(&state.pool).await.unwrap_or(0);
    Ok(Json(serde_json::json!({"accounts": accounts, "devices": devices, "channels": channels, "messages": messages})))
}
