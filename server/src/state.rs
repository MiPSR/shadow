use crate::config::ServerConfig;
use shared::crypto;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<ServerConfig>,
    pub broadcast: broadcast::Sender<String>,
    pub server_mldsa_pk: Vec<u8>,
    pub server_mldsa_sk: Vec<u8>,
}
