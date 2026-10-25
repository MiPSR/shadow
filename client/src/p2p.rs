#[cfg(not(target_arch = "wasm32"))]
pub fn verify_credential(server_pk_b64: &str, from_device: &str, action: &str, peer_device: &str, channel_id: &str, expires_at: i64, sig_b64: &str) -> bool {
    use shared::crypto;
    let pk = match crypto::b64_decode(server_pk_b64) { Ok(b) => b, Err(_) => return false };
    let sig = match crypto::b64_decode(sig_b64) { Ok(b) => b, Err(_) => return false };
    let msg = format!("p2p:{}:{}:{}:{}:{}", from_device, action, peer_device, channel_id, expires_at);
    crypto::mldsa::verify(&pk, msg.as_bytes(), &sig)
}
