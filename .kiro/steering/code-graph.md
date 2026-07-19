---
inclusion: auto
description: "Code graph và knowledge base tổng hợp cho hệ thống SECS — module map, schema, endpoints, business rules"
---

# Code Graph — SECS System Knowledge Base

Tài liệu này là knowledge base tập trung cho toàn bộ hệ thống SECS.
Khi cần tìm hiểu, truy vấn thông tin về bất kỳ thành phần nào, xem ở đây trước.

**Cập nhật lần cuối**: 2026-07-17
**Quy tắc**: File này PHẢI được cập nhật mỗi khi thêm module/file/endpoint mới.

---

## 1. Tổng quan kiến trúc

```
┌─────────────┐        ┌──────────────┐        ┌─────────────────┐
│   Frontend  │◄──────►│   Backend    │◄──────►│  PostgreSQL     │
│   (Svelte)  │  HTTP  │   (Rust)     │  sqlx  │  (internal DB)  │
│   :5173     │  + WS  │   :3000      │        │  :5432          │
└─────────────┘        └──────┬───────┘        └─────────────────┘
                              │
                    ┌─────────┴──────────┐
                    │  DatabaseExecutor  │
                    │  (trait)           │
                    ├────────────────────┤
                    │ PgExecutor         │──► PostgreSQL target (TLS)
                    │ MySqlExecutor      │──► MySQL target (TLS)
                    │ OracleExecutor     │──► Oracle target (TCPS)
                    │ MongoExecutor      │──► MongoDB target (TLS)
                    └────────────────────┘
```

---

## 2. Backend Module Map

```
backend/src/
├── main.rs              → Entry point, startup, dual logging (stdout JSON + file daily rotation)
├── config.rs            → Config::from_env(), fail fast, LOG_DIR support
├── error.rs             → AppError enum + IntoResponse (4xx/5xx mapping)
├── state.rs             → AppState { db, config, notification_tx, oidc_client, oidc_sessions }
│
├── models/              → Domain types (no I/O, no side effects)
│   ├── mod.rs
│   ├── user.rs          → User, Role, UserDbPermission
│   ├── request.rs       → SqlRequest, RequestStatus + state machine + approval_levels_required
│   ├── database.rs      → DatabaseConnection, DbType, DatabaseConnectionResponse, MakerDatabaseResponse, UpdateCredentialsBody
│   ├── approval.rs      → AuditLog, AuditAction (incl. Executing, RoleChanged, UserStatusChanged, CredentialRotated)
│   ├── notification.rs  → Notification, NotificationType (incl. PartialApproval)
│   └── request_approval.rs → RequestApproval (multi-level approval tracking)
│
├── api/                 → HTTP handlers (axum routes) — MỎNG
│   ├── mod.rs           → build_router(), CORS, security headers, RequestBodyLimitLayer 100KB
│   ├── health.rs        → GET /health (DB pool check)
│   ├── auth.rs          → OIDC login/callback/logout/me, OidcSessionStore, session cleanup
│   ├── requests.rs      → Maker: create/list/detail/update/submit/cancel/clone (pagination)
│   ├── approvals.rs     → Checker: list_pending/detail/approve/reject + async SQL execution
│   ├── audit.rs         → GET /requests/:id/audit (access control: owner/approver/admin)
│   ├── databases.rs     → Admin: CRUD + test/test-new/credentials rotation + Maker: my-permissions
│   ├── users.rs         → Admin: list/detail/update (roles/status) + permissions CRUD
│   ├── dashboard.rs     → Role-aware stats (Admin/Checker/Maker)
│   └── ip_extractor.rs  → extract_client_ip() from X-Forwarded-For/X-Real-IP/ConnectInfo
│
├── db/                  → Database access layer (PostgreSQL internal)
│   ├── mod.rs
│   ├── users.rs         → find_by_subject, upsert_from_oidc, get_user_roles, first-user bootstrap
│   ├── requests.rs      → CRUD, state transitions, multi-level approval (insert/count/has_approved)
│   ├── audit.rs         → insert, get_by_request_id, insert_admin_action
│   ├── databases.rs     → CRUD, list_by_user_permission, update_credentials
│   └── permissions.rs   → check/grant/revoke/list permissions
│
├── executor/            → SQL execution trên database đích
│   ├── mod.rs           → DatabaseExecutor trait, execute_sql(), test_db_connection(), ExecutorError
│   ├── encryption.rs    → AES-256-GCM encrypt/decrypt credentials (nonce || ciphertext → base64)
│   ├── postgres.rs      → PgExecutor (sslmode=require, transaction, multi-statement)
│   ├── mysql.rs         → MySqlExecutor (ssl-mode=REQUIRED, transaction, multi-statement)
│   ├── oracle.rs        → OracleExecutor (TCPS, spawn_blocking, transaction)
│   ├── mongodb.rs       → MongoExecutor (tls=true, JSON commands, session transaction)
│   └── sql_analyzer.rs  → DDL detection (DdlSeverity::Warning/Danger, SqlAnalysis)
│
├── auth/                → Authentication & Authorization
│   ├── mod.rs
│   ├── jwt.rs           → Claims, generate_token, verify_token (HS256, 8h, issuer validation)
│   ├── oidc.rs          → create_oidc_client, build_auth_url, handle_oidc_callback (PKCE + CSRF + at_hash)
│   ├── middleware.rs    → require_auth (extract Bearer token → inject Claims)
│   ├── extractors.rs    → RequireAuth, RequireAdmin, RequireMaker, RequireChecker (FromRequestParts)
│   └── rate_limit.rs    → RateLimiter (10 req/min per IP on /auth/login, sliding window)
│
└── ws/                  → WebSocket handlers
    └── mod.rs           → ws_handler, role-based notification filtering, has_previously_reviewed
```

