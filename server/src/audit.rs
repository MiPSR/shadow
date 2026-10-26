use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use crate::state::AppState;

fn token(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn log_action(pool: &sqlx::SqlitePool, action: &str, actor: Option<&str>, target: Option<&str>) {
    let _ = sqlx::query("INSERT INTO audit_log (action, actor, target, created_at) VALUES (?1,?2,?3,?4)")
        .bind(action).bind(actor).bind(target).bind(chrono::Utc::now().timestamp())
        .execute(pool).await;
}

pub async fn list_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = crate::auth::device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    // admin check
    let acct: Option<String> = sqlx::query_scalar("SELECT account_id FROM devices WHERE id=?1").bind(&dev).fetch_optional(&state.pool).await.ok().flatten();
    let is_admin = if let Some(a) = acct { sqlx::query_scalar::<_, i64>("SELECT is_admin FROM accounts WHERE id=?1").bind(&a).fetch_optional(&state.pool).await.ok().flatten().unwrap_or(0) != 0 } else { false };
    if !is_admin { return Err(StatusCode::FORBIDDEN); }
    let rows: Vec<(i64,String,Option<String>,Option<String>,i64)> = sqlx::query_as("SELECT id, action, actor, target, created_at FROM audit_log ORDER BY created_at DESC LIMIT 100")
        .fetch_all(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out = rows.into_iter().map(|(id,action,actor,target,created_at)| serde_json::json!({"id": id, "action": action, "actor": actor, "target": target, "created_at": created_at})).collect();
    Ok(Json(out))
}
