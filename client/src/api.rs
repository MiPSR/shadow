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
