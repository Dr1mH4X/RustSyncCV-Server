use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// 用户ID -> 最新剪贴板内容
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ClipboardState {
    pub content_type: String,
    pub data: String,
    pub sender_device_id: String,
}

pub type StateMap = Arc<Mutex<HashMap<String, ClipboardState>>>;

pub trait StateMapExt {
    async fn update(&self, user_id: &str, state: ClipboardState);
    async fn get_state(&self, user_id: &str) -> Option<ClipboardState>;
}

impl StateMapExt for StateMap {
    async fn update(&self, user_id: &str, state: ClipboardState) {
        let mut map = self.lock().await;
        map.insert(user_id.to_string(), state);
    }
    async fn get_state(&self, user_id: &str) -> Option<ClipboardState> {
        let map = self.lock().await;
        map.get(user_id).cloned()
    }
}
