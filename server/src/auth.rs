use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use shared::protocol::*;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::state::AppState;

fn now() -> i64 { chrono::Utc::now().timestamp() }

fn admin_authorized(headers: &HeaderMap, state: &AppState) -> bool {
    if let Some(v) = headers.get("x-admin-token") {
        if let Ok(s) = v.to_str() {
            return s == state.config.bootstrap_admin_token;
        }
    }
    false
}

pub async fn create_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateAccountRequest>,
) -> Result<Json<CreateAccountResponse>, StatusCode> {
    if !admin_authorized(&headers, &state) {
        return Err(StatusCode::FORBIDDEN);
    }
    let id = Uuid::new_v4().to_string();
    let methods = serde_json::to_string(&req.auth_methods).unwrap();
    sqlx::query("INSERT INTO accounts (id, username, auth_methods, is_admin) VALUES (?1,?2,?3,?4)")
        .bind(&id)
        .bind(&req.username)
        .bind(&methods)
        .bind(if req.is_admin { 1 } else { 0 })
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let account = Account {
        id,
        username: req.username,
        auth_methods: req.auth_methods,
        is_admin: req.is_admin,
    };
    Ok(Json(CreateAccountResponse { account }))
}

pub async fn register_device(
    State(state): State<AppState>,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // verify account exists
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM accounts WHERE id=?1")
        .bind(&req.account_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if exists.is_none() { return Err(StatusCode::NOT_FOUND); }

    // verify signature: device signs "register:{account_id}:{device_id}:{mldsa_pk}:{mlkem_pk}"
    let msg = format!("register:{}:{}:{}:{}", req.account_id, req.device_id, req.mldsa_pk, req.mlkem_pk);
    // Actually mldsa_pk is base64 of public key; signature verifies msg
    let sig_bytes = shared::crypto::b64_decode(&req.signature).map_err(|_| StatusCode::BAD_REQUEST)?;
    // Need pk bytes from req.mldsa_pk (already base64). Use as pk.
    let pk_raw = shared::crypto::b64_decode(&req.mldsa_pk).map_err(|_| StatusCode::BAD_REQUEST)?;
    if !shared::crypto::mldsa::verify(&pk_raw, msg.as_bytes(), &sig_bytes) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    sqlx::query("INSERT OR REPLACE INTO devices (id, account_id, mldsa_pk, mlkem_pk, revoked, created_at) VALUES (?1,?2,?3,?4,0,?5)")
        .bind(&req.device_id)
        .bind(&req.account_id)
        .bind(&req.mldsa_pk)
        .bind(&req.mlkem_pk)
        .bind(now())
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(Deserialize)]
pub struct ChallengeReq { pub device_id: Id }
#[derive(Serialize)]
pub struct ChallengeResp { pub nonce: String }

pub async fn challenge(
    State(state): State<AppState>,
    Json(req): Json<ChallengeRequest>,
) -> Result<Json<ChallengeResponse>, StatusCode> {
    // device must exist and not revoked
    let row: Option<(i64,)> = sqlx::query_as("SELECT revoked FROM devices WHERE id=?1")
        .bind(&req.device_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some((revoked,)) = row else { return Err(StatusCode::NOT_FOUND); };
    if revoked != 0 { return Err(StatusCode::FORBIDDEN); }
    let mut nonce = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce);
    let nonce_b64 = B64.encode(nonce);
    // store nonce as challenge in sessions? We'll store a pending challenge row with token = nonce? Use sessions table with device_id and expires; token = nonce_b64, expires now+300
    sqlx::query("INSERT INTO sessions (token, device_id, expires_at) VALUES (?1,?2,?3)")
        .bind(&nonce_b64)
        .bind(&req.device_id)
        .bind(now() + 300)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ChallengeResponse { nonce: nonce_b64 }))
}

