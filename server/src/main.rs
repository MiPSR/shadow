mod auth;
mod channels;
mod config;
mod db;
mod keys;
mod attachments;
mod messages;
mod p2p;
mod settings;
mod signal;
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
        .route("/health", get(|| async { "ok" }))
        .route("/admin/accounts", post(auth::create_account))
        .route("/devices/register", post(auth::register_device))
        .route("/auth/challenge", post(auth::challenge))
        .route("/auth/verify", post(auth::verify))
        .route("/channels", post(channels::create_channel).get(channels::list_channels))
        .route("/messages", post(messages::send_message))
        .route("/messages/:channel_id", get(messages::list_messages))
        .route("/keys/announce", post(keys::announce_key))
        .route("/keys/announcements", get(keys::fetch_key_announcements))
        .route("/admin/settings", get(settings::get_settings).post(settings::set_setting))
        .route("/signal", post(signal::send_signal))
        .route("/attachments", post(attachments::upload))
        .route("/attachments/:id", get(attachments::download))
        .route("/p2p/server-key", get(p2p::server_key))
        .route("/p2p/authorize", post(p2p::authorize))
        .route("/signals", get(signal::poll_signals))
        .route("/admin/accounts/:account_id", axum::routing::delete(auth::delete_account))
        .route("/admin/accounts", get(auth::list_accounts))
        .route("/admin/devices/:device_id/revoke", axum::routing::post(auth::revoke_device))
        .route("/accounts/:account_id/devices", get(auth::list_devices))
        .route("/devices/:device_id", get(auth::get_device))
        .merge(ws::router())
        .with_state(state);

    let addr: std::net::SocketAddr = cfg.bind.parse().unwrap();
    tracing::info!("listening on {}", addr);
    // retention cleanup task
    let pool2 = pool.clone();
    let retention = cfg.message_retention_days;
    tokio::spawn(async move {
        loop {
            let cutoff = chrono::Utc::now().timestamp() - retention * 86400;
            let _ = sqlx::query("DELETE FROM messages WHERE created_at < ?1").bind(cutoff).execute(&pool2).await;
            let _ = sqlx::query("DELETE FROM sessions WHERE expires_at < ?1").bind(chrono::Utc::now().timestamp()).execute(&pool2).await;
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
