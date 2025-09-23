use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::extract::ws::Message;
use tokio::sync::mpsc::UnboundedSender;

// 用户ID -> 设备ID -> Sender
pub type ConnectionMap = Arc<Mutex<HashMap<String, HashMap<String, UnboundedSender<Message>>>>>;

pub trait ConnectionMapExt {
    async fn register(&self, user_id: &str, device_id: &str, tx: UnboundedSender<Message>);
    async fn unregister(&self, user_id: &str, device_id: &str);
    async fn get_other_devices(&self, user_id: &str, exclude_device_id: &str) -> Vec<UnboundedSender<Message>>;
}

impl ConnectionMapExt for ConnectionMap {
    async fn register(&self, user_id: &str, device_id: &str, tx: UnboundedSender<Message>) {
        let mut map = self.lock().await;
        map.entry(user_id.to_string())
            .or_insert_with(HashMap::new)
            .insert(device_id.to_string(), tx);
    }
    async fn unregister(&self, user_id: &str, device_id: &str) {
        let mut map = self.lock().await;
        if let Some(devices) = map.get_mut(user_id) {
            devices.remove(device_id);
            if devices.is_empty() {
                map.remove(user_id);
            }
        }
    }
    async fn get_other_devices(&self, user_id: &str, exclude_device_id: &str) -> Vec<UnboundedSender<Message>> {
        let map = self.lock().await;
        map.get(user_id)
            .map(|devices| {
                devices.iter()
                    .filter(|(id, _)| *id != exclude_device_id)
                    .map(|(_, tx)| tx.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}