pub async fn verify(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    // find challenge session
    let row: Option<(String, i64)> = sqlx::query_as("SELECT device_id, expires_at FROM sessions WHERE token=?1")
        .bind(&req.device_id) // wait token is nonce, not device. Need pass nonce separately. Oops.
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // Our VerifyRequest lacks nonce. Change protocol? We'll adjust: nonce is challenge token; client signs nonce; server needs nonce. Add nonce field? Let's instead look up any unexpired session for device.
    let row: Option<(String, i64)> = sqlx::query_as("SELECT token, expires_at FROM sessions WHERE device_id=?1 ORDER BY expires_at DESC LIMIT 1")
        .bind(&req.device_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some((nonce, expires)) = row else { return Err(StatusCode::UNAUTHORIZED); };
    if expires < now() { return Err(StatusCode::UNAUTHORIZED); }
    // get device pk
    let pk_row: Option<(String, i64)> = sqlx::query_as("SELECT mldsa_pk, revoked FROM devices WHERE id=?1")
        .bind(&req.device_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some((pk_b64, revoked)) = pk_row else { return Err(StatusCode::NOT_FOUND); };
    if revoked != 0 { return Err(StatusCode::FORBIDDEN); }
    let pk = shared::crypto::b64_decode(&pk_b64).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let sig = shared::crypto::b64_decode(&req.signature).map_err(|_| StatusCode::BAD_REQUEST)?;
    let nonce_bytes = shared::crypto::b64_decode(&nonce).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !shared::crypto::mldsa::verify(&pk, &nonce_bytes, &sig) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    // issue session token
    let token = Uuid::new_v4().to_string();
    // remove challenge session rows for device
    sqlx::query("DELETE FROM sessions WHERE device_id=?1 AND token=?2").bind(&req.device_id).bind(&nonce).execute(&state.pool).await.ok();
    sqlx::query("INSERT INTO sessions (token, device_id, expires_at) VALUES (?1,?2,?3)")
        .bind(&token)
        .bind(&req.device_id)
        .bind(now() + 86400)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(VerifyResponse { session_token: token }))
}

pub async fn device_from_token(pool: &SqlitePool, token: &str) -> Option<String> {
    let row: Option<(String, i64)> = sqlx::query_as("SELECT device_id, expires_at FROM sessions WHERE token=?1")
        .bind(token)
        .fetch_optional(pool)
        .await
        .ok()?;
    let (dev, exp) = row?;
    if exp < now() { return None; }
    Some(dev)
}

pub async fn list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(account_id): axum::extract::Path<String>,
) -> Result<Json<Vec<shared::protocol::Device>>, StatusCode> {
    let token = headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let rows: Vec<(String,String,String,String,i64)> = sqlx::query_as(
        "SELECT id, account_id, mldsa_pk, mlkem_pk, revoked FROM devices WHERE account_id=?1"
    )
        .bind(&account_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out = rows.into_iter().map(|(id, account_id, mldsa_pk, mlkem_pk, revoked)| {
        shared::protocol::Device { id, account_id, mldsa_pk, mlkem_pk, revoked: revoked != 0 }
    }).collect();
    Ok(Json(out))
}

pub async fn get_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(device_id): axum::extract::Path<String>,
) -> Result<Json<shared::protocol::Device>, StatusCode> {
    let token = headers.get("authorization").and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let _dev = device_from_token(&state.pool, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let row: Option<(String,String,String,String,i64)> = sqlx::query_as(
        "SELECT id, account_id, mldsa_pk, mlkem_pk, revoked FROM devices WHERE id=?1"
    )
        .bind(&device_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some((id, account_id, mldsa_pk, mlkem_pk, revoked)) = row else { return Err(StatusCode::NOT_FOUND); };
    Ok(Json(shared::protocol::Device { id, account_id, mldsa_pk, mlkem_pk, revoked: revoked != 0 }))
}

pub async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(account_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !admin_authorized(&headers, &state) { return Err(StatusCode::FORBIDDEN); }
    sqlx::query("DELETE FROM devices WHERE account_id=?1").bind(&account_id).execute(&state.pool).await.ok();
    sqlx::query("DELETE FROM accounts WHERE id=?1").bind(&account_id).execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"ok": true})))
}

pub async fn list_accounts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<shared::protocol::Account>>, StatusCode> {
    if !admin_authorized(&headers, &state) { return Err(StatusCode::FORBIDDEN); }
    let rows: Vec<(String,String,String,i64)> = sqlx::query_as("SELECT id, username, auth_methods, is_admin FROM accounts")
        .fetch_all(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let out = rows.into_iter().map(|(id,username,methods,is_admin)| {
        let auth_methods: Vec<shared::protocol::AuthMethod> = serde_json::from_str(&methods).unwrap_or_default();
        shared::protocol::Account { id, username, auth_methods, is_admin: is_admin != 0 }
    }).collect();
    Ok(Json(out))
}

pub async fn revoke_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(device_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !admin_authorized(&headers, &state) { return Err(StatusCode::FORBIDDEN); }
    sqlx::query("UPDATE devices SET revoked=1 WHERE id=?1").bind(&device_id).execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // remove sessions
    sqlx::query("DELETE FROM sessions WHERE device_id=?1").bind(&device_id).execute(&state.pool).await.ok();
    Ok(Json(serde_json::json!({"ok": true})))
}
