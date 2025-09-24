# =========================
# 1. Build Stage
# =========================
FROM rust:1.80 as builder
WORKDIR /app

# Create a dummy project to cache dependencies
RUN USER=root cargo new rustsynccv-server
WORKDIR /app/rustsynccv-server

# Copy manifest files first to leverage Docker layer caching
COPY Cargo.toml Cargo.lock ./
# Pre-build dependencies (will fail at linking main, but caches deps)
RUN cargo build --release || true

# Now copy full source
COPY src ./src
COPY config.toml .
COPY users.toml .

# Build release binary
RUN cargo build --release

# =========================
# 2. Runtime Stage
# =========================
FROM debian:stable-slim AS runtime
WORKDIR /app

# Install minimal CA certs (if TLS used)
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /app/rustsynccv-server/target/release/RustSyncCV-Server /app/RustSyncCV-Server
# Copy default config (can override via volume)
COPY --from=builder /app/rustsynccv-server/config.toml /app/config.toml
COPY --from=builder /app/rustsynccv-server/users.toml /app/users.toml

# Non-root user
RUN useradd -m appuser
USER appuser

EXPOSE 8080
ENV RUST_LOG=info

# Healthcheck (simple TCP check)
HEALTHCHECK --interval=30s --timeout=3s --retries=3 CMD ["/bin/sh", "-c", "nc -z 127.0.0.1 8080 || exit 1"]

CMD ["/app/RustSyncCV-Server"]
