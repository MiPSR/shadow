use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceKeys {
    pub account_id: String,
    pub device_id: String,
    pub mldsa_sk: String, // base64
    pub mlkem_sk: String, // base64
    pub mldsa_pk: String,
    pub mlkem_pk: String,
    pub session_token: Option<String>,
}

pub fn data_dir() -> PathBuf {
    let proj = directories::ProjectDirs::from("com", "shadow", "shadow").expect("dirs");
    proj.data_dir().to_path_buf()
}

pub fn device_path() -> PathBuf {
    data_dir().join("device.json")
}

pub fn load_device() -> Option<DeviceKeys> {
    let p = device_path();
    let s = fs::read_to_string(p).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn save_device(d: &DeviceKeys) -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let s = serde_json::to_string_pretty(d).map_err(|e| e.to_string())?;
    fs::write(device_path(), s).map_err(|e| e.to_string())
}

pub fn clear_device() {
    let _ = fs::remove_file(device_path());
}