---

## 3. Frontend Module Map

```
frontend/src/
├── main.js              → Svelte app mount
├── app.css              → Tailwind + DaisyUI
├── App.svelte           → Root component, svelte-spa-router, auth guard, WS connect
│
├── api/                 → HTTP client tập trung (DUY NHẤT nơi gọi fetch)
│   ├── index.js         → apiFetch() wrapper + api.auth/requests/approvals/databases/users/dashboard
│   └── endpoints.js     → Constants cho tất cả endpoint paths (single source of truth)
│
├── stores/              → Svelte stores (global state)
│   ├── auth.js          → authStore { token, user, login(), logout() } — localStorage persistence
│   └── notifications.js → WebSocket notification store, auto-reconnect 3s
│
├── components/          → Reusable UI components
│   ├── Navbar.svelte    → Logo, theme toggle (dark/light), user dropdown
│   ├── Sidebar.svelte   → Role-based navigation menu
│   ├── SqlEditor.svelte → CodeMirror 6 (SQL + JSON highlighting, readonly mode)
│   ├── StatusBadge.svelte → Badge theo status (color mapping)
│   └── Toast.svelte     → Notification toast (top-end, auto từ WS notifications)
│
└── pages/               → Page-level views
    ├── Login.svelte         → SSO login redirect
    ├── AuthCallback.svelte  → OIDC callback (token từ URL fragment → authStore)
    ├── Dashboard.svelte     → Role-aware stats + quick links
    ├── maker/
    │   ├── NewRequest.svelte    → Form tạo request (auto-detect SQL/JSON by db_type)
    │   ├── MyRequests.svelte    → Paginated list + status filter + submit/cancel actions
    │   └── RequestDetail.svelte → Detail + edit DRAFT + audit timeline + clone
    ├── checker/
    │   ├── PendingList.svelte   → Filterable/sortable list + multi-level progress
    │   └── ReviewDetail.svelte  → SQL viewer + DDL analysis + approve/reject
    └── admin/
        ├── Databases.svelte     → CRUD + test connection (existing + new)
        └── Users.svelte         → Role management + permissions + activate/deactivate
```

---

## 4. Database Schema (8 migrations)

