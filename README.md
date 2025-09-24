# RustSyncCV-Server

RustSyncCV-Server 是基于 Rust、Axum 和 WebSocket 构建的剪贴板同步后端服务。

## 功能

- 用户认证：通过用户名/密码（TOML 文件）验证，生成 JWT 令牌
- WebSocket 通信：客户端连接后，通过 WebSocket 发送剪贴板更新
- 剪贴板广播：将某用户在一个设备上的剪贴板更新广播给该用户的其他设备
- 状态管理：服务端保存每位用户的最新剪贴板状态
- TLS 支持：当提供有效的 `.pem` 和 `.key` 文件时，使用 HTTPS/WSS；否则降级为 HTTP/WS
- 多用户支持：支持多个用户账户，每个用户的剪贴板独立同步，用户之间数据互不干扰
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
[[users]]
username = "testuser"
password = "testpass"
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

本项目基于 MIT 许可证，详情见 [LICENSE](LICENSE)。

## Docker 构建与运行

项目提供多阶段 Docker 构建，运行镜像体积精简。

### 1. 构建镜像

```powershell
docker build -t rustsynccv-server:latest .
```

### 2. 运行容器

```powershell
# 默认使用镜像内置 config.toml / users.toml
docker run --rm -p 8080:8080 rustsynccv-server:latest

# 挂载本地配置（推荐）
docker run --rm -p 8080:8080 `
    -v ${PWD}/config.toml:/app/config.toml `
    -v ${PWD}/users.toml:/app/users.toml `
    rustsynccv-server:latest
```

### 3. 推送到镜像仓库（Docker Hub 示例）

```powershell
$env:IMAGE_NAME="yourname/rustsynccv-server"
docker tag rustsynccv-server:latest $env:IMAGE_NAME:latest
docker login
docker push $env:IMAGE_NAME:latest
```

### 4. 多架构构建 (amd64 + arm64)

```powershell
docker buildx create --use --name multi || docker buildx use multi
docker buildx build --platform linux/amd64,linux/arm64 `
    -t yourname/rustsynccv-server:latest `
    --push .
```

### 5. 使用自定义证书启用 WSS

假设你有 `certs/server.pem` 与 `certs/server.key`，并在 `config.toml` 中配置：

```toml
tls_cert = "certs/server.pem"
tls_key = "certs/server.key"
```

运行时挂载：

```powershell
docker run --rm -p 8080:8080 `
    -v ${PWD}/config.toml:/app/config.toml `
    -v ${PWD}/users.toml:/app/users.toml `
    -v ${PWD}/certs:/app/certs `
    rustsynccv-server:latest
```

### 6. 调整日志级别

```powershell
docker run --rm -e RUST_LOG=debug -p 8080:8080 rustsynccv-server:latest
```

### 7. 常见问题

- 用户列表更新：更新 `users.toml` 后重启容器即可，无需重建镜像。
- config 修改：修改端口或证书路径后需同步更新容器挂载。
- 无证书运行：若证书文件不存在，自动回退到 HTTP/WS。
- 健康检查：可添加 `--health-cmd` 自定义 HTTP/TCP 探测。

### 8. 配置与数据持久化示例

#### 方式一：主机路径挂载（适合本地开发）

```powershell
# 假设当前目录包含 config.toml 与 users.toml
docker run --name rustsynccv `
    -p 8080:8080 `
    -v ${PWD}/config.toml:/app/config.toml `
    -v ${PWD}/users.toml:/app/users.toml `
    -v ${PWD}/certs:/app/certs `
    rustsynccv-server:latest
```

#### 方式二：命名卷（适合长期运行）

先初始化卷并复制默认文件：
```powershell
docker volume create rustsynccv_config
docker run --rm -v rustsynccv_config:/data busybox sh -c "mkdir -p /data && echo 'address = \"0.0.0.0\"\nport = 8080\ntls_cert = \"certs/server.pem\"\ntls_key = \"certs/server.key\"' > /data/config.toml && echo '[[users]]\nusername = \"test\"\npassword = \"test\"' > /data/users.toml"
```

运行容器并挂载卷：
```powershell
docker run -d --name rustsynccv `
    -p 8080:8080 `
    -v rustsynccv_config:/app `
    rustsynccv-server:latest
```

你可以进入卷修改配置：
```powershell
docker run --rm -it -v rustsynccv_config:/data busybox sh
# vi /data/users.toml 或 sed 编辑
```

#### 方式三：Kubernetes ConfigMap / Secret（拓展）

在 K8s 场景中，可将 `config.toml` 作为 ConfigMap，证书作为 Secret 挂载到 `/app` 路径，保持与容器内目录结构一致。

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=Dr1mH4X/RustSyncCV-Client,Dr1mH4X/RustSyncCV-Server&type=Date)](https://www.star-history.com/#Dr1mH4X/RustSyncCV-Client&Dr1mH4X/RustSyncCV-Server&Date)
