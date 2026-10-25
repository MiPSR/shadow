use serde::{Deserialize, Serialize};

pub type Id = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthMethod {
    DeviceKey,
    Passkey,
    DiscordOAuth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Id,
    pub username: String,
    pub auth_methods: Vec<AuthMethod>,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: Id,
    pub account_id: Id,
    pub mldsa_pk: String, // base64
    pub mlkem_pk: String, // base64
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelKind {
    Direct,
    Group,
    Text,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Id,
    pub name: String,
    pub kind: ChannelKind,
    pub members: Vec<Id>, // account ids
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMessage {
    pub id: Id,
    pub channel_id: Id,
    pub sender_device_id: Id,
    pub created_at: i64,
    pub payload: String, // base64 ciphertext (includes nonce prefix maybe)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub id: Id,
    pub message_id: Id,
    pub size: u64,
    pub media_type: String,
}

// API requests/responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountRequest {
    pub username: String,
    pub auth_methods: Vec<AuthMethod>,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountResponse {
    pub account: Account,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDeviceRequest {
    pub account_id: Id,
    pub device_id: Id,
    pub mldsa_pk: String,
    pub mlkem_pk: String,
    pub signature: String, // device signs registration payload with mldsa
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeRequest {
    pub device_id: Id,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub nonce: String, // base64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub device_id: Id,
    pub signature: String, // sign nonce
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub session_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    pub kind: ChannelKind,
    pub members: Vec<Id>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub channel_id: Id,
    pub payload: String, // base64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyAnnouncement {
    pub from_device: Id,
    pub to_device: Id,
    pub channel_id: Option<Id>,
    pub kem_ciphertext: String, // base64
    pub nonce: String, // base64
    pub wrapped_key: String, // base64
    pub signature: String, // mldsa signature over announcement
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsClientMessage {
    Auth { token: String },
    SendMessage { channel_id: Id, payload: String },
    KeyAnnounce { announcement: KeyAnnouncement },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WsServerMessage {
    Message(EncryptedMessage),
    KeyAnnounce(KeyAnnouncement),
    Error { message: String },
    Pong,
}
