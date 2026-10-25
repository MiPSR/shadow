use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use shared::crypto;
use crate::storage::DeviceKeys;

pub fn export(keys: &DeviceKeys, passphrase: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(passphrase.as_bytes(), &salt).map_err(|e| e.to_string())?;
    // derive 32 bytes from hash output
    let mut key = [0u8; 32];
    let out = hash.hash.unwrap();
    let bytes = out.as_bytes();
    for i in 0..32 { key[i] = bytes[i % bytes.len()]; }
    let json = serde_json::to_string(keys).map_err(|e| e.to_string())?;
    let (nonce, ct) = crypto::xchacha::encrypt(&key, json.as_bytes(), &[]).map_err(|e| e.to_string())?;
    let mut outbuf = salt.as_str().as_bytes().to_vec();
    outbuf.push(b':');
    outbuf.extend_from_slice(&nonce);
    outbuf.push(b':');
    outbuf.extend_from_slice(&ct);
    Ok(crypto::b64_encode(&outbuf))
}

pub fn import(blob_b64: &str, passphrase: &str) -> Result<DeviceKeys, String> {
    let raw = crypto::b64_decode(blob_b64)?;
    let parts: Vec<&[u8]> = raw.split(|&b| b == b':').collect();
    if parts.len() != 3 { return Err("bad backup format".into()); }
    let salt_str = std::str::from_utf8(parts[0]).map_err(|e| e.to_string())?;
    let salt = SaltString::from_b64(salt_str).map_err(|e| e.to_string())?;
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(passphrase.as_bytes(), &salt).map_err(|e| e.to_string())?;
    let mut key = [0u8; 32];
    let out = hash.hash.unwrap();
    let bytes = out.as_bytes();
    for i in 0..32 { key[i] = bytes[i % bytes.len()]; }
    let pt = crypto::xchacha::decrypt(&key, parts[1], parts[2], &[]).map_err(|e| e.to_string())?;
    let json = String::from_utf8(pt).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}
