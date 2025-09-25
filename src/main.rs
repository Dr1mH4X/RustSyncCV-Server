use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use axum::{extract::ws::{WebSocket, WebSocketUpgrade, Message}, extract::State, routing::get, Router};
use axum::response::IntoResponse;
use config::{Config, File};
use serde::Deserialize;
use tracing_subscriber;

#[derive(Debug, Deserialize)]
struct Settings {
    address: String,
    port: u16,
    tls_cert: String,
    tls_key: String,
}

mod auth;
mod connection;
mod state;
mod protocol;

use auth::{UserDB, verify_user, create_jwt};
use connection::{ConnectionMap, ConnectionMapExt};
use state::StateMap;
use protocol::{AuthRequest, AuthResponse, ClipboardUpdate};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // 读取配置文件
    let settings = Config::builder()
        .add_source(File::with_name("config.toml"))
        .build()
        .unwrap()
        .try_deserialize::<Settings>()
        .unwrap();

    // 用户数据（TOML路径可自定义）
    let user_db = Arc::new(UserDB::from_toml("users.toml"));
    let connections = ConnectionMap::default();
    let state_map = StateMap::default();

    // 全局状态
    let app_state = AppState {
        user_db: user_db.clone(),
        connections: connections.clone(),
        state_map: state_map.clone(),
        jwt_secret: "secret_key".to_string(), // 可从配置读取
    };
    let app_state = Arc::new(app_state);

    // 构建 axum 路由
    let app: Router<()> = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state.clone());

    // TLS配置（如果证书加载失败或不存在，则回退到HTTP），并输出详细日志
    let addr = format!("{}:{}", settings.address, settings.port);
    let cert_path = settings.tls_cert.clone();
    let key_path = settings.tls_key.clone();
    let cert_exists = std::path::Path::new(&cert_path).exists();
    let key_exists = std::path::Path::new(&key_path).exists();

    if cert_exists && key_exists {
        tracing::info!(cert = %cert_path, key = %key_path, "TLS: trying to load cert and key");
        match axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path).await {
            Ok(tls_config) => {
                tracing::info!("TLS enabled on {}", addr);
                axum_server::bind_rustls(addr.parse().unwrap(), tls_config)
                    .serve(app.into_make_service())
                    .await
                    .unwrap();
                return;
            }
            Err(e) => {
                tracing::error!(error = %e, cert = %cert_path, key = %key_path, "TLS load failed; falling back to HTTP");
            }
        }
    } else {
        if !cert_exists {
            tracing::warn!(cert = %cert_path, "TLS certificate file not found");
        }
        if !key_exists {
            tracing::warn!(key = %key_path, "TLS private key file not found");
        }
        tracing::warn!("TLS certificates not found, starting HTTP server on {}", addr);
    }

    axum_server::bind(addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

struct AppState {
    user_db: Arc<UserDB>,
    connections: ConnectionMap,
    state_map: StateMap,
    jwt_secret: String,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("ws connection established");

    // 首条消息必须为认证请求
    let first = match socket.recv().await {
        Some(Ok(Message::Text(t))) => t,
        Some(Ok(other)) => {
            tracing::warn!("first message not text: {:?}", other);
            let resp = AuthResponse { success: false, message: "expected text auth message".to_string(), token: None };
            let _ = socket.send(Message::Text(serde_json::to_string(&resp).unwrap())).await;
            return;
        }
        Some(Err(e)) => {
            tracing::error!("error receiving first message: {}", e);
            return;
        }
        None => {
            tracing::warn!("connection closed before first message");
            return;
        }
    };

    tracing::debug!("raw auth message: {}", first);

    let auth_req: AuthRequest = match serde_json::from_str(&first) {
        Ok(req) => req,
        Err(e) => {
            tracing::warn!("failed to parse auth json: {}", e);
            let resp = AuthResponse { success: false, message: format!("invalid auth json: {}", e), token: None };
            let _ = socket.send(Message::Text(serde_json::to_string(&resp).unwrap())).await;
            return;
        }
    };
    tracing::info!("auth attempt for user {}", auth_req.username);
    // 校验用户名密码
    if !verify_user(&state.user_db, &auth_req.username, &auth_req.password) {
        tracing::warn!("auth failed for user {}", auth_req.username);
        let resp = AuthResponse { success: false, message: "Auth failed".to_string(), token: None };
        let _ = socket.send(Message::Text(serde_json::to_string(&resp).unwrap())).await;
        return;
    }
    tracing::info!("auth successful for user {}", auth_req.username);
    // 认证成功，生成JWT
    let token = create_jwt(&auth_req.username, 3600, &state.jwt_secret).unwrap();
    let resp = AuthResponse { success: true, message: "Authentication successful".to_string(), token: Some(token.clone()) };
    let _ = socket.send(Message::Text(serde_json::to_string(&resp).unwrap())).await;

    // 创建 channel 用于广播
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    state.connections.register(&auth_req.username, &auth_req.username, tx).await;

    // 拆分 socket 为 sender/receiver
    let (mut sender, mut receiver) = socket.split();
    let username = auth_req.username.clone();
    let connections = state.connections.clone();
    let state_map = state.state_map.clone();

    // 后台任务：将 rx 的消息发送到客户端
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = sender.send(msg).await;
        }
    });

    // 主循环：接收客户端消息
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(txt)) => {
                tracing::debug!("recv text from {}: {}", username, txt);
                match serde_json::from_str::<ClipboardUpdate>(&txt) {
                    Ok(update) => {
                        let content_type = update.payload.content_type.clone();
                        // 生成内容预览：文本显示前200字符，二进制/图片显示长度
                        let preview = if content_type == "text" {
                            let txt = update.payload.data.clone();
                            if txt.len() > 200 {
                                format!("{}... (len={})", &txt[..200], txt.len())
                            } else {
                                txt
                            }
                        } else {
                            let len = update.payload.data.len();
                            format!("{} (len={})", content_type, len)
                        };

                        tracing::info!("clipboard update from {}: type={} preview={}", username, content_type, preview);
                        crate::protocol::dispatcher::dispatch_clipboard_update(
                            &username,
                            update,
                            &connections,
                            &state_map
                        ).await;
                    }
                    Err(e) => {
                        tracing::warn!("failed to parse clipboard update: {}", e);
                        let _ = receiver; // no-op to satisfy lint
                    }
                }
            }
            Ok(other) => tracing::debug!("recv non-text message: {:?}", other),
            Err(e) => tracing::error!("websocket recv error: {}", e),
        }
    }
    state.connections.unregister(&username, &username).await;
}