use crate::config::ServerConfig;
use shared::crypto;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct CachedKey {
    pub mldsa_pk: String,
    pub mlkem_pk: String,
    pub expires_at: i64,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<ServerConfig>,
    pub broadcast: broadcast::Sender<String>,
    pub server_mldsa_pk: Vec<u8>,
    pub server_mldsa_sk: Vec<u8>,
    pub pubkey_cache: Arc<Mutex<HashMap<String, CachedKey>>>,
}
