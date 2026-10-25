use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use crate::state::AppState;

fn admin_ok(headers: &HeaderMap, state: &AppState) -> bool {
    headers.get("x-admin-token").and_then(|v| v.to_str().ok()).map(|s| s == state.config.bootstrap_admin_token).unwrap_or(false)
}

pub async fn get_settings(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
    if !admin_ok(&headers, &state) { return Err(StatusCode::FORBIDDEN); }
    let rows: Vec<(String,String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut map = serde_json::Map::new();
    for (k,v) in rows { map.insert(k, serde_json::Value::String(v)); }
    Ok(Json(serde_json::Value::Object(map)))
}

pub async fn set_setting(State(state): State<AppState>, headers: HeaderMap, Json(body): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !admin_ok(&headers, &state) { return Err(StatusCode::FORBIDDEN); }
    if let Some(obj) = body.as_object() {
        for (k,v) in obj {
            // validate known settings
            if !matches!(k.as_str(), "message_retention_days" | "key_cache_hours" | "max_upload_bytes") { continue; }
            let val = v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string());
            sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?1,?2)")
                .bind(k).bind(&val).execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    Ok(Json(serde_json::json!({"ok": true})))
}
