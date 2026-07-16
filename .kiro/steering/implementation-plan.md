---
inclusion: auto
description: "Lộ trình triển khai SECS — kế hoạch từng giai đoạn, thứ tự dependency, và tiến độ implementation"
---

# Lộ trình triển khai SECS

Kế hoạch từng bước theo thứ tự dependency. Mỗi giai đoạn phải hoàn thành trước
khi bắt đầu giai đoạn tiếp theo (trừ G3+G4 có thể song song).

---

## Giai đoạn 0 — Chuẩn bị môi trường (1-2 ngày)

**Mục tiêu**: Team có thể clone và chạy được ngay.

### Việc cần làm
- [ ] `cargo new --lib backend` + `pnpm create vite frontend --template svelte`
- [ ] `docker-compose.yml`: PostgreSQL 16 + Keycloak 24 (giả lập SSO cho dev)
- [ ] `.env.example` với tất cả biến cần thiết
- [ ] `Cargo.toml` với đầy đủ dependencies (axum, sqlx, tokio, serde, jsonwebtoken, openidconnect, validator, thiserror, anyhow, aes-gcm, tracing, tower-http)
- [ ] `.gitignore` bao gồm `.env`, `target/`, `node_modules/`
- [ ] README.md: hướng dẫn setup môi trường

### Key Cargo.toml dependencies
```toml
axum = { version = "0.7", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "migrate"] }
uuid = { version = "1", features = ["v4", "serde"] }
jsonwebtoken = "9"
openidconnect = "3"
validator = { version = "0.18", features = ["derive"] }
thiserror = "1"
anyhow = "1"
aes-gcm = "0.10"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1"
# async-trait không cần thiết với Rust >= 1.75 (async fn in traits stable)
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
```

---

## Giai đoạn 1 — Nền tảng Backend (3-5 ngày)

**Quy tắc vàng**: Viết migration SQL trước, chạy `sqlx migrate run`, rồi mới viết Rust code.
`query!` macro cần database thật để compile.

### 1.1 Database migrations (làm đầu tiên)
```
migrations/
  001_create_users.sql
  002_create_database_connections.sql
  003_create_sql_requests.sql           ← CHECK CONSTRAINT four_eyes
  004_create_audit_log.sql
  005_create_user_db_permissions.sql
  006_create_indexes.sql
```
Chi tiết SQL: xem §14 trong `implementation-references.md`.

### 1.2 Cấu trúc file cần tạo (thứ tự)
```
src/config.rs          ← Load từ env, fail fast nếu thiếu
src/error.rs           ← AppError + IntoResponse (xem §8 trong impl-refs)
src/models/            ← Rust structs mapping với DB tables
  user.rs
  request.rs           ← RequestStatus enum + can_transition_to()
  database.rs
src/main.rs            ← Startup: pool, migrate, OIDC, broadcast, serve
```

### 1.3 main.rs skeleton
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    let config = Config::from_env()?;
    let db = PgPoolOptions::new().max_connections(20).connect(&config.database_url).await?;
    sqlx::migrate!("./src/db/migrations").run(&db).await?;
    let oidc_client = create_oidc_client(&config).await?;
    let (notification_tx, _) = broadcast::channel(128);
    let state = AppState { db, config, oidc_client, notification_tx };
    let app = build_router(state);
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

---

## Giai đoạn 2 — Authentication (2-3 ngày)

**Thứ tự**: jwt.rs → oidc.rs → middleware → api/auth.rs

### 2.1 Các file cần tạo
```
src/auth/jwt.rs      ← generate_token(), verify_token() — xem §9 trong impl-refs
src/auth/oidc.rs     ← create_oidc_client(), build_auth_url(), handle_callback()
src/api/auth.rs      ← 3 endpoints: /auth/login, /auth/callback, /auth/logout
```

### 2.2 Middleware auth (trong api/mod.rs)
Xem §1.1 và §1.2 trong `implementation-references.md`.
Inject `Claims` vào request extension, handler extract bằng `Extension<Claims>`.

### 2.3 API endpoints auth
```
GET  /auth/login     → redirect tới OIDC provider (build_auth_url)
GET  /auth/callback  → exchange code → tạo JWT → trả { token, user }
POST /auth/logout    → client-side only (clear localStorage)
GET  /auth/me        → trả user info từ JWT claims
```

