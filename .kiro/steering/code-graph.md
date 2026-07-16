---
inclusion: auto
description: "Code graph và knowledge base tổng hợp cho hệ thống SECS — module map, schema, endpoints, business rules"
---

# Code Graph — SECS System Knowledge Base

Tài liệu này là knowledge base tập trung cho toàn bộ hệ thống SECS.
Khi cần tìm hiểu, truy vấn thông tin về bất kỳ thành phần nào, xem ở đây trước.

**Cập nhật lần cuối**: 2026-07-11  
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
                    ├───────────────────-┤
                    │ PgExecutor         │──► PostgreSQL target
                    │ MySqlExecutor      │──► MySQL target
                    │ OracleExecutor     │──► Oracle target
                    │ MongoExecutor      │──► MongoDB target
                    └───────────────────-┘
```

---

## 2. Backend Module Map

```
backend/src/
├── main.rs              → Entry point, startup sequence
├── config.rs            → Config::from_env(), fail fast
├── error.rs             → AppError enum + IntoResponse
├── state.rs             → AppState { db, config, notification_tx }
│
├── models/              → Domain types (no I/O, no side effects)
│   ├── mod.rs
│   ├── user.rs          → User, Role, UserDbPermission
│   ├── request.rs       → SqlRequest, RequestStatus + state machine
│   ├── database.rs      → DatabaseConnection, DbType, DatabaseConnectionResponse
│   ├── approval.rs      → AuditLog, AuditAction
│   └── notification.rs  → Notification, NotificationType
│
├── api/                 → HTTP handlers (axum routes) — MỎNG
│   ├── mod.rs           → build_router(), CORS, security headers middleware
│   ├── health.rs        → GET /health
│   ├── auth.rs          → [Unit 2] /auth/login, callback, logout, me
│   ├── requests.rs      → [Unit 3] CRUD + submit/cancel/approve/reject/clone
│   ├── approvals.rs     → [Unit 3] Checker-specific views
│   ├── databases.rs     → [Unit 3] CRUD + test connection
│   ├── users.rs         → [Unit 3] User management + permissions
│   └── dashboard.rs     → [Unit 3] Stats theo role
│
├── db/                  → [Unit 3] Database access layer (PostgreSQL internal)
│   ├── mod.rs
│   ├── users.rs         → CRUD users, check permission
│   ├── requests.rs      → CRUD requests, state transitions
│   └── audit.rs         → Append audit log
│
├── executor/            → [Unit 3] SQL execution trên database đích
│   ├── mod.rs           → DatabaseExecutor trait
│   ├── postgres.rs      → PgExecutor impl
│   ├── mysql.rs         → MySqlExecutor impl
│   ├── oracle.rs        → OracleExecutor impl
│   ├── mongodb.rs       → MongoExecutor impl
│   └── sql_analyzer.rs  → Static SQL analysis (DDL detect, dangerous patterns)
│
├── auth/                → [Unit 2] Authentication
│   ├── mod.rs
│   ├── jwt.rs           → generate/verify JWT, Claims struct
│   ├── oidc.rs          → OIDC/PKCE flow
│   └── middleware.rs    → Auth middleware + RBAC extractors
│
└── ws/                  → [Unit 3] WebSocket handlers
    └── mod.rs           → Realtime notification broadcast
```

---

## 3. Frontend Module Map

```
frontend/src/
├── main.js              → Svelte app mount
├── app.css              → Tailwind + DaisyUI
├── App.svelte           → Root component (routing sẽ thêm ở Unit 4)
│
├── api/                 → HTTP client tập trung (DUY NHẤT nơi gọi fetch)
│   └── index.js         → apiFetch() + api.auth/requests/databases/users/dashboard
│
├── stores/              → Svelte stores (global state)
│   ├── auth.js          → authStore { token, user, login(), logout() }
│   └── notifications.js → [Unit 4] WebSocket notifications
│
├── components/          → [Unit 4] Reusable UI components
│   ├── Navbar.svelte
│   ├── SqlEditor.svelte
│   ├── StatusBadge.svelte
│   ├── RequestCard.svelte
│   ├── Toast.svelte
│   ├── ConfirmModal.svelte
│   └── AuditTimeline.svelte
│
└── pages/               → [Unit 4] Page-level views
    ├── Login.svelte
    ├── Dashboard.svelte
    ├── maker/
    │   ├── NewRequest.svelte
    │   └── MyRequests.svelte
    ├── checker/
    │   ├── PendingList.svelte
    │   └── ReviewDetail.svelte
    └── admin/
        ├── Users.svelte
        └── Databases.svelte
