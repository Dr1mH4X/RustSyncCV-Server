# RustSyncCV-Server

RustSyncCV-Server 是基于 Rust、Axum 和 WebSocket 构建的剪贴板同步后端服务。

## 功能

- 用户认证：通过用户名/密码（TOML 文件）验证，生成 JWT 令牌
- WebSocket 通信：客户端连接后，通过 WebSocket 发送剪贴板更新
- 剪贴板广播：将某用户在一个设备上的剪贴板更新广播给该用户的其他设备
- 状态管理：服务端保存每位用户的最新剪贴板状态
- TLS 支持：当提供有效的 `.pem` 和 `.key` 文件时，使用 HTTPS/WSS；否则降级为 HTTP/WS
- 调试日志：记录连接、认证、消息解析、剪贴板预览和广播等信息

## 配置

在项目根目录放置 `config.toml`：

```toml
address = "0.0.0.0"
port = 8080
tls_cert = "certs/server.pem"
tls_key = "certs/server.key"
```

在项目根目录放置 `users.toml`，格式如下：

```toml
[john]
password = "password123"

test = { password = "test" }
```

## 快速开始

```powershell
# 克隆仓库
git clone <repo-url>
cd RustSyncCV-Server

# 构建并运行（如果没有证书，则使用 HTTP）
cargo run
```

默认监听 `config.toml` 中配置的地址和端口。

## 反向代理

在生产环境中，通常需要通过反向代理（如 Nginx）将外部 WebSocket 请求转发到内部服务。以下是 Nginx 示例配置：

```nginx
server {
    listen 80;
    server_name example.com;

    location /ws/ {
        proxy_pass         http://127.0.0.1:8067;
        proxy_http_version 1.1;
        proxy_set_header   Upgrade $http_upgrade;
        proxy_set_header   Connection "Upgrade";
        proxy_set_header   Host $host;
        # 可选：WebSocket 心跳和超时配置
        proxy_read_timeout 60s;
        proxy_send_timeout 60s;
    }
}
```

修改 `server_name`、`proxy_pass` 等字段以匹配实际环境。

## 客户端示例

使用 `wscat`（或类似工具）测试：

```bash
wscat -c ws://localhost:8080/ws
# 发送认证
{"username":"test","password":"test"}
# 等待成功消息
# 发送剪贴板更新
{"type":"clipboard_update","payload":{"content_type":"text","data":"Hello from CLI","sender_device_id":"device1"}}
```
更多客户端实现示例请参考： [RustSyncCV-Client](https://github.com/Dr1mH4X/RustSyncCV-Client)

## 开发与部署

- 依赖管理：Cargo
- 环境：Rust >= 1.60

```powershell
# 运行测试
cargo test
```

## 许可证

MIT

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Dr1mH4X/RustSyncCV-Client,Dr1mH4X/RustSyncCV-Server&type=Date)](https://www.star-history.com/#Dr1mH4X/RustSyncCV-Client&Dr1mH4X/RustSyncCV-Server&Date)
