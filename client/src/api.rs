use reqwest::blocking::Client;
use shared::protocol::*;
use std::time::Duration;

pub struct Api {
    pub base: String,
    pub client: Client,
}

impl Api {
    pub fn new(base: String) -> Self {
        Self { base: base.trim_end_matches('/').to_string(), client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap() }
    }

    fn url(&self, path: &str) -> String { format!("{}{}", self.base, path) }

    pub fn create_account(&self, admin_token: &str, req: &CreateAccountRequest) -> Result<Account, String> {
        let resp = self.client.post(self.url("/admin/accounts"))
            .header("x-admin-token", admin_token)
            .json(req).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        let r: CreateAccountResponse = resp.json().map_err(|e| e.to_string())?;
        Ok(r.account)
    }

    pub fn register_device(&self, req: &RegisterDeviceRequest) -> Result<(), String> {
        let resp = self.client.post(self.url("/devices/register")).json(req).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }

    pub fn challenge(&self, device_id: &str) -> Result<String, String> {
        let resp = self.client.post(self.url("/auth/challenge")).json(&ChallengeRequest { device_id: device_id.into() }).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        let r: ChallengeResponse = resp.json().map_err(|e| e.to_string())?;
        Ok(r.nonce)
    }

    pub fn verify(&self, device_id: &str, signature_b64: &str) -> Result<String, String> {
        let resp = self.client.post(self.url("/auth/verify")).json(&VerifyRequest { device_id: device_id.into(), signature: signature_b64.into() }).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        let r: VerifyResponse = resp.json().map_err(|e| e.to_string())?;
        Ok(r.session_token)
    }

    pub fn create_channel(&self, token: &str, req: &CreateChannelRequest) -> Result<Channel, String> {
        let resp = self.client.post(self.url("/channels")).bearer_auth(token).json(req).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }

    pub fn list_channels(&self, token: &str) -> Result<Vec<Channel>, String> {
        let resp = self.client.get(self.url("/channels")).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }

    pub fn send_message(&self, token: &str, req: &SendMessageRequest) -> Result<EncryptedMessage, String> {
        let resp = self.client.post(self.url("/messages")).bearer_auth(token).json(req).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }

