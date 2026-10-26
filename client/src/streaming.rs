#[cfg(not(target_arch = "wasm32"))]
pub fn negotiate_stream(api: &crate::api::Api, token: &str, peer_device: &str, channel_id: &str, offered: &[&str]) -> Result<String, String> {
    use serde_json::json;
    let payload = json!({"codecs": offered}).to_string();
    api.send_signal(token, peer_device, "stream-offer", &payload, Some(channel_id))?;
    // poll for answer for up to 10s
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Ok(sigs) = api.poll_signals(token) {
            for s in sigs {
                if s.get("kind").and_then(|x| x.as_str()) == Some("stream-answer") {
                    if let Some(sel) = s.get("payload").and_then(|x| x.as_str()).and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok()).and_then(|v| v.get("selected").and_then(|x| x.as_str()).map(|s| s.to_string())) {
                        return Ok(sel);
                    }
                }
            }
        }
    }
    Err("negotiation timeout".into())
}