```

---

## 4. Database Schema Graph

```
┌───────────────┐     ┌─────────────────────┐     ┌──────────────────┐
│    users      │     │  database_connections│     │  user_db_perms   │
├───────────────┤     ├─────────────────────┤     ├──────────────────┤
│ id (PK)       │◄───┐│ id (PK)             │◄──┐ │ id (PK)          │
│ oidc_subject  │    ││ name (unique)       │   │ │ user_id (FK)────►│users
│ email (unique)│    ││ db_type             │   │ │ database_id (FK)►│database_connections
│ display_name  │    ││ host, port          │   │ │ granted_by (FK)─►│users
│ is_active     │    ││ database_name       │   │ │ granted_at       │
│ created_at    │    ││ schema_name         │   │ │ UNIQUE(user,db)  │
│ updated_at    │    ││ encrypted_credentials   │ └──────────────────┘
└───────┬───────┘    ││ timeout_secs        │
        │            ││ is_active           │
        │            │└─────────────────────┘
        │            │
┌───────┴───────┐    │    ┌──────────────────────────────┐
│  user_roles   │    │    │        sql_requests           │
├───────────────┤    │    ├──────────────────────────────┤
│ id (PK)       │    │    │ id (PK)                      │
│ user_id (FK)──┤────┘    │ description                  │
│ role          │         │ sql_content                  │
│ granted_by    │         │ status (CHECK enum)          │
│ granted_at    │         │ database_id (FK)─────────────┤►database_connections
│ UNIQUE(uid,r) │         │ created_by (FK)──────────────┤►users
└───────────────┘         │ approved_by (FK)─────────────┤►users
                          │ rejection_reason             │
                          │ executed_at, rows_affected   │
                          │ execution_error              │
                          │ submitted_at                 │
                          │ CHECK: four_eyes             │
                          │ CHECK: rejection_reason      │
                          └──────────────┬───────────────┘
                                         │
                          ┌──────────────┴───────────────┐
                          │         audit_log            │
                          ├─────────────────────────────┤
                          │ id (PK)                      │
                          │ request_id (FK)──────────────┤►sql_requests
                          │ action (CHECK enum)          │
                          │ actor_id (FK)────────────────┤►users
                          │ detail                       │
                          │ execution_result             │
                          │ ip_address                   │
                          │ timestamp                    │
                          │ ** APPEND-ONLY **            │
                          └─────────────────────────────┘
```

---

## 5. Data Flow: Request Lifecycle

```
Maker                    System                   Checker              Target DB
  │                        │                        │                     │
  │──CREATE───────────────►│ sql_requests.DRAFT     │                     │
  │                        │ audit_log.CREATED      │                     │
  │──SUBMIT───────────────►│ sql_requests.PENDING   │                     │
  │                        │ audit_log.SUBMITTED    │                     │
  │                        │ ──WS notification─────►│                     │
  │                        │                        │                     │
  │                        │◄──────────APPROVE──────│                     │
  │                        │ sql_requests.APPROVED  │                     │
  │                        │ audit_log.APPROVED     │                     │
  │                        │                        │                     │
  │                        │──EXECUTE (async)──────────────────────────►│
  │                        │                        │     rows_affected   │
  │                        │◄─────────────────────────────────────────── │
  │                        │ sql_requests.EXECUTED  │                     │
  │                        │ audit_log.EXECUTED     │                     │
  │◄──WS notification─────│────────WS notification►│                     │
```

---

## 6. Dependency Graph (Crates)

```
secs-backend
├── axum 0.7        (HTTP framework + WebSocket)
├── tower 0.4       (middleware)
├── tower-http 0.5  (CORS, trace, timeout, limit)
├── tokio 1         (async runtime)
├── sqlx 0.8        (PostgreSQL + MySQL driver, migrations, compile-time check)
├── serde 1         (serialization)
├── serde_json 1    (JSON)
├── jsonwebtoken 9  (JWT create/verify)
├── validator 0.18  (input validation)
├── openidconnect 3 (OIDC/PKCE flow)
├── oauth2 4        (OAuth2 base)
├── thiserror 2     (custom errors)
├── anyhow 1        (application errors)
├── uuid 1          (v4, serde)
├── chrono 0.4      (datetime, serde)
├── tracing 0.1     (structured logging)
├── tracing-subscriber 0.3 (JSON output)
├── dotenvy 0.15    (.env loading)
├── aes-gcm 0.10    (AES-256-GCM credential encryption)
├── base64 0.22     (encoding)
├── hex 0.4         (hex decode for encryption key)
└── regex 1         (SQL analysis patterns)

