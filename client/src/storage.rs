use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceKeys {
    pub account_id: String,
    pub device_id: String,
    pub mldsa_sk: String,
    pub mlkem_sk: String,
    pub mldsa_pk: String,
    pub mlkem_pk: String,
    pub session_token: Option<String>,
}

pub fn data_dir() -> PathBuf {
    let proj = directories::ProjectDirs::from("com", "shadow", "shadow").expect("dirs");
    proj.data_dir().to_path_buf()
}

fn entry() -> Option<Entry> { Entry::new("shadow", "device").ok() }

pub fn load_device() -> Option<DeviceKeys> {
    if let Some(e) = entry() {
        if let Ok(s) = e.get_password() {
            if let Ok(d) = serde_json::from_str(&s) { return Some(d); }
        }
    }
    // fallback file
    let p = data_dir().join("device.json");
    let s = fs::read_to_string(p).ok()?;
    serde_json::from_str(&s).ok()
}

pub fn save_device(d: &DeviceKeys) -> Result<(), String> {
    let s = serde_json::to_string_pretty(d).map_err(|e| e.to_string())?;
    if let Some(e) = entry() {
        if e.set_password(&s).is_ok() { return Ok(()); }
    }
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join("device.json"), s).map_err(|e| e.to_string())
}

pub fn clear_device() {
    if let Some(e) = entry() { let _ = e.delete_credential(); }
    let _ = fs::remove_file(data_dir().join("device.json"));
}
