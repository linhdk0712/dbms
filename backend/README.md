# SECS Backend

Backend server cho SQL Execution Control System — xây dựng bằng Rust với Axum framework.

## Tech Stack

| Thành phần | Công nghệ |
|-----------|-----------|
| Language | Rust (stable) |
| HTTP Framework | Axum 0.7 |
| Async Runtime | Tokio |
| Database | PostgreSQL (via SQLx 0.8) |
| Auth | OAuth2/OIDC + JWT |
| Encryption | AES-256-GCM |
| Target DBs | PostgreSQL, MySQL, MongoDB, Oracle (optional) |

## Cấu trúc thư mục

```
src/
├── main.rs              # Entry point, khởi tạo server
├── config.rs            # Load cấu hình từ env
├── error.rs             # Custom error types (thiserror)
├── state.rs             # Application state
├── api/                 # HTTP handlers (axum routes)
│   ├── auth.rs          # SSO callback, login, logout
│   ├── requests.rs      # CRUD cho SQL requests
│   ├── approvals.rs     # Review, approve, reject
│   ├── databases.rs     # Quản lý database connections
│   ├── users.rs         # Quản lý users và roles
│   ├── dashboard.rs     # Dữ liệu tổng hợp
│   ├── audit.rs         # Audit log endpoints
│   └── health.rs        # Health check
├── auth/                # Authentication & Authorization
│   ├── oidc.rs          # OpenID Connect flow
│   ├── jwt.rs           # JWT tạo và xác thực
│   ├── middleware.rs    # Auth middleware
│   ├── extractors.rs    # Axum extractors
│   └── rate_limit.rs    # Rate limiting
├── db/                  # Database access layer (PostgreSQL nội bộ)
│   ├── users.rs         # User queries
│   ├── requests.rs      # SQL request queries
│   ├── databases.rs     # Database connection queries
│   ├── permissions.rs   # Permission queries
│   └── audit.rs         # Audit log queries
├── executor/            # Thực thi SQL trên database đích
│   ├── mod.rs           # DatabaseExecutor trait
│   ├── postgres.rs      # PostgreSQL driver
│   ├── mysql.rs         # MySQL driver
│   ├── mongodb.rs       # MongoDB driver
│   ├── oracle.rs        # Oracle driver (optional)
│   ├── encryption.rs    # Credential encryption
│   └── sql_analyzer.rs  # SQL statement analysis
├── models/              # Domain types
│   ├── user.rs          # User, Role
│   ├── request.rs       # SqlRequest, RequestStatus
│   ├── approval.rs      # Approval logic
│   ├── database.rs      # DatabaseConnection config
│   └── notification.rs  # Notification types
└── ws/                  # WebSocket handlers
    └── mod.rs           # Realtime notifications
```

## Yêu cầu

- Rust stable (1.88+)
- PostgreSQL 16+
- (Optional) Oracle Instant Client — nếu cần hỗ trợ Oracle database

## Cài đặt & Chạy

### 1. Cấu hình environment

```bash
cp ../.env.example .env
# Chỉnh sửa .env với thông tin phù hợp
```

Các biến môi trường quan trọng:

| Biến | Mô tả |
|------|--------|
| `DATABASE_URL` | Connection string PostgreSQL nội bộ |
| `JWT_SECRET` | Secret key cho JWT (min 32 chars) |
| `ENCRYPTION_KEY` | AES-256 key cho encrypt credentials (64 hex chars) |
| `OIDC_ISSUER_URL` | URL của OIDC provider (Keycloak) |
| `OIDC_CLIENT_ID` | Client ID đăng ký trên OIDC provider |
| `OIDC_CLIENT_SECRET` | Client secret |
| `OIDC_REDIRECT_URI` | Redirect URI sau khi login |
| `FRONTEND_URL` | URL frontend (cho CORS) |
| `SQL_EXECUTION_TIMEOUT_SECS` | Timeout thực thi SQL (default: 30s) |
| `RUST_LOG` | Log level (default: `info,secs_backend=debug`) |
| `LOG_DIR` | Thư mục lưu file log rotation (default: `./logs`) |
| `PORT` | Server port (default: 3000) |

### 2. Chạy database migration

```bash
sqlx migrate run
```

### 3. Build & Run

```bash
# Development
cargo run

# Production
cargo build --release
./target/release/secs-backend
```

### 4. Build với Oracle support

```bash
cargo build --release --features oracle-support
```

## Commands

```bash
cargo build                  # Compile debug
cargo build --release        # Compile production
cargo run                    # Chạy server (dev)
cargo test                   # Chạy tests
cargo clippy -- -D warnings  # Lint (bắt buộc pass)
cargo fmt                    # Format code
sqlx migrate run             # Chạy migrations
sqlx migrate add <name>      # Tạo migration mới
```

## Docker

```bash
docker build -t secs-backend .
docker run -p 3000:3000 --env-file .env secs-backend
```

## API Endpoints

| Method | Path | Mô tả |
|--------|------|--------|
| GET | `/health` | Health check |
| GET | `/auth/login` | Redirect đến OIDC login |
| GET | `/auth/callback` | OIDC callback |
| POST | `/auth/logout` | Logout |
| GET | `/api/users` | Danh sách users (Admin) |
| GET | `/api/requests` | Danh sách SQL requests |
| POST | `/api/requests` | Tạo SQL request mới |
| POST | `/api/requests/:id/submit` | Submit request để review |
| POST | `/api/approvals/:id/approve` | Approve request |
| POST | `/api/approvals/:id/reject` | Reject request |
| GET | `/api/databases` | Danh sách database connections |
| GET | `/api/dashboard` | Dashboard data |
| GET | `/api/audit` | Audit log |
| GET | `/ws` | WebSocket connection |

## Quy ước code

- Không dùng `unwrap()` trong production — dùng `?` hoặc xử lý explicit
- `cargo clippy -- -D warnings` phải pass trước khi commit
- `cargo fmt` trước mỗi commit
- Dùng `query!` / `query_as!` macro cho compile-time type checking
- Parameterized queries — không bao giờ string-concat SQL với user input
