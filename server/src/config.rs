use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_database")]
    pub database_url: String,
    #[serde(default = "default_bootstrap")]
    pub bootstrap_admin_token: String,
    #[serde(default = "default_retention_days")]
    pub message_retention_days: i64,
    #[serde(default = "default_key_cache_hours")]
    pub key_cache_hours: i64,
    #[serde(default = "default_max_upload")]
    pub max_upload_bytes: u64,
    #[serde(default = "default_web_root")]
    pub web_root: Option<String>,
}

fn default_bind() -> String { "0.0.0.0:8080".into() }
fn default_database() -> String { "sqlite:shadow.db?mode=rwc".into() }
fn default_bootstrap() -> String { "change-me".into() }
fn default_retention_days() -> i64 { 7 }
fn default_key_cache_hours() -> i64 { 24 }
fn default_max_upload() -> u64 { 10 * 1024 * 1024 }
fn default_web_root() -> Option<String> { None }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            database_url: default_database(),
            bootstrap_admin_token: default_bootstrap(),
            message_retention_days: default_retention_days(),
            key_cache_hours: default_key_cache_hours(),
            max_upload_bytes: default_max_upload(),
            web_root: default_web_root(),
        }
    }
}

pub fn load(path: &str) -> ServerConfig {
    if let Ok(s) = std::fs::read_to_string(path) {
        toml::from_str(&s).unwrap_or_default()
    } else {
        ServerConfig::default()
    }
}
