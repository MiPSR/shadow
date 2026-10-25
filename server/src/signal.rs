use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use crate::state::AppState;
use crate::auth::device_from_token;

fn token(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
}

pub async fn send_signal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let to = body.get("to_device").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?.to_string();
    let kind = body.get("kind").and_then(|v| v.as_str()).unwrap_or("offer").to_string();
    let payload = body.get("payload").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let channel = body.get("channel_id").and_then(|v| v.as_str()).map(|s| s.to_string());
    sqlx::query("INSERT INTO signals (from_device, to_device, channel_id, kind, payload, created_at) VALUES (?1,?2,?3,?4,?5,?6)")
        .bind(&dev).bind(&to).bind(&channel).bind(&kind).bind(&payload).bind(chrono::Utc::now().timestamp())
        .execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn poll_signals(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let tok = token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let dev = device_from_token(&state.pool, &tok).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let rows: Vec<(i64,String,String,Option<String>,String,String)> = sqlx::query_as(
        "SELECT id, from_device, to_device, channel_id, kind, payload FROM signals WHERE to_device=?1"
    )
        .bind(&dev)
        .fetch_all(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out: Vec<serde_json::Value> = rows.into_iter().map(|(id,f,t,c,k,p)| {
        serde_json::json!({"id": id, "from_device": f, "to_device": t, "channel_id": c, "kind": k, "payload": p})
    }).collect();
    // delete polled
    sqlx::query("DELETE FROM signals WHERE to_device=?1").bind(&dev).execute(&state.pool).await.ok();
    Ok(Json(out))
}