### 2.4 Kiểm tra
- [ ] Login flow hoạt động với Keycloak local
- [ ] JWT hết hạn → 401
- [ ] Token sai signature → 401
- [ ] Request không có token → 401

---

## Giai đoạn 3 — Core Business Logic (5-7 ngày)

**Thứ tự implement theo dependency**:

### 3.1 db/ layer (không có business logic ở đây)
```
src/db/users.rs      ← get_by_oidc_sub, upsert, list, assign_role
src/db/requests.rs   ← create, submit, approve, reject, cancel, list, get_by_id
src/db/audit.rs      ← record_audit() — append-only
```

**Quy tắc cho mọi state-changing function trong db/requests.rs:**
1. Validate state transition (`can_transition_to()`)
2. Check 4-eyes nếu approve
3. Open transaction
4. Ghi audit log TRONG transaction
5. Update trạng thái
6. Commit
7. Broadcast notification SAU commit

### 3.2 executor/ layer
```
src/executor/mod.rs       ← DatabaseExecutor trait + create_executor() factory
src/executor/sql_analyzer.rs  ← analyze_sql(), detect_dangerous_patterns()
src/executor/postgres.rs  ← impl DatabaseExecutor cho PostgreSQL
src/executor/mysql.rs     ← impl DatabaseExecutor cho MySQL
src/executor/oracle.rs    ← stub/mock trước, implement sau
src/executor/mongodb.rs   ← stub/mock trước, implement sau
```

### 3.3 API handlers (thin layer)
```
src/api/mod.rs          ← build_router(), middleware stack
src/api/requests.rs     ← CRUD + state transitions
src/api/approvals.rs    ← approve, reject
src/api/databases.rs    ← CRUD DB connections
src/api/users.rs        ← list, assign role, permissions
src/api/dashboard.rs    ← stats aggregation
src/api/extractors.rs   ← ValidatedJson extractor
```

### 3.4 WebSocket
```
src/ws/mod.rs    ← ws_handler, handle_ws với tokio::select
```

### 3.5 Endpoints cần implement (ưu tiên theo thứ tự)
```
Batch 1 — Maker core:
  POST /api/v1/requests
  GET  /api/v1/requests
  GET  /api/v1/requests/:id
  POST /api/v1/requests/:id/submit
  POST /api/v1/requests/:id/cancel

Batch 2 — Checker core:
  POST /api/v1/requests/:id/approve
  POST /api/v1/requests/:id/reject

Batch 3 — Admin:
  GET/POST/PUT /api/v1/databases
  POST /api/v1/databases/:id/test
  GET/PUT /api/v1/users
  POST/DELETE /api/v1/users/:id/permissions

Batch 4 — Dashboard:
  GET /api/v1/dashboard
  GET /api/v1/requests/:id/audit
  GET /ws
```

---

## Giai đoạn 4 — Frontend (5-7 ngày)

Bắt đầu sau khi auth + Batch 1 backend hoàn thành.

### 4.1 Cài đặt
```bash
pnpm add -D tailwindcss daisyui @tailwindcss/typography prettier prettier-plugin-svelte
pnpm add @codemirror/view @codemirror/state @codemirror/lang-sql codemirror
```

### 4.2 Thứ tự implement pages
```
1. App.svelte           ← routing + auth guard
2. stores/auth.js       ← writable user, derived isChecker/isMaker/isAdmin
3. api/index.js         ← apiFetch với Bearer token
4. Login.svelte         ← SSO redirect button
5. components/Navbar.svelte + Sidebar
6. Dashboard.svelte     ← stats cards
7. maker/NewRequest.svelte  ← SqlEditor + form
8. maker/MyRequests.svelte  ← table + filter
9. checker/PendingList.svelte
10. checker/ReviewDetail.svelte  ← timeline + approve/reject modal
11. stores/notifications.js + components/Toast.svelte
12. admin/Users.svelte
13. admin/Databases.svelte
```