    pub fn list_messages(&self, token: &str, channel_id: &str) -> Result<Vec<EncryptedMessage>, String> {
        let resp = self.client.get(self.url(&format!("/messages/{}", channel_id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }

    pub fn announce_key(&self, token: &str, ann: &KeyAnnouncement) -> Result<(), String> {
        let resp = self.client.post(self.url("/keys/announce")).bearer_auth(token).json(ann).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }

    pub fn fetch_key_announcements(&self, token: &str) -> Result<Vec<KeyAnnouncement>, String> {
        let resp = self.client.get(self.url("/keys/announcements")).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }

    pub fn list_devices(&self, token: &str, account_id: &str) -> Result<Vec<Device>, String> {
        let resp = self.client.get(self.url(&format!("/accounts/{}/devices", account_id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn get_device(&self, token: &str, device_id: &str) -> Result<Device, String> {
        let resp = self.client.get(self.url(&format!("/devices/{}", device_id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn upload_attachment(&self, token: &str, media_type: &str, payload_b64: &str, message_id: Option<&str>) -> Result<String, String> {
        let mut body = serde_json::json!({"media_type": media_type, "payload": payload_b64});
        if let Some(m) = message_id { body["message_id"] = serde_json::json!(m); }
        let resp = self.client.post(self.url("/attachments")).bearer_auth(token).json(&body).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        let v: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        Ok(v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string())
    }
    pub fn download_attachment(&self, token: &str, id: &str) -> Result<(String,String), String> {
        let resp = self.client.get(self.url(&format!("/attachments/{}", id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        let v: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        let payload = v.get("payload").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let mt = v.get("media_type").and_then(|x| x.as_str()).unwrap_or("").to_string();
        Ok((payload, mt))
    }
}

impl Api {
    pub fn list_accounts(&self, admin_token: &str) -> Result<Vec<Account>, String> {
        let resp = self.client.get(self.url("/admin/accounts")).header("x-admin-token", admin_token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
    pub fn revoke_device(&self, admin_token: &str, device_id: &str) -> Result<(), String> {
        let resp = self.client.post(self.url(&format!("/admin/devices/{}/revoke", device_id))).header("x-admin-token", admin_token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
    pub fn delete_account_admin(&self, admin_token: &str, account_id: &str) -> Result<(), String> {
        let resp = self.client.delete(self.url(&format!("/admin/accounts/{}", account_id))).header("x-admin-token", admin_token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
    pub fn get_settings(&self, admin_token: &str) -> Result<serde_json::Value, String> {
        let resp = self.client.get(self.url("/admin/settings")).header("x-admin-token", admin_token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
    pub fn set_setting(&self, admin_token: &str, key: &str, value: &str) -> Result<(), String> {
        let mut body = serde_json::Map::new();
        body.insert(key.to_string(), serde_json::json!(value));
        let resp = self.client.post(self.url("/admin/settings")).header("x-admin-token", admin_token).json(&body).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
}

impl Api {
    pub fn server_key(&self) -> Result<String, String> {
        let resp = self.client.get(self.url("/p2p/server-key")).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        let v: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        Ok(v.get("public_key").and_then(|x| x.as_str()).unwrap_or("").to_string())
    }
    pub fn authorize_p2p(&self, token: &str, action: &str, peer_device: &str, channel_id: Option<&str>) -> Result<serde_json::Value, String> {
        let mut body = serde_json::json!({"action": action, "peer_device": peer_device});
        if let Some(c) = channel_id { body["channel_id"] = serde_json::json!(c); }
        let resp = self.client.post(self.url("/p2p/authorize")).bearer_auth(token).json(&body).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn add_channel_member(&self, token: &str, channel_id: &str, account_id: &str) -> Result<Channel, String> {
        let body = serde_json::json!({"account_id": account_id});
        let resp = self.client.post(self.url(&format!("/channels/{}/members", channel_id))).bearer_auth(token).json(&body).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn account_data(&self, admin_token: &str, account_id: &str) -> Result<serde_json::Value, String> {
        let resp = self.client.get(self.url(&format!("/admin/accounts/{}/data", account_id))).header("x-admin-token", admin_token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn send_signal(&self, token: &str, to_device: &str, kind: &str, payload: &str, channel_id: Option<&str>) -> Result<(), String> {
        let mut body = serde_json::json!({"to_device": to_device, "kind": kind, "payload": payload});
        if let Some(c) = channel_id { body["channel_id"] = serde_json::json!(c); }
        let resp = self.client.post(self.url("/signal")).bearer_auth(token).json(&body).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
    pub fn poll_signals(&self, token: &str) -> Result<Vec<serde_json::Value>, String> {
        let resp = self.client.get(self.url("/signals")).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn join_stream(&self, token: &str, channel_id: &str) -> Result<(), String> {
        let body = serde_json::json!({"channel_id": channel_id});
        let resp = self.client.post(self.url("/streams/join")).bearer_auth(token).json(&body).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
}

impl Api {
    pub fn delete_channel(&self, token: &str, channel_id: &str) -> Result<(), String> {
        let resp = self.client.delete(self.url(&format!("/channels/{}", channel_id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
}

impl Api {
    pub fn delete_message(&self, token: &str, id: &str) -> Result<(), String> {
        let resp = self.client.delete(self.url(&format!("/messages/{}", id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
}

impl Api {
    pub fn delete_attachment(&self, token: &str, id: &str) -> Result<(), String> {
        let resp = self.client.delete(self.url(&format!("/attachments/{}", id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
}

impl Api {
    pub fn revoke_all_devices(&self, admin_token: &str, account_id: &str) -> Result<(), String> {
        let resp = self.client.post(self.url(&format!("/admin/accounts/{}/revoke-all", account_id))).header("x-admin-token", admin_token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
}

impl Api {
    pub fn health(&self) -> Result<String, String> {
        let resp = self.client.get(self.url("/health")).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(resp.text().unwrap_or_default())
    }
}

impl Api {
    pub fn search_channels(&self, token: &str, q: &str) -> Result<Vec<Channel>, String> {
        let resp = self.client.get(self.url(&format!("/channels/search?q={}", urlencoding::encode(q)))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn remove_channel_member(&self, token: &str, channel_id: &str, account_id: &str) -> Result<Channel, String> {
        let resp = self.client.delete(self.url(&format!("/channels/{}/members/{}", channel_id, account_id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn list_audit(&self, token: &str) -> Result<Vec<serde_json::Value>, String> {
        let resp = self.client.get(self.url("/admin/audit")).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn admin_stats(&self, token: &str) -> Result<serde_json::Value, String> {
        let resp = self.client.get(self.url("/admin/stats")).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn search_accounts(&self, admin_token: &str, q: &str) -> Result<Vec<Account>, String> {
        let resp = self.client.get(self.url(&format!("/admin/accounts/search?q={}", urlencoding::encode(q)))).header("x-admin-token", admin_token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn rename_channel(&self, token: &str, channel_id: &str, name: &str) -> Result<Channel, String> {
        let body = serde_json::json!({"name": name});
        let resp = self.client.patch(self.url(&format!("/channels/{}", channel_id))).bearer_auth(token).json(&body).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
    pub fn clear_channel(&self, token: &str, channel_id: &str) -> Result<(), String> {
        let resp = self.client.post(self.url(&format!("/channels/{}/clear", channel_id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
}

impl Api {
    pub fn edit_message(&self, token: &str, id: &str, payload: &str) -> Result<EncryptedMessage, String> {
        let body = serde_json::json!({"payload": payload});
        let resp = self.client.put(self.url(&format!("/messages/{}", id))).bearer_auth(token).json(&body).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        resp.json().map_err(|e| e.to_string())
    }
}

impl Api {
}

impl Api {
    pub fn pin_message(&self, token: &str, id: &str) -> Result<(), String> {
        let resp = self.client.post(self.url(&format!("/messages/{}/pin", id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
    pub fn archive_channel(&self, token: &str, channel_id: &str) -> Result<(), String> {
        let resp = self.client.post(self.url(&format!("/channels/{}/archive", channel_id))).bearer_auth(token).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(())
    }
}

impl Api {
    pub fn version(&self) -> Result<String, String> {
        let resp = self.client.get(self.url("/version")).send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() { return Err(resp.text().unwrap_or_default()); }
        Ok(resp.text().unwrap_or_default())
    }
}
