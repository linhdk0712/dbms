# Business Logic Model — Unit 1: Foundation

## Tổng quan

Unit 1 cung cấp nền tảng cho toàn bộ hệ thống:
- Database schema (migrations)
- Domain models (Rust structs)
- Application infrastructure (config, error handling, server startup)
- Health check endpoint

Unit này không chứa business workflow — chỉ cung cấp building blocks cho Unit 2, 3, 4.

---

## 1. Config Loading

```
Config::from_env() {
    DATABASE_URL          (required)
    JWT_SECRET            (required, min 32 chars)
    ENCRYPTION_KEY        (required, 32-byte hex)
    OIDC_ISSUER_URL       (required)
    OIDC_CLIENT_ID        (required)
    OIDC_CLIENT_SECRET    (required)
    FRONTEND_URL          (required — cho CORS)
    SQL_EXECUTION_TIMEOUT_SECS (optional, default: 30)
    RUST_LOG              (optional, default: "info")
}
```

Fail fast nếu thiếu bất kỳ required config nào — không khởi động server với config không đầy đủ.

---

## 2. Error Handling Model

```
AppError (thiserror) {
    // 400
    FourEyesViolation
    InvalidStateTransition { from: String, to: String }
    ImmutableRequest
    ValidationError(String)

    // 403
    DatabasePermissionDenied { db_id: Uuid }
    Forbidden

    // 401
    Unauthorized

    // 404
    NotFound

    // 500 (không expose detail ra client)
    Database(#[from] sqlx::Error)
    Jwt(#[from] jsonwebtoken::errors::Error)
    Internal(#[from] anyhow::Error)
}
```

`IntoResponse` impl: 5xx → log với `tracing::error!` + trả generic message. 4xx → trả message cụ thể.

---

## 3. Database Migration Sequence

```
001_create_users.sql
    → bảng users (id, oidc_subject, email, display_name, is_active, timestamps)

002_create_database_connections.sql
    → bảng database_connections (id, name, db_type, host, port, ..., encrypted_credentials)

003_create_sql_requests.sql
    → bảng sql_requests (id, description, sql_content, status, database_id, created_by, approved_by, ...)
    → CHECK CONSTRAINT: approved_by IS NULL OR approved_by != created_by

004_create_audit_log.sql
    → bảng audit_log (id, request_id, action, actor_id, detail, execution_result, ip_address, timestamp)
    → KHÔNG có UPDATE/DELETE cho app user

005_create_user_db_permissions.sql
    → bảng user_db_permissions (id, user_id, database_id, granted_by, granted_at)
    → UNIQUE(user_id, database_id)

006_create_indexes.sql
    → idx_sql_requests_status
    → idx_sql_requests_created_by
    → idx_sql_requests_database_id
    → idx_audit_log_request_id
    → idx_audit_log_actor_id
    → idx_audit_log_timestamp
    → idx_user_db_permissions_user_id
```

---

## 4. AppState Structure

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub notification_tx: broadcast::Sender<Notification>,
    // OIDC client được inject vào state khi startup — Unit 2 sẽ add
}
```

---

## 5. Server Startup Sequence

```
1. Init tracing (JSON structured logging)
2. Config::from_env() → fail fast nếu thiếu
3. PgPool connect với max 20 connections
4. sqlx migrate run → fail fast nếu migration lỗi
5. broadcast::channel(128) cho notifications
6. Build AppState
7. Build axum Router với middleware stack:
   - TraceLayer (request logging với request_id)
   - CorsLayer (chỉ allow FRONTEND_URL)
   - DefaultBodyLimit (100KB)
   - Security headers middleware (SECURITY-04)
8. TcpListener bind 0.0.0.0:3000
9. axum::serve
```

---

## 6. HTTP Security Headers Middleware (SECURITY-04)

Tower middleware inject vào tất cả responses:
```
Content-Security-Policy: default-src 'self'
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: strict-origin-when-cross-origin
```

---

## 7. Health Check Endpoint

```
GET /health
Response 200: { "status": "ok", "db": "connected" }
Response 503: { "status": "error", "db": "disconnected" }
```

Logic: Thực hiện `SELECT 1` để verify DB pool còn sống.