### 4.3 Component checklist
```
□ SqlEditor.svelte    ← CodeMirror 6 + SQL highlighting
□ StatusBadge.svelte  ← badge màu theo status
□ RequestCard.svelte  ← card trong danh sách
□ Navbar.svelte       ← notification bell + user menu
□ Toast.svelte        ← auto-dismiss sau 4s
□ ConfirmModal.svelte ← reusable cho approve/reject
□ AuditTimeline.svelte ← DaisyUI timeline
□ DdlWarning.svelte   ← alert warning cho DDL
□ Skeleton.svelte     ← loading placeholder
```

---

## Giai đoạn 5 — Hardening & Testing (2-3 ngày)

### 5.1 Security hardening
```
□ CORS: chỉ allow frontend origin
□ Rate limiting: tower-governor cho /auth/login
□ DefaultBodyLimit: 100KB
□ HTTPS redirect trong nginx
□ Revoke DELETE/UPDATE trên audit_log cho app DB user
□ Test 4-eyes constraint tại DB level
```

### 5.2 Error handling & UX
```
□ Tất cả 5xx hiển thị message thân thiện, không expose stack trace
□ Network error → toast "Mất kết nối, đang thử lại..."
□ WebSocket disconnect → auto reconnect với exponential backoff
□ Form validation error → highlight field + message cụ thể
```

### 5.3 Tests tối thiểu
```rust
// tests/integration/
□ test_auth_flow           ← login, JWT verify, expired token
□ test_request_lifecycle   ← create → submit → approve → execute
□ test_four_eyes           ← maker không approve được của mình
□ test_immutable_request   ← không sửa sau khi PENDING
□ test_audit_log           ← mọi action đều có log
□ test_state_machine       ← invalid transition trả lỗi
□ test_sql_analyzer        ← DDL detection, dangerous patterns
```

---

## Giai đoạn 6 — Production Deploy (1-2 ngày)

### 6.1 Build
```bash
cargo build --release              # binary: target/release/backend
cd frontend && pnpm build          # output: dist/
```

### 6.2 Docker images
```dockerfile
# backend/Dockerfile
FROM rust:1.79-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/backend /usr/local/bin/
CMD ["backend"]
```

### 6.3 nginx.conf (WebSocket support)
```nginx
location /ws {
    proxy_pass http://backend:3000;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_read_timeout 3600s;   # keep WS alive 1 giờ
}
```

### 6.4 Environment variables production
```env
DATABASE_URL=postgres://...     ← từ secret manager
JWT_SECRET=                     ← random 256-bit, không tái sử dụng
ENCRYPTION_KEY=                 ← 32-byte hex cho AES-256-GCM
OIDC_ISSUER_URL=                ← SSO công ty (không phải Keycloak dev)
OIDC_CLIENT_ID=
OIDC_CLIENT_SECRET=
SQL_EXECUTION_TIMEOUT_SECS=30
RUST_LOG=info,secs=debug
```

---

## Tổng thời gian ước tính

| Giai đoạn | Thời gian | Song song? |
|-----------|-----------|-----------|
| G0: Môi trường | 1-2 ngày | — |
| G1: Nền tảng | 3-5 ngày | — |
| G2: Auth | 2-3 ngày | — |
| G3: Backend core | 5-7 ngày | G4 bắt đầu sau Batch 1 |
| G4: Frontend | 5-7 ngày | Song song với G3 |
| G5: Hardening | 2-3 ngày | — |
| G6: Deploy | 1-2 ngày | — |
| **Tổng (1 dev)** | **~4 tuần** | |
| **Tổng (2 dev)** | **~2.5 tuần** | G3+G4 song song |

---

## Những lỗi thường gặp — cần tránh

1. **Không set `DATABASE_URL` khi build** → `query!` macro lỗi compile
2. **Audit log ngoài transaction** → mất trace nếu DB fail
3. **SQL execution sync trong handler** → HTTP timeout nếu SQL chạy lâu — dùng `tokio::spawn`
4. **Credential plaintext trong log** → vi phạm security, dùng `tracing` cẩn thận
5. **`unwrap()` trong production code** → panic không có graceful error
6. **CORS `allow_origin(Any)`** → lỗ hổng bảo mật
7. **WebSocket không verify JWT trước upgrade** → unauthorized connection
8. **Quên `CHECK CONSTRAINT` four_eyes** → business rule bị bypass nếu có bug
