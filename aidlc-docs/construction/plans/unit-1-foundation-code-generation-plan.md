# Code Generation Plan — Unit 1: Foundation

**Unit**: unit-1-foundation  
**Date**: 2026-07-11  
**Status**: Executing

---

## Unit Context

- **Workspace Root**: `d:\Apps\dbms\`
- **Backend path**: `d:\Apps\dbms\backend\`
- **Frontend path**: `d:\Apps\dbms\frontend\`
- **Stories covered**: Project setup, DB schema, core models, health check
- **Dependencies**: None

---

## Steps

### Step 1: Project Structure Setup — Backend
- [x] Tạo `backend/Cargo.toml` với đầy đủ dependencies
- [x] Tạo `backend/src/main.rs` (skeleton)
- [x] Tạo `.env.example`
- [x] Tạo `docker-compose.yml`
- [x] Tạo `.gitignore`

### Step 2: Config Module
- [x] Tạo `backend/src/config.rs` — load từ env, fail fast

### Step 3: Error Module
- [x] Tạo `backend/src/error.rs` — `AppError` + `IntoResponse`

### Step 4: Domain Models
- [x] Tạo `backend/src/models/mod.rs`
- [x] Tạo `backend/src/models/user.rs` — `User`, `Role` enum
- [x] Tạo `backend/src/models/request.rs` — `SqlRequest`, `RequestStatus` + state machine
- [x] Tạo `backend/src/models/database.rs` — `DatabaseConnection`, `DbType`
- [x] Tạo `backend/src/models/approval.rs` — `AuditLog`, `AuditAction`
- [x] Tạo `backend/src/models/notification.rs` — `Notification`, `NotificationType`

### Step 5: Database Migrations
- [x] Tạo `backend/migrations/001_create_users.sql`
- [x] Tạo `backend/migrations/002_create_database_connections.sql`
- [x] Tạo `backend/migrations/003_create_sql_requests.sql` (với CHECK CONSTRAINT four-eyes)
- [x] Tạo `backend/migrations/004_create_audit_log.sql`
- [x] Tạo `backend/migrations/005_create_user_db_permissions.sql`
- [x] Tạo `backend/migrations/006_create_indexes.sql`

### Step 6: API Module Skeleton + Health Check
- [x] Tạo `backend/src/api/mod.rs` — router builder + security headers middleware
- [x] Tạo `backend/src/api/health.rs` — `GET /health`

### Step 7: Main.rs — Server Startup
- [x] Hoàn thiện `backend/src/main.rs` — full startup sequence
- [x] Tạo `backend/src/state.rs` — `AppState`

### Step 8: Frontend Project Setup
- [x] Tạo `frontend/` với Vite + Svelte template
- [x] Tạo `frontend/package.json` với đúng dependencies
- [x] Tạo `frontend/vite.config.js`
- [x] Tạo `frontend/index.html`
- [x] Tạo `frontend/src/app.css` — Tailwind + DaisyUI config

### Step 9: Unit Tests — Models
- [x] Tạo `backend/src/models/request.rs` tests — `can_transition_to()` invariants
- [x] PBT với `proptest`: state machine không có cycle, terminal states, serde round-trip

### Step 10: Unit Tests — Error & Config
- [x] Tests cho `AppError::into_response()` — đúng HTTP status codes
- [x] Tests cho `Config::from_env()` — fail khi thiếu required vars

### Step 11: Code Summary Documentation
- [x] Tạo `aidlc-docs/construction/unit-1-foundation/code/summary.md`

---

## Kết quả build & test

- **Build**: ✅ PASS (`cargo +stable-x86_64-pc-windows-gnu build`)
- **Tests**: ✅ 14/14 PASS (gồm 2 property-based tests)
- **Toolchain**: `stable-x86_64-pc-windows-gnu` + WinLibs MinGW-w64 gcc (máy không có MSVC linker)

---

## File Output Summary

```
d:\Apps\dbms\
├── backend/
│   ├── Cargo.toml
│   ├── migrations/
│   │   ├── 001_create_users.sql
│   │   ├── 002_create_database_connections.sql
│   │   ├── 003_create_sql_requests.sql
│   │   ├── 004_create_audit_log.sql
│   │   ├── 005_create_user_db_permissions.sql
│   │   └── 006_create_indexes.sql
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── error.rs
│       ├── models/
│       │   ├── mod.rs
│       │   ├── user.rs
│       │   ├── request.rs
│       │   ├── database.rs
│       │   └── approval.rs
│       └── api/
│           ├── mod.rs
│           └── health.rs
├── frontend/
│   ├── package.json
│   ├── vite.config.js
│   ├── index.html
│   └── src/
│       └── app.css
├── docker-compose.yml
├── .env.example
└── .gitignore
```
