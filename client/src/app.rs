use crate::api::Api;
use crate::config::{ClientConfig, load as load_config, save as save_config};
use crate::storage::{DeviceKeys, load_device, save_device, clear_device};
use egui::Context;
use shared::crypto;
use shared::protocol::*;
use std::collections::HashMap;

#[derive(PartialEq)]
pub enum Screen { Setup, Main }

pub struct ShadowApp {
    pub config: ClientConfig,
    pub api: Api,
    pub device: Option<DeviceKeys>,
    pub screen: Screen,
    // inputs
    pub server_url_input: String,
    pub admin_token_input: String,
    pub username_input: String,
    pub account_id_input: String,
    pub channel_name_input: String,
    pub message_input: String,
    pub status: String,
    // data
    pub channels: Vec<Channel>,
    pub selected_channel: Option<String>,
    pub messages: Vec<EncryptedMessage>,
    pub channel_keys: HashMap<String, Vec<([u8;32], i64)>>,
    pub ws_messages: std::sync::Arc<std::sync::Mutex<Vec<EncryptedMessage>>>,
    pub voice: Option<crate::voice::VoiceSession>,
    pub screen_session: Option<crate::screen::ScreenSession>,
    pub peer_addr_input: String,
}

impl ShadowApp {
    pub fn new() -> Self {
        let config = load_config();
        let device = load_device();
        let ws_messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::<shared::protocol::EncryptedMessage>::new()));
        if let Some(d) = &device {
            if let Some(tok) = &d.session_token {
                let url = config.server_url.replace("http", "ws") + "/ws";
                crate::wsclient::start(url, tok.clone(), ws_messages.clone());
            }
        }
        let api = Api::new(config.server_url.clone());
        Self {
            server_url_input: config.server_url.clone(),
            admin_token_input: config.admin_token.clone().unwrap_or_default(),
            config,
            api,
            device,
            screen: if load_device().is_some() { Screen::Main } else { Screen::Setup },
            username_input: String::new(),
            account_id_input: String::new(),
            channel_name_input: String::new(),
            message_input: String::new(),
            status: String::new(),
            channels: Vec::new(),
            selected_channel: None,
            messages: Vec::new(),
            channel_keys: HashMap::new(),
            ws_messages: ws_messages.clone(),
            voice: None,
            screen_session: None,
            peer_addr_input: String::new(),
        }
    }

    fn refresh_channels(&mut self) {
        if let Some(dev) = &self.device {
            if let Some(tok) = &dev.session_token {
                match self.api.list_channels(tok) {
                    Ok(chs) => { self.channels = chs; self.status = "channels loaded".into(); }
                    Err(e) => self.status = e,
                }
            }
        }
    }

    fn ensure_login(&mut self) -> bool {
        if self.device.is_none() { self.status = "no device".into(); return false; }
        let dev = self.device.as_ref().unwrap();
        if dev.session_token.is_some() { return true; }
        // challenge + verify
        let nonce_b64 = match self.api.challenge(&dev.device_id) { Ok(n) => n, Err(e) => { self.status = e; return false; } };
        let nonce = match crypto::b64_decode(&nonce_b64) { Ok(b) => b, Err(e) => { self.status = e; return false; } };
        let sk = match crypto::b64_decode(&dev.mldsa_sk) { Ok(b) => b, Err(e) => { self.status = e; return false; } };
        let sig = match crypto::mldsa::sign(&sk, &nonce) { Ok(s) => s, Err(e) => { self.status = e; return false; } };
        let sig_b64 = crypto::b64_encode(&sig);
        match self.api.verify(&dev.device_id, &sig_b64) {
            Ok(tok) => {
                let mut d = dev.clone();
                d.session_token = Some(tok);
                save_device(&d).ok();
                self.device = Some(d);
                true
            }
            Err(e) => { self.status = e; false }
        }
    }
}

impl eframe::App for ShadowApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        match self.screen {
            Screen::Setup => self.ui_setup(ctx),
            Screen::Main => self.ui_main(ctx),
        }
    }
}