### Tables
| Table | Purpose |
|-------|---------|
| `users` | OIDC users (soft delete via is_active) |
| `user_roles` | Many-to-many: user ↔ role (ADMIN/MAKER/CHECKER) |
| `database_connections` | Target DB configs + encrypted credentials + approval_levels_required |
| `sql_requests` | Central entity: SQL + status + 4-eyes constraint + approval_levels_required |
| `request_approvals` | Multi-level approval tracking (UNIQUE request_id + approver_id) |
| `user_db_permissions` | Explicit user ↔ database permission grants |
| `audit_log` | Append-only audit trail (nullable request_id for admin actions) |

### Key Constraints
- `four_eyes_check`: approved_by IS NULL OR approved_by != created_by
- `rejection_reason_required`: status != 'REJECTED' OR rejection_reason >= 10 chars
- `audit_log.action` CHECK: CREATED, SUBMITTED, APPROVED, REJECTED, EXECUTED, FAILED, CANCELLED, ROLE_CHANGED, USER_STATUS_CHANGED, CREDENTIAL_ROTATED
- `approval_levels_required` BETWEEN 1 AND 5
- `request_approvals` UNIQUE(request_id, approver_id)

---

## 5. Endpoint Index (complete, current)

| Method | Path | Role | Mô tả |
|--------|------|------|--------|
| GET | /health | Public | Health check (DB pool) |
| GET | /auth/login | Public | Redirect OIDC (rate limited 10/min) |
| GET | /auth/callback | Public | OIDC callback → JWT |
| GET | /auth/me | Any | Current user info from JWT |
| POST | /auth/logout | Any | Logout (stateless) |
| POST | /api/v1/requests | MAKER | Create request (DRAFT or PENDING) |
| GET | /api/v1/requests | MAKER | List my requests (paginated, status filter) |
| GET | /api/v1/requests/:id | MAKER | Request detail (owner only, incl. sql_content) |
| PATCH | /api/v1/requests/:id | MAKER | Edit DRAFT (description + sql_content) |
| POST | /api/v1/requests/:id/submit | MAKER | DRAFT → PENDING |
| POST | /api/v1/requests/:id/cancel | MAKER | PENDING → CANCELLED |
| POST | /api/v1/requests/:id/clone | MAKER | Clone → new DRAFT |
| GET | /api/v1/requests/:id/audit | Any* | Audit trail (owner/approver/admin) |
| GET | /api/v1/approvals/pending | CHECKER | Pending requests (excluding own) |
| GET | /api/v1/approvals/:id | CHECKER | Request detail for review |
| POST | /api/v1/approvals/:id/approve | CHECKER | Approve (multi-level, triggers execution when threshold met) |
| POST | /api/v1/approvals/:id/reject | CHECKER | Reject (reason min 10 chars) |
| GET | /api/v1/databases | ADMIN | List all databases |
| GET | /api/v1/databases/:id | ADMIN | Database detail |
| POST | /api/v1/databases | ADMIN | Create database (encrypt credentials) |
| PATCH | /api/v1/databases/:id | ADMIN | Update database metadata |
| PATCH | /api/v1/databases/:id/credentials | ADMIN | Rotate credentials (audit log) |
| POST | /api/v1/databases/:id/test | ADMIN | Test existing connection |
| POST | /api/v1/databases/test-new | ADMIN | Test with raw params (pre-save) |
| GET | /api/v1/databases/my-permissions | MAKER | List databases user has access to |
| GET | /api/v1/users | ADMIN | List all users with roles |
| GET | /api/v1/users/:id | ADMIN | User detail |
| PATCH | /api/v1/users/:id | ADMIN | Update user (roles, is_active) |
| GET | /api/v1/users/:id/permissions | ADMIN | List user's DB permissions |
| POST | /api/v1/users/:id/permissions | ADMIN | Grant permission |
| DELETE | /api/v1/users/:id/permissions/:db_id | ADMIN | Revoke permission |
| GET | /api/v1/dashboard | Any | Role-aware stats |
| GET | /ws?token=JWT | Any | WebSocket (role-based filtering) |

