# Tech Stack

## Backend

- **Ngôn ngữ**: Rust (stable toolchain)
- **Build tool**: Cargo
- **HTTP Framework**: `axum` — async, type-safe, tích hợp tốt với tower middleware
- **Async runtime**: `tokio`
- **Serialization**: `serde` + `serde_json`
- **Validation**: `validator`

### Database hệ thống (lưu trữ nội bộ)
- **PostgreSQL** — lưu users, SQL requests, audit log, database connections config
- **ORM/Query**: `sqlx` với compile-time query checking
- **Migrations**: 8 files (001-008), bao gồm multi-level approval và admin audit actions

### Kết nối Database đích (target databases)
- `sqlx` — PostgreSQL, MySQL
- `oracle` crate (hoặc JDBC bridge) — Oracle Database
- `mongodb` — MongoDB
- Các driver được wrap qua một `DatabaseExecutor` trait chung

### Xác thực & Phân quyền
- **SSO**: OAuth2/OIDC — thư viện `openidconnect` + `oauth2`
- **JWT**: `jsonwebtoken` — xác thực session sau khi đăng nhập SSO
- **RBAC**: tự implement theo role (Admin / Maker / Checker)

### Realtime & Notification
- **WebSocket**: axum built-in — thông báo realtime khi có request mới hoặc được approve
- **Logging**: `tracing` + `tracing-subscriber` (JSON stdout) + `tracing-appender` (daily file rotation)

## Frontend

- **Framework**: **Svelte** + Vite
- **Package manager**: `pnpm`
- **UI Components**: DaisyUI (Tailwind CSS) — nhẹ, không cần thư viện JS nặng
- **State management**: Svelte stores (built-in)
- **HTTP client**: `fetch` API (native), wrap qua một module `api/`
- **WebSocket client**: native browser WebSocket API

## Database hệ thống

- PostgreSQL (lưu toàn bộ dữ liệu của ứng dụng)
- Migration tool: `sqlx migrate`

## Common Commands

### Backend (Rust)

```bash
cargo build                  # compile debug
cargo build --release        # compile production
cargo run                    # chạy server
cargo test                   # chạy unit/integration tests
cargo clippy -- -D warnings  # lint (bắt buộc pass trước khi commit)
cargo fmt                    # format code
sqlx migrate run             # chạy database migration
sqlx migrate add <name>      # tạo migration mới
```

### Frontend (Svelte + Vite)

```bash
pnpm install       # cài dependencies
pnpm dev           # chạy dev server (http://localhost:5173)
pnpm build         # build production
pnpm preview       # preview bản build
pnpm lint          # lint với Prettier
```

## Code Style

### Rust
- Tuân theo `rustfmt` mặc định
- `cargo clippy -- -D warnings` phải pass trước khi merge
- Error handling: dùng `thiserror` cho custom errors, `anyhow` cho application-level errors
- Không dùng `unwrap()` trong production code — dùng `?` hoặc xử lý explicit

### Svelte / Frontend
- Format bằng Prettier
- Mỗi component trong một file `.svelte` riêng
- Không gọi API trực tiếp trong component — phải qua module `src/api/`

## Bảo mật (Security Rules)

- Không log câu SQL chứa dữ liệu nhạy cảm dưới dạng plaintext
- Tất cả kết nối đến database đích phải qua encrypted connection (TLS)
- Credentials của database đích phải được encrypt khi lưu vào PostgreSQL
- JWT token có thời hạn tối đa 8 giờ