impl ShadowApp {
    fn ui_setup(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Shadow Setup");
            ui.label("Server URL");
            ui.text_edit_singleline(&mut self.server_url_input);
            ui.label("Admin token (optional, for account creation)");
            ui.text_edit_singleline(&mut self.admin_token_input);
            ui.separator();
            ui.label("Username for new account");
            ui.text_edit_singleline(&mut self.username_input);
            if ui.button("Create account (admin)").clicked() {
                self.config.server_url = self.server_url_input.clone();
                self.config.admin_token = Some(self.admin_token_input.clone());
                save_config(&self.config).ok();
                self.api = Api::new(self.config.server_url.clone());
                let req = CreateAccountRequest {
                    username: self.username_input.clone(),
                    auth_methods: vec![AuthMethod::DeviceKey],
                    is_admin: false,
                };
                match self.api.create_account(&self.admin_token_input, &req) {
                    Ok(acc) => {
                        self.account_id_input = acc.id.clone();
                        self.status = format!("account created: {}", acc.id);
                    }
                    Err(e) => self.status = e,
                }
            }
            ui.separator();
            ui.label("Account ID to register device for");
            ui.text_edit_singleline(&mut self.account_id_input);
            if ui.button("Register this device").clicked() {
                self.config.server_url = self.server_url_input.clone();
                save_config(&self.config).ok();
                self.api = Api::new(self.config.server_url.clone());
                // generate keys
                let (mldsa_pk, mldsa_sk) = crypto::mldsa::keypair();
                let (mlkem_pk, mlkem_sk) = crypto::mlkem::keypair();
                let device_id = uuid::Uuid::new_v4().to_string();
                let msg = format!("register:{}:{}:{}:{}", self.account_id_input, device_id,
                    crypto::b64_encode(&mldsa_pk), crypto::b64_encode(&mlkem_pk));
                let sig = crypto::mldsa::sign(&mldsa_sk, msg.as_bytes()).unwrap();
                let req = RegisterDeviceRequest {
                    account_id: self.account_id_input.clone(),
                    device_id: device_id.clone(),
                    mldsa_pk: crypto::b64_encode(&mldsa_pk),
                    mlkem_pk: crypto::b64_encode(&mlkem_pk),
                    signature: crypto::b64_encode(&sig),
                };
                match self.api.register_device(&req) {
                    Ok(()) => {
                        let dk = DeviceKeys {
                            account_id: self.account_id_input.clone(),
                            device_id,
                            mldsa_sk: crypto::b64_encode(&mldsa_sk),
                            mlkem_sk: crypto::b64_encode(&mlkem_sk),
                            mldsa_pk: crypto::b64_encode(&mldsa_pk),
                            mlkem_pk: crypto::b64_encode(&mlkem_pk),
                            session_token: None,
                        };
                        save_device(&dk).ok();
                        self.device = Some(dk);
                        self.screen = Screen::Main;
                        self.status = "device registered".into();
                    }
                    Err(e) => self.status = e,
                }
            }
            ui.separator();
            ui.label("Voice peer addr");
            ui.text_edit_singleline(&mut self.peer_addr_input);
            if self.voice.is_none() {
                if ui.button("Start voice").clicked() {
                    if let Some(ch) = &self.selected_channel {
                        if let Some(key) = self.channel_keys.get(ch).and_then(|v| v.last().map(|x| x.0)) {
                            if let Ok(addr) = self.peer_addr_input.parse() {
                                match crate::voice::VoiceSession::start(addr, key) {
                                    Ok(v) => { self.voice = Some(v); self.status = "voice started".into(); }
                                    Err(e) => self.status = e,
                                }
                            } else { self.status = "bad address".into(); }
                        } else { self.status = "no channel key".into(); }
                    } else { self.status = "no channel".into(); }
                }
            } else {
                if ui.button("Stop voice").clicked() {
                    if let Some(mut v) = self.voice.take() { v.stop(); }
                    self.status = "voice stopped".into();
                }
            }
            if self.screen_session.is_none() {
                if ui.button("Start screen share").clicked() {
                    if let Some(ch) = &self.selected_channel {
                        if let Some(key) = self.channel_keys.get(ch).and_then(|v| v.last().map(|x| x.0)) {
                            if let Ok(addr) = self.peer_addr_input.parse() {
                                match crate::screen::ScreenSession::start(addr, key) {
                                    Ok(v) => { self.screen_session = Some(v); self.status = "screen started".into(); }
                                    Err(e) => self.status = e,
                                }
                            } else { self.status = "bad address".into(); }
                        } else { self.status = "no channel key".into(); }
                    } else { self.status = "no channel".into(); }
                }
            } else {
                if ui.button("Stop screen").clicked() {
                    if let Some(mut v) = self.screen_session.take() { v.stop(); }
                    self.status = "screen stopped".into();
                }
            }
            ui.separator();
            ui.collapsing("Admin", |ui| {
                ui.text_edit_singleline(&mut self.admin_token_input);
                if ui.button("List accounts").clicked() {
                    match self.api.list_accounts(&self.admin_token_input) {
                        Ok(accs) => self.status = format!("{} accounts", accs.len()),
                        Err(e) => self.status = e,
                    }
                }
                ui.text_edit_singleline(&mut self.account_id_input);
                if ui.button("Delete account").clicked() {
                    match self.api.delete_account_admin(&self.admin_token_input, &self.account_id_input) {
                        Ok(()) => self.status = "deleted".into(),
                        Err(e) => self.status = e,
                    }
                }
                if ui.button("Revoke device (by id)").clicked() {
                    match self.api.revoke_device(&self.admin_token_input, &self.account_id_input) {
                        Ok(()) => self.status = "revoked".into(),
                        Err(e) => self.status = e,
                    }
                }
            });
            ui.separator();
            ui.label(&self.status);
        });
    }

    fn ui_main(&mut self, ctx: &Context) {
        if let Ok(mut v) = self.ws_messages.lock() {
            if !v.is_empty() { self.messages.extend(v.drain(..)); }
        }
        // ensure login
        if !self.ensure_login() {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Login failed");
                ui.label(&self.status);
                if ui.button("Back to setup").clicked() {
                    clear_device();
                    self.device = None;
                    self.screen = Screen::Setup;
                }
            });
            return;
        }
        egui::SidePanel::left("channels").show(ctx, |ui| {
            ui.heading("Channels");
            if ui.button("Refresh").clicked() { self.refresh_channels(); }
            ui.text_edit_singleline(&mut self.channel_name_input);
            if ui.button("Create channel").clicked() {
                if let Some(dev) = &self.device {
                    if let Some(tok) = &dev.session_token {
                        // create a group channel including self account? members empty for now
                        let req = CreateChannelRequest {
                            name: self.channel_name_input.clone(),
                            kind: ChannelKind::Group,
                            members: vec![dev.account_id.clone()],
                        };
                        match self.api.create_channel(tok, &req) {
                            Ok(ch) => {
                                let mut key = [0u8; 32];
                                getrandom::getrandom(&mut key).ok();
                                self.channel_keys.insert(ch.id.clone(), vec![(key, chrono::Utc::now().timestamp())]);
                                let ch_id = ch.id.clone();
                                let members = req.members.clone();
                                for m in members {
                                    if m == dev.account_id { continue; }
                                    if let Ok(devs) = self.api.list_devices(tok, &m) {
                                        for d in devs {
                                            if d.revoked { continue; }
                                            if let Ok(pk) = crypto::b64_decode(&d.mlkem_pk) {
                                                if let Ok((ss, ct)) = crypto::mlkem::encapsulate(&pk) {
                                                    let mut sskey = [0u8;32];
                                                    if ss.len() >= 32 { sskey.copy_from_slice(&ss[..32]); }
                                                    let w = crypto::xchacha::encrypt(&sskey, &key, &[]).unwrap();
                                                    let msg = format!("key:{}:{}:{}", dev.device_id, d.id, ch_id);
                                                    let sk = crypto::b64_decode(&dev.mldsa_sk).unwrap();
                                                    let sig = crypto::mldsa::sign(&sk, msg.as_bytes()).unwrap();
                                                    let ann = KeyAnnouncement {
                                                        from_device: dev.device_id.clone(),
                                                        to_device: d.id.clone(),
                                                        channel_id: Some(ch_id.clone()),
                                                        kem_ciphertext: crypto::b64_encode(&ct),
                                                        nonce: crypto::b64_encode(&w.0),
                                                        wrapped_key: crypto::b64_encode(&w.1),
                                                        signature: crypto::b64_encode(&sig),
                                                    };
                                                    let _ = self.api.announce_key(tok, &ann);
                                                }
                                            }
                                        }
                                    }
                                }
                                self.channels.push(ch);
                                self.status = "channel created".into();
                            }
                            Err(e) => self.status = e,
                        }
                    }
                }
            }
            // list
            let mut sel = self.selected_channel.clone();
            for ch in &self.channels {
                if ui.selectable_label(sel.as_deref() == Some(&ch.id), &ch.name).clicked() {
                    sel = Some(ch.id.clone());
                }
            }
            if sel != self.selected_channel {
                self.selected_channel = sel.clone();
                // load messages
                if let (Some(ch), Some(dev)) = (&self.selected_channel, &self.device) {
                    if let Some(tok) = &dev.session_token {
                        // fetch key announcements first
                        if let Ok(anns) = self.api.fetch_key_announcements(tok) {
                            for ann in anns {
                                if ann.channel_id.as_deref() == Some(ch) {
                                    let dev = self.device.as_ref().unwrap();
                                    if let Ok(sender_dev) = self.api.get_device(tok, &ann.from_device) {
                                        if let Ok(pk) = crypto::b64_decode(&sender_dev.mldsa_pk) {
                                            let msg = format!("key:{}:{}:{}", ann.from_device, ann.to_device, ch);
                                            if let Ok(sig) = crypto::b64_decode(&ann.signature) {
                                                if !crypto::mldsa::verify(&pk, msg.as_bytes(), &sig) { continue; }
                                            }
                                        }
                                    }
                                    let sk = crypto::b64_decode(&dev.mlkem_sk).unwrap();
                                    let ct = crypto::b64_decode(&ann.kem_ciphertext).unwrap();
                                    if let Ok(ss) = crypto::mlkem::decapsulate(&sk, &ct) {
                                        let mut sskey = [0u8;32];
                                        if ss.len() >= 32 { sskey.copy_from_slice(&ss[..32]); }
                                        let nonce = crypto::b64_decode(&ann.nonce).unwrap();
                                        let wk = crypto::b64_decode(&ann.wrapped_key).unwrap();
                                        if let Ok(pt) = crypto::xchacha::decrypt(&sskey, &nonce, &wk, &[]) {
                                            if pt.len() >= 32 {
                                                let mut key = [0u8;32];
                                                key.copy_from_slice(&pt[..32]);
                                                let entry = self.channel_keys.entry(ch.clone()).or_default();
                                                entry.push((key, chrono::Utc::now().timestamp()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if let Ok(msgs) = self.api.list_messages(tok, ch) {
                            self.messages = msgs;
                        }

                                            }
                }
            }
            ui.separator();
            ui.label(&self.status);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ch) = &self.selected_channel {
                ui.heading(ch.clone());
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for m in &self.messages {
                        let text = if let Some(keys) = self.channel_keys.get(ch) {
                            if let Ok(raw) = crypto::b64_decode(&m.payload) {
                                if raw.len() >= 24 {
                                    let (nonce, ct) = raw.split_at(24);
                                    let mut done = None;
                                    for (k, _) in keys.iter().rev() {
                                        if let Ok(pt) = crypto::xchacha::decrypt(k, nonce, ct, &[]) {
                                            done = Some(String::from_utf8_lossy(&pt).to_string());
                                            break;
                                        }
                                    }
                                    done.unwrap_or_else(|| "[decrypt failed]".into())
                                } else { "[bad payload]".into() }
                            } else { "[bad b64]".into() }
                        } else {
                            format!("[no key] {}", m.payload)
                        };
                        ui.label(format!("{}: {}", m.sender_device_id, text));
                    }
                });
                ui.separator();
                ui.text_edit_singleline(&mut self.message_input);
                if ui.button("Send").clicked() {
                    if let Some(dev) = &self.device {
                        if let Some(tok) = &dev.session_token {
                            if let Some(key) = self.channel_keys.get(ch).and_then(|v| v.last().map(|x| x.0)) {
                                // encrypt message_input
                                let pt = self.message_input.as_bytes();
                                if let Ok((nonce, ct)) = crypto::xchacha::encrypt(&key, pt, &[]) {
                                    let mut payload = nonce.clone();
                                    payload.extend_from_slice(&ct);
                                    let req = SendMessageRequest {
                                        channel_id: ch.clone(),
                                        payload: crypto::b64_encode(&payload),
                                    };
                                    match self.api.send_message(tok, &req) {
                                        Ok(m) => { self.messages.push(m); self.message_input.clear(); }
                                        Err(e) => self.status = e,
                                    }
                                }
                            } else {
                                self.status = "no channel key".into();
                            }
                        }
                    }
                }
            } else {
                ui.label("Select a channel");
            }
        });
    }
}