---

## 6. Dependency Graph (Crates)

```
secs-backend v0.1.0
├── axum 0.7        (HTTP + WebSocket, macros)
├── tower 0.4       (middleware)
├── tower-http 0.5  (CORS, trace, timeout, limit)
├── tokio 1         (async runtime, full)
├── sqlx 0.8        (PostgreSQL + MySQL, migrations, chrono, uuid)
├── serde 1         (serialization, derive)
├── serde_json 1    (JSON)
├── jsonwebtoken 9  (JWT HS256, 8h expiry)
├── validator 0.18  (input validation, derive)
├── openidconnect 3 (OIDC/PKCE flow)
├── oauth2 4        (OAuth2 base)
├── thiserror 2     (custom errors)
├── anyhow 1        (application errors)
├── async-trait 0.1 (trait async methods)
├── uuid 1          (v4, serde)
├── chrono 0.4      (datetime, serde)
├── tracing 0.1     (structured logging)
├── tracing-subscriber 0.3 (env-filter, json)
├── tracing-appender 0.2   (daily file rotation, non-blocking)
├── dotenvy 0.15    (.env loading)
├── aes-gcm 0.10    (AES-256-GCM credential encryption)
├── base64 0.22     (encoding)
├── hex 0.4         (encryption key decode)
├── url 2           (URL types for OIDC)
├── urlencoding 2   (URL-encode credentials in connection strings)
├── regex 1         (SQL analysis patterns)
├── mongodb 3       (MongoDB driver)
└── oracle 0.6      (optional, behind "oracle-support" feature)

[dev-dependencies]
├── proptest 1      (property-based testing)
└── tokio-test 0.4  (async test utils)
```

---

## 7. Business Rules Quick Reference

| ID | Rule | Enforce tại |
|----|------|-------------|
| BR-001 | 4-eyes: Maker ≠ Checker cho cùng request | DB CHECK constraint + application |
| BR-002 | Immutable: SQL không sửa được sau PENDING | Application (status check) |
| BR-003 | State machine: chỉ valid transitions | `can_transition_to()` |
| BR-004 | Rejection reason ≥ 10 chars | DB CHECK + validator |
| BR-005 | Audit log bắt buộc cho mọi state change | Application (same flow) |
| BR-006 | Permission check trước tạo request | `check_permission()` |
| BR-007 | Multi-statement SQL trong transaction | Executor wraps in TX |
| BR-008 | Timeout enforcement (default 30s, max 300s) | `tokio::time::timeout` + config cap |
| BR-009 | Credentials encrypt AES-256-GCM, TLS bắt buộc | Encrypt before save, TLS in connection strings |
| BR-010 | Soft delete users (is_active=false) | Application + middleware active check |
| BR-011 | Multi-level approval (1-5 levels per database) | request_approvals table + count check |
| BR-012 | Duplicate approval prevention | UNIQUE constraint + has_already_approved() |
| BR-013 | First-user bootstrap (auto ADMIN) | ensure_first_user_has_admin() |
| BR-014 | Admin self-protection | Cannot deactivate self or remove own ADMIN |

---

## 8. Security Controls

| Control | Implementation |
|---------|---------------|
| Auth | OIDC/PKCE → JWT (HS256, 8h expiry, issuer validation) |
| RBAC | Middleware inject Claims, extractors per role (RequireAdmin/Maker/Checker) |
| HTTP Headers | CSP, HSTS, X-Frame-Options DENY, X-Content-Type-Options nosniff, Referrer-Policy |
| CORS | Only FRONTEND_URL origin |
| Input validation | `validator` crate on DTOs, manual validate in handlers |
| Credential storage | AES-256-GCM (random 96-bit nonce), never log/return in API |
| Audit | Append-only, nullable request_id for admin actions |
| Rate limit | /auth/login max 10 req/min per IP (sliding window) |
| Body size | 100KB max (RequestBodyLimitLayer) |
| IP tracking | X-Forwarded-For → X-Real-IP → ConnectInfo (max 45 chars) |
| TLS enforcement | PostgreSQL: sslmode=require, MySQL: ssl-mode=REQUIRED, Oracle: TCPS, MongoDB: tls=true |

