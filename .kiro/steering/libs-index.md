---
inclusion: auto
description: "Tổng hợp tất cả thư viện Rust dùng trong dự án. Load file cụ thể khi cần triển khai."
---

# Library Reference Index — SECS Backend

Tổng hợp tất cả thư viện Rust dùng trong dự án. Load file cụ thể khi cần triển khai.

---

## Cách sử dụng

Khi làm việc với một module cụ thể, thêm file steering tương ứng vào context bằng cách gõ `#` trong chat:

| Module cần triển khai | File cần load |
|---|---|
| API routes, handlers, extractors | `#libs-axum.md` |
| Database queries, migrations, transactions | `#libs-sqlx.md` |
| Async tasks, channels, timeout | `#libs-tokio-serde.md` |
| JSON serialize/deserialize | `#libs-tokio-serde.md` |
| JWT token, auth middleware, RBAC | `#libs-auth.md` |
| Input validation | `#libs-auth.md` |
| Custom error types, error handling | `#libs-thiserror.md` |

---

## Tóm tắt Library Stack

### HTTP Layer
- **`axum 0.7`** — Router, handlers, extractors (Path/Query/Json/State), WebSocket, middleware
- **`tower`** / **`tower-http`** — ServiceBuilder, TimeoutLayer, CorsLayer, TraceLayer

### Database
- **`sqlx 0.8`** — Async SQL, PgPool, query!, query_as!, FromRow, Transaction, Migrate

### Async Runtime
- **`tokio 1`** — `#[tokio::main]`, spawn, broadcast channel, mpsc channel, Mutex, timeout

### Serialization
- **`serde`** + **`serde_json`** — Derive Serialize/Deserialize, rename_all, skip, json! macro

### Auth
- **`jsonwebtoken 9`** — encode/decode JWT, Claims, Validation, HS256/RS256
- **`validator 0.18`** — `#[derive(Validate)]`, email/length/range/custom validation

### Error Handling
- **`thiserror 2`** — `#[derive(Error)]`, `#[from]`, `#[error(...)]`, `IntoResponse`
- **`anyhow 1`** — Application-level errors, Context, bail!, ensure!

---

## Cargo.toml đầy đủ

```toml
[dependencies]
# HTTP
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "timeout"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# Database
sqlx = { version = "0.8", features = [
    "postgres",
    "mysql",
    "runtime-tokio-rustls",
    "migrate",
    "chrono",
    "uuid",
] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Auth
jsonwebtoken = "9"
validator = { version = "0.18", features = ["derive"] }

# Error handling
thiserror = "2"
anyhow = "1"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Config
dotenvy = "0.15"

# Encryption (cho credentials database đích)
aes-gcm = "0.10"
base64 = "0.22"

# OAuth2/OIDC
openidconnect = "3"
oauth2 = "4"
```

---

## Cấu trúc Error Flow

```
Handler (api/)
    ↓ trả Result<Json<T>, AppError>
AppError (error.rs)
    ↓ impl IntoResponse → HTTP response
    ↑ convert từ:
        - sqlx::Error      (#[from])
        - jsonwebtoken::Error (#[from])
        - ExecutorError    (From impl thủ công)
        - anyhow::Error    (#[from] transparent)
```

## Cấu trúc Database Flow

```
Handler (api/)
    → db/ layer (PgPool — PostgreSQL nội bộ)
        → sqlx queries (query!, query_as!, transaction)
    → executor/ layer (DatabaseExecutor trait)
        → postgres.rs / mysql.rs / oracle.rs / mongodb.rs
        (kết nối đến database đích của khách hàng)
```
