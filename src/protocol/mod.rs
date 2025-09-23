pub mod dispatcher;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipboardUpdatePayload {
    pub content_type: String, // "text" or "image_png"
    pub data: String,         // 文本或base64图片
    pub sender_device_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipboardUpdate {
    pub r#type: String,
    pub payload: ClipboardUpdatePayload,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipboardBroadcast {
    pub content_type: String,
    pub data: String,
}
