# RustSyncCV-Server

RustSyncCV-Server 是基于 Rust、Axum 和 WebSocket 构建的剪贴板同步后端服务。

## 功能

- **用户认证**：通过用户名/密码（TOML 文件）进行认证。
- **WebSocket 通信**：客户端通过 WebSocket 实时同步剪贴板。
- **HTTP API**：提供 RESTful 接口，方便 iOS 快捷指令等非 WebSocket 客户端集成。
- **剪贴板广播**：将某用户在一个设备上的剪贴板更新广播给该用户的其他设备。
- **状态管理**：服务端保存每位用户的最新剪贴板状态。
- **多用户支持**：支持多个用户账户，每个用户的剪贴板独立同步。
- **调试日志**：通过 `tracing` 记录详细的运行时信息。

## 配置

在项目根目录放置 `config.toml`：

```toml
address = "0.0.0.0"
port = 8067
```

在项目根目录放置 `users.toml`，格式如下：

```toml
[[users]]
username = "testuser"
password = "testpass"
```

## 快速开始

```powershell
# 克隆仓库
git clone <repo-url>
cd RustSyncCV-Server

# 构建并运行
cargo run
```

服务器将默认监听 `config.toml` 中配置的地址和端口。

## HTTP API (for iOS Shortcuts, etc.)

为了方便像 iOS 快捷指令这样不支持 WebSocket 的客户端，服务器额外提供了简单的 HTTP 接口。这些接口通过在请求体中直接包含用户名和密码进行认证。

### 获取剪贴板

**`POST /api/clipboard/get`**

获取指定用户的最新剪贴板内容。

#### 请求体 (JSON)
```json
{
    "username": "your_username",
    "password": "your_password"
}
```

#### 成功响应 (200 OK)
```json
{
    "content_type": "text",
    "data": "这是剪贴板的内容",
    "sender_device_id": "some_device"
}
```

#### 示例 (curl)
```bash
curl -X POST http://localhost:8067/api/clipboard/get \
-H "Content-Type: application/json" \
-d '{"username": "testuser", "password": "testpass"}'
```

### 更新剪贴板

**`POST /api/clipboard/set`**

更新指定用户的剪贴板内容，并向该用户的所有 WebSocket 连接广播此更新。

#### 请求体 (JSON)
```json
{
    "username": "your_username",
    "password": "your_password",
    "payload": {
        "content_type": "text",
        "data": "新的剪贴板内容",
        "sender_device_id": "iPhoneShortcut"
    }
}
```

#### 成功响应 (200 OK)
响应体为空。

#### 示例 (curl)
```bash
curl -X POST http://localhost:8067/api/clipboard/set \
-H "Content-Type: application/json" \
-d '{"username": "testuser", "password": "testpass", "payload": {"content_type": "text", "data": "Hello from curl", "sender_device_id": "curl_client"}}'
```

## WebSocket API

WebSocket 是主要的实时通信方式。

### 连接地址
`ws://<server_address>:<port>/ws`

### 通信流程
1.  客户端连接后，必须发送第一条文本消息作为认证请求。
2.  认证成功后，服务器会返回成功消息和 JWT。
3.  之后，客户端可以发送或接收 `ClipboardUpdate` 消息。

#### 认证请求
```json
{"username":"testuser","password":"testpass"}
```

#### 剪贴板更新
```json
{"type":"clipboard_update","payload":{"content_type":"text","data":"Hello from WebSocket","sender_device_id":"device1"}}
```

## 反向代理

在生产环境中，推荐使用反向代理（如 Nginx）来暴露服务并处理 HTTPS。

```nginx
server {
    listen 443 ssl;
    server_name your.domain.com;

    # SSL/TLS 配置
    ssl_certificate /path/to/your/fullchain.pem;
    ssl_certificate_key /path/to/your/privkey.pem;
    # ... 其他 SSL 配置

    location / {
        # 如果需要一个简单的健康检查或欢迎页面
        return 200 'RustSyncCV-Server is running';
        add_header Content-Type text/plain;
    }

    location /ws {
        proxy_pass         http://127.0.0.1:8067; # 指向 Rust 服务的地址
        proxy_http_version 1.1;
        proxy_set_header   Upgrade $http_upgrade;
        proxy_set_header   Connection "Upgrade";
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
    }

    location /api {
        proxy_pass         http://127.0.0.1:8067; # 指向 Rust 服务的地址
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

## 更新日志

### v0.1.1
- **新增**: 为 iOS 快捷指令等客户端添加了 HTTP API (`/api/clipboard/get`, `/api/clipboard/set`)。
- **移除**: 删除了所有 TLS/WSS 相关代码，简化部署，推荐在生产环境中使用反向代理处理 HTTPS。

### v0.1.0
- 初始版本发布。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Dr1mH4X/RustSyncCV-Client,Dr1mH4X/RustSyncCV-Server&type=Date)](https://www.star-history.com/#Dr1mH4X/RustSyncCV-Client&Dr1mH4X/RustSyncCV-Server&Date)
