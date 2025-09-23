use crate::protocol::{ClipboardUpdate, ClipboardBroadcast};
use crate::connection::{ConnectionMap, ConnectionMapExt};
use crate::state::{StateMap, ClipboardState, StateMapExt};
use axum::extract::ws::Message;

pub async fn dispatch_clipboard_update(
    user_id: &str,
    update: ClipboardUpdate,
    connections: &ConnectionMap,
    state: &StateMap,
) {
    let payload = update.payload;
    // 更新服务端状态
    let new_state = ClipboardState {
        content_type: payload.content_type.clone(),
        data: payload.data.clone(),
        sender_device_id: payload.sender_device_id.clone(),
    };
    state.update(user_id, new_state).await;

    // 构造广播消息
    let broadcast = ClipboardBroadcast {
        content_type: payload.content_type,
        data: payload.data,
    };
    let msg = Message::Text(serde_json::to_string(&broadcast).unwrap());

    // 广播给同一用户的其他设备
    let others = connections.get_other_devices(user_id, &payload.sender_device_id).await;
    for ws in others {
        let _ = ws.send(msg.clone());
    }
}
