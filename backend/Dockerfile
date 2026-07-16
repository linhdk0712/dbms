# ────────────────────────────────────────────────────────────────────────────────
# SECS Backend — Multi-stage Dockerfile
# Build: Rust stable, Production: distroless/debian for minimal attack surface
# ────────────────────────────────────────────────────────────────────────────────

# Stage 1: Build
FROM rust:1.88-bookworm AS builder

WORKDIR /app

# Cài thêm dependencies cho sqlx (OpenSSL, pkg-config)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies — copy Cargo files trước
COPY Cargo.toml Cargo.lock ./

# Tạo dummy src để build dependencies trước (cache layer)
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src target/release/deps/secs_backend* target/release/secs-backend*

# Copy source code thực
COPY src/ src/
COPY migrations/ migrations/

# Build production binary — force rebuild với source thật
# SQLX_OFFLINE=true để không cần DB lúc compile (dùng .sqlx cache nếu có)
ENV SQLX_OFFLINE=true
RUN touch src/main.rs && cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime

# Cài runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user cho security
RUN groupadd -r secs && useradd -r -g secs -s /usr/sbin/nologin secs
USER secs

WORKDIR /app

# Copy binary từ builder
COPY --from=builder /app/target/release/secs-backend /app/secs-backend

# Copy migrations (chạy lúc startup)
COPY --from=builder /app/migrations /app/migrations

# Health check
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/app/secs-backend", "--health-check"] || exit 1

EXPOSE 3000

ENTRYPOINT ["/app/secs-backend"]
