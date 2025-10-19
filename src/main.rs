use axum::response::{IntoResponse, Response};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use config::{Config, File};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing_subscriber;

#[derive(Debug, Deserialize)]
struct Settings {
    address: String,
    port: u16,
}

mod auth;
mod connection;
mod protocol;
mod state;

use auth::{create_jwt, verify_user, UserDB};
use connection::{ConnectionMap, ConnectionMapExt};
use protocol::{AuthRequest, AuthResponse, ClipboardUpdate, ClipboardUpdatePayload};
use state::{StateMap, StateMapExt}; // Import the extension trait

// --- 新增用于 HTTP API 的请求/响应结构体 ---

#[derive(Deserialize)]
struct ClipboardGetRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct ClipboardSetRequest {
    username: String,
    password: String,
    payload: ClipboardUpdatePayload, // Corrected from ClipboardPayload
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// -----------------------------------------

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
        // --- 为快捷指令等客户端新增的 HTTP API ---
        .route("/api/login", post(http_login_handler))
        .route("/api/clipboard/get", post(http_get_clipboard_handler))
        .route("/api/clipboard/set", post(http_set_clipboard_handler))
        // -----------------------------------------
        .with_state(app_state.clone());

    // 启动HTTP服务器
    let addr = format!("{}:{}", settings.address, settings.port);
    tracing::info!("Starting HTTP server on {}", addr);

    axum::Server::bind(&addr.parse().unwrap())
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

// --- 新增的 HTTP API 处理器 ---

/// HTTP 登录处理器 (返回 JWT, 供高级客户端使用)
async fn http_login_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuthRequest>,
) -> Response {
    if verify_user(&state.user_db, &payload.username, &payload.password) {
        let token = create_jwt(&payload.username, 3600, &state.jwt_secret).unwrap();
        let response = AuthResponse {
            success: true,
            message: "Authentication successful".to_string(),
            token: Some(token),
        };
        (StatusCode::OK, Json(response)).into_response()
    } else {
        let error_response = ErrorResponse {
            error: "Invalid username or password".to_string(),
        };
        (StatusCode::UNAUTHORIZED, Json(error_response)).into_response()
    }
}

/// HTTP 获取剪贴板处理器 (为快捷指令设计)
async fn http_get_clipboard_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ClipboardGetRequest>,
) -> Response {
    // 每次都验证用户名和密码
    if !verify_user(&state.user_db, &payload.username, &payload.password) {
        let error_response = ErrorResponse {
            error: "Invalid username or password".to_string(),
        };
        return (StatusCode::UNAUTHORIZED, Json(error_response)).into_response();
    }

    if let Some(clipboard) = state.state_map.get_state(&payload.username).await {
        (StatusCode::OK, Json(clipboard)).into_response()
    } else {
        let error_response = ErrorResponse {
            error: "No clipboard data found for user".to_string(),
        };
        (StatusCode::NOT_FOUND, Json(error_response)).into_response()
    }
}

/// HTTP 发送剪贴板处理器 (为快捷指令设计)
async fn http_set_clipboard_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ClipboardSetRequest>,
) -> Response {
    // 每次都验证用户名和密码
    if !verify_user(&state.user_db, &payload.username, &payload.password) {
        let error_response = ErrorResponse {
            error: "Invalid username or password".to_string(),
        };
        return (StatusCode::UNAUTHORIZED, Json(error_response)).into_response();
    }

    // 构造一个 `ClipboardUpdate` 用于广播和状态更新
    let update = ClipboardUpdate {
        r#type: "clipboard_update".to_string(),
        payload: payload.payload,
    };

    // 广播给此用户的其他 WebSocket 连接并更新状态
    crate::protocol::dispatcher::dispatch_clipboard_update(
        &payload.username,
        update,
        &state.connections,
        &state.state_map,
    )
    .await;

    (StatusCode::OK).into_response()
}

// --- WebSocket 处理器 (保持不变) ---

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("ws connection established");

    // 首条消息必须为认证请求
    let first = match socket.recv().await {
        Some(Ok(Message::Text(t))) => t,
        Some(Ok(other)) => {
            tracing::warn!("first message not text: {:?}", other);
            let resp = AuthResponse {
                success: false,
                message: "expected text auth message".to_string(),
                token: None,
            };
            let _ = socket
                .send(Message::Text(serde_json::to_string(&resp).unwrap()))
                .await;
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
            let resp = AuthResponse {
                success: false,
                message: format!("invalid auth json: {}", e),
                token: None,
            };
            let _ = socket
                .send(Message::Text(serde_json::to_string(&resp).unwrap()))
                .await;
            return;
        }
    };
    tracing::info!("auth attempt for user {}", auth_req.username);
    // 校验用户名密码
    if !verify_user(&state.user_db, &auth_req.username, &auth_req.password) {
        tracing::warn!("auth failed for user {}", auth_req.username);
        let resp = AuthResponse {
            success: false,
            message: "Auth failed".to_string(),
            token: None,
        };
        let _ = socket
            .send(Message::Text(serde_json::to_string(&resp).unwrap()))
            .await;
        return;
    }
    tracing::info!("auth successful for user {}", auth_req.username);
    // 认证成功，生成JWT
    let token = create_jwt(&auth_req.username, 3600, &state.jwt_secret).unwrap();
    let resp = AuthResponse {
        success: true,
        message: "Authentication successful".to_string(),
        token: Some(token.clone()),
    };
    let _ = socket
        .send(Message::Text(serde_json::to_string(&resp).unwrap()))
        .await;

    // 创建 channel 用于广播
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    state
        .connections
        .register(&auth_req.username, &auth_req.username, tx)
        .await;

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

                        tracing::info!(
                            "clipboard update from {}: type={} preview={}",
                            username,
                            content_type,
                            preview
                        );
                        crate::protocol::dispatcher::dispatch_clipboard_update(
                            &username,
                            update,
                            &connections,
                            &state_map,
                        )
                        .await;
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
