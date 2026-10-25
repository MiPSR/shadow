use crate::config::ServerConfig;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<ServerConfig>,
    pub broadcast: broadcast::Sender<String>,
}
