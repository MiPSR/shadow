use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    #[serde(default = "default_server")]
    pub server_url: String,
    #[serde(default)]
    pub admin_token: Option<String>,
}

fn default_server() -> String { "http://127.0.0.1:8080".into() }

impl Default for ClientConfig {
    fn default() -> Self { Self { server_url: default_server(), admin_token: None } }
}

pub fn config_path() -> PathBuf {
    let proj = directories::ProjectDirs::from("com", "shadow", "shadow").expect("dirs");
    proj.config_dir().join("client.toml")
}

pub fn load() -> ClientConfig {
    if let Ok(s) = fs::read_to_string(config_path()) {
        toml::from_str(&s).unwrap_or_default()
    } else {
        ClientConfig::default()
    }
}

pub fn save(cfg: &ClientConfig) -> Result<(), String> {
    let p = config_path();
    let dir = p.parent().unwrap();
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let s = toml::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(), s).map_err(|e| e.to_string())
}
