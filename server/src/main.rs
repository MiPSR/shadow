mod auth;
mod channels;
mod config;
mod db;
mod keys;
mod attachments;
mod audit;
mod messages;
mod stats;
mod p2p;
mod settings;
mod signal;
mod streams;
mod state;
mod ws;

use crate::config::ServerConfig;
use crate::state::AppState;
use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use std::collections::HashMap;
use std::sync::Mutex;
use shared::crypto;
use tokio::sync::broadcast;
use sqlx;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cfg = config::load("server.toml");
    let pool = db::connect(&cfg.database_url).await;
    db::migrate(&pool).await;
    let (tx, _rx) = broadcast::channel(256);
    let (server_pk, server_sk) = crypto::mldsa::keypair();
    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(cfg.clone()),
        broadcast: tx,
        server_mldsa_pk: server_pk,
        server_mldsa_sk: server_sk,
        pubkey_cache: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(|| async { axum::Json(serde_json::json!({"status": "ok", "version": "0.1.0"})) }))
        .route("/ready", get(|| async { "ready" }))
        .route("/admin/audit", get(audit::list_audit))
        .route("/admin/stats", get(stats::admin_stats))
        .route("/metrics", get(|| async { "messages_total 0\n" }))
        .route("/version", get(|| async { env!("CARGO_PKG_VERSION") }))
        .route("/admin/accounts", post(auth::create_account))
        .route("/devices/register", post(auth::register_device))
        .route("/auth/challenge", post(auth::challenge))
        .route("/auth/verify", post(auth::verify))
        .route("/channels", post(channels::create_channel).get(channels::list_channels))
        .route("/channels/search", get(channels::search_channels))
        .route("/channels/:channel_id/members", axum::routing::post(channels::add_member))
        .route("/channels/:channel_id/members/:account_id", axum::routing::delete(channels::remove_member))
        .route("/channels/:channel_id", axum::routing::patch(channels::rename_channel))
        .route("/channels/:channel_id", axum::routing::delete(channels::delete_channel))
        .route("/messages", post(messages::send_message))
        .route("/messages/:channel_id", get(messages::list_messages))
        .route("/messages/:id", axum::routing::delete(messages::delete_message).put(messages::edit_message))
        .route("/messages/:id/pin", axum::routing::post(messages::pin_message))
        .route("/keys/announce", post(keys::announce_key))
        .route("/keys/announcements", get(keys::fetch_key_announcements))
        .route("/admin/settings", get(settings::get_settings).post(settings::set_setting))
        .route("/signal", post(signal::send_signal))
        .route("/attachments", post(attachments::upload))
        .route("/attachments/:id", get(attachments::download).delete(attachments::delete_attachment))
        .route("/p2p/server-key", get(p2p::server_key))
        .route("/p2p/authorize", post(p2p::authorize))
        .route("/signals", get(signal::poll_signals))
        .route("/streams/join", post(streams::join_stream))
        .route("/admin/accounts/:account_id", axum::routing::delete(auth::delete_account))
        .route("/admin/accounts/:account_id/data", get(auth::account_data))
        .route("/admin/accounts", get(auth::list_accounts))
        .route("/admin/accounts/search", get(auth::search_accounts))
        .route("/admin/devices/:device_id/revoke", axum::routing::post(auth::revoke_device))
        .route("/admin/accounts/:account_id/revoke-all", axum::routing::post(auth::revoke_all_devices))
        .route("/accounts/:account_id/devices", get(auth::list_devices))
        .route("/devices/:device_id", get(auth::get_device))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .merge(ws::router())
        .fallback_service(tower_http::services::ServeDir::new(cfg.web_root.clone().unwrap_or_else(|| "web".into())))
        .with_state(state.clone());

    let addr: std::net::SocketAddr = cfg.bind.parse().unwrap();
    tracing::info!("listening on {}", addr);
    // retention cleanup task
    let pool2 = pool.clone();
    let cache2 = state.pubkey_cache.clone();
    let retention = cfg.message_retention_days;
    tokio::spawn(async move {
        loop {
            let cutoff = chrono::Utc::now().timestamp() - retention * 86400;
            let _ = sqlx::query("DELETE FROM messages WHERE created_at < ?1").bind(cutoff).execute(&pool2).await;
            let _ = sqlx::query("DELETE FROM sessions WHERE expires_at < ?1").bind(chrono::Utc::now().timestamp()).execute(&pool2).await;
            let _ = sqlx::query("DELETE FROM signals WHERE created_at < ?1").bind(chrono::Utc::now().timestamp() - 300).execute(&pool2).await;
            // clean expired pubkey cache
            if let Ok(mut c) = cache2.lock() {
                c.retain(|_, v| v.expires_at > chrono::Utc::now().timestamp());
            }
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