[dev-dependencies]
├── proptest 1      (property-based testing)
└── tokio-test 0.4  (async test utils)
```

---

## 7. Business Rules Quick Reference

| ID | Rule | Enforce tại |
|----|------|-------------|
| BR-001 | 4-eyes: Maker ≠ Checker cho cùng request | DB constraint + application |
| BR-002 | Immutable: SQL không sửa được sau PENDING | Application layer |
| BR-003 | State machine: chỉ valid transitions | `can_transition_to()` |
| BR-004 | Rejection reason ≥ 10 chars | DB constraint + validator |
| BR-005 | Audit log bắt buộc cho mọi state change | Transaction (cùng TX) |
| BR-006 | Permission check trước tạo request | `user_db_permissions` lookup |
| BR-007 | Multi-statement SQL trong transaction | Executor wraps in TX |
| BR-008 | Timeout enforcement (default 30s, max 300s) | `tokio::time::timeout` |
| BR-009 | Credentials encrypt AES-256-GCM, TLS bắt buộc | Encrypt before save, TLS connect |
| BR-010 | Soft delete users (is_active=false) | Application + middleware check |

---

## 8. Security Controls

| Control | Implementation |
|---------|---------------|
| Auth | OIDC/PKCE → JWT (8h expiry, no refresh) |
| RBAC | Middleware inject Claims, extractors per role |
| HTTP Headers | CSP, HSTS, X-Frame-Options, X-Content-Type-Options, Referrer-Policy |
| CORS | Only FRONTEND_URL origin |
| Input validation | `validator` crate, `ValidatedJson` extractor |
| Credential storage | AES-256-GCM, never log/return in API |
| Audit | Append-only, app user has no DELETE on audit_log |
| Rate limit | /auth/login max 10 req/min |
| Body size | 100KB max (tower-http RequestBodyLimitLayer) |

---

## 9. Endpoint Index (cho frontend reference)

| Method | Path | Role | Mô tả |
|--------|------|------|--------|
| GET | /health | Public | Health check |
| GET | /api/v1/auth/login | Public | Redirect OIDC |
| GET | /api/v1/auth/callback | Public | OIDC callback |
| POST | /api/v1/auth/logout | Any | Logout |
| GET | /api/v1/auth/me | Any | Current user info |
| POST | /api/v1/requests | MAKER | Create request |
| GET | /api/v1/requests | Any | List requests |
| GET | /api/v1/requests/:id | Any | Request detail |
| PUT | /api/v1/requests/:id | MAKER | Edit DRAFT |
| POST | /api/v1/requests/:id/submit | MAKER | Submit |
| POST | /api/v1/requests/:id/cancel | MAKER | Cancel |
| POST | /api/v1/requests/:id/approve | CHECKER | Approve |
| POST | /api/v1/requests/:id/reject | CHECKER | Reject |
| POST | /api/v1/requests/:id/clone | MAKER | Clone |
| GET | /api/v1/databases | Any | List databases |
| POST | /api/v1/databases | ADMIN | Create database |
| PATCH | /api/v1/databases/:id | ADMIN | Update database |
| POST | /api/v1/databases/:id/test | ADMIN | Test connection |
| GET | /api/v1/users | ADMIN | List users |
| PATCH | /api/v1/users/:id | ADMIN | Update user |
| POST | /api/v1/users/:uid/permissions | ADMIN | Grant permission |
| DELETE | /api/v1/users/:uid/permissions/:db_id | ADMIN | Revoke permission |
| GET | /api/v1/dashboard | Any | Dashboard stats |
| GET | /ws?token=JWT | Any | WebSocket |

---

## 10. File Ownership (ai sở hữu file nào)

| Concern | Files | Layer |
|---------|-------|-------|
| Domain logic | `models/*.rs` | Pure, no I/O |
| DB queries | `db/*.rs` | sqlx queries, transactions |
| HTTP handlers | `api/*.rs` | Parse req → call db → respond |
| SQL execution | `executor/*.rs` | Connect target DB, execute SQL |
| Auth flow | `auth/*.rs` | OIDC, JWT, middleware |
| Realtime | `ws/mod.rs` | WebSocket broadcast |
| Config | `config.rs` | Env vars |
| Errors | `error.rs` | Central error mapping |
| Frontend API | `frontend/src/api/index.js` | Single source of truth |
| Frontend state | `frontend/src/stores/*.js` | Svelte stores |
| Frontend UI | `frontend/src/components/*.svelte` | Reusable components |
| Frontend pages | `frontend/src/pages/**/*.svelte` | Page views |

---

## 11. Build & Run

```bash
# Backend
cargo +stable-x86_64-pc-windows-gnu build      # compile
cargo +stable-x86_64-pc-windows-gnu test        # test (14 pass)
cargo +stable-x86_64-pc-windows-gnu run         # start server :3000

# Frontend
pnpm install                                     # install deps
pnpm dev                                         # dev server :5173
pnpm build                                       # production build

# Infrastructure
docker compose up -d                             # PostgreSQL :5432 + Keycloak :8080
```

---

## 12. Conventions

| Area | Convention |
|------|-----------|
| API routes | `/api/v1/resource` (plural nouns) |
| Error codes | HTTP status + JSON `{ "error": "message" }` |
| DB naming | snake_case tables/columns |
| Rust naming | snake_case functions, PascalCase types |
| Frontend files | PascalCase .svelte, camelCase .js |
| Commits | `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:` |
| State transitions | Enum method `can_transition_to()` |
| Secrets | Never log, never return in API, AES-256-GCM at rest |