---

## 9. Config Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| DATABASE_URL | ✓ | - | PostgreSQL nội bộ |
| JWT_SECRET | ✓ | - | Min 32 chars |
| ENCRYPTION_KEY | ✓ | - | 64 hex chars (32 bytes) |
| OIDC_ISSUER_URL | ✓ | - | Keycloak realm URL |
| OIDC_CLIENT_ID | ✓ | - | Client ID |
| OIDC_CLIENT_SECRET | ✓ | - | Client secret |
| OIDC_REDIRECT_URI | ✓ | - | Callback URL |
| FRONTEND_URL | ✓ | - | CORS origin |
| SQL_EXECUTION_TIMEOUT_SECS | ✗ | 30 | Max 300s |
| OIDC_SESSION_TTL_SECS | ✗ | 300 | Range 60-900 |
| OIDC_CLEANUP_INTERVAL_SECS | ✗ | 60 | Range 10-300 |
| PORT | ✗ | 3000 | Server port |
| LOG_DIR | ✗ | ./logs | Log file directory (daily rotation) |
| RUST_LOG | ✗ | info | Tracing filter |

---

## 10. Build & Run

```bash
# Backend
cargo build                    # compile debug
cargo build --release          # compile production
cargo run                      # start server :3000
cargo test                     # run all tests
cargo clippy -- -D warnings    # lint
cargo fmt                      # format

# Frontend
pnpm install                   # install deps
pnpm dev                       # dev server :5173
pnpm build                     # production build
pnpm lint                      # prettier check

# Infrastructure
docker compose up -d           # PostgreSQL :5432 + Keycloak :8080
docker compose up              # Full stack (backend :3000 + frontend :8088)
```

---

## 11. Conventions

| Area | Convention |
|------|-----------|
| API routes | `/api/v1/resource` (plural nouns) |
| Error codes | HTTP status + JSON `{ "error": "message" }` |
| Pagination | `{ data: [...], pagination: { page, per_page, total, total_pages } }` |
| DB naming | snake_case tables/columns |
| Rust naming | snake_case functions, PascalCase types |
| Frontend files | PascalCase .svelte, camelCase .js |
| Commits | `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:` |
| State transitions | Enum method `can_transition_to()` |
| Secrets | Never log, never return in API, AES-256-GCM at rest |
| Logging | Dual output: stdout (JSON) + file (daily rotation, human-readable) |
| Routing | svelte-spa-router (hash-based: `/#/path`) |

---

## 12. File Ownership

| Concern | Files | Layer |
|---------|-------|-------|
| Domain logic | `models/*.rs` | Pure, no I/O |
| DB queries | `db/*.rs` | sqlx queries, transactions |
| HTTP handlers | `api/*.rs` | Parse req → call db → respond |
| SQL execution | `executor/*.rs` | Connect target DB, execute SQL |
| Credential security | `executor/encryption.rs` | AES-256-GCM |
| SQL safety | `executor/sql_analyzer.rs` | DDL detection |
| Auth flow | `auth/*.rs` | OIDC, JWT, middleware, extractors |
| Rate limiting | `auth/rate_limit.rs` | Sliding window per IP |
| IP extraction | `api/ip_extractor.rs` | X-Forwarded-For priority |
| Realtime | `ws/mod.rs` | WebSocket broadcast + role filter |
| Config | `config.rs` | Env vars |
| Errors | `error.rs` | Central error mapping |
| Frontend API | `frontend/src/api/index.js` | Single fetch wrapper |
| Frontend paths | `frontend/src/api/endpoints.js` | Single source of truth |
| Frontend state | `frontend/src/stores/*.js` | Svelte stores |
| Frontend UI | `frontend/src/components/*.svelte` | Reusable components |
| Frontend pages | `frontend/src/pages/**/*.svelte` | Page views |
