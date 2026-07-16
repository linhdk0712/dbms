# Units of Work — SECS

**Date**: 2026-07-11  
**Status**: Approved

---

## Unit 1 — Foundation

**ID**: `unit-1-foundation`  
**Priority**: 1 (bắt đầu ngay)  
**Dependency**: None

### Scope
- Database migrations (6 migration files)
- Rust models (`user.rs`, `request.rs`, `approval.rs`, `database.rs`)
- `config.rs` — load từ env, fail fast
- `error.rs` — `AppError` + `IntoResponse`
- `main.rs` — startup: pool, migrate, build router, serve
- `GET /health` endpoint

### Deliverables
- Server khởi động, connect DB, run migrations tự động
- Health check endpoint hoạt động
- `proptest` configured (PBT-09)

---

## Unit 2 — Authentication

**ID**: `unit-2-auth`  
**Priority**: 2  
**Dependency**: Unit 1

### Scope
- `src/auth/jwt.rs` — generate/verify JWT
- `src/auth/oidc.rs` — OIDC/PKCE flow
- `src/api/auth.rs` — `/auth/login`, `/auth/callback`, `/auth/logout`, `/auth/me`
- Auth middleware — inject `Claims` vào request extensions
- RBAC extractors (`RequireMaker`, `RequireChecker`, `RequireAdmin`)

### Deliverables
- Login flow hoàn chỉnh với Keycloak local
- JWT verify trên mọi protected request
- 401/403 đúng theo role

---

## Unit 3 — Core Business Logic

**ID**: `unit-3-core`  
**Priority**: 3  
**Dependency**: Unit 1 + Unit 2

### Scope
- `src/db/` layer — users, requests, audit
- `src/executor/` — `DatabaseExecutor` trait + PostgreSQL + MySQL + Oracle + MongoDB impls + `sql_analyzer.rs`
- `src/api/` handlers — requests, approvals, databases, users, dashboard
- `src/ws/` — WebSocket notification
- Tất cả business endpoints (Batch 1–4 theo `implementation-plan.md`)

### Deliverables
- Full Maker-Checker workflow hoạt động end-to-end
- SQL execution với timeout và transaction
- Realtime notification qua WebSocket
- Audit log đầy đủ

---

## Unit 4 — Frontend

**ID**: `unit-4-frontend`  
**Priority**: 4 (có thể song song với Unit 3 sau khi Unit 2 xong)  
**Dependency**: Unit 1 + Unit 2 (Unit 3 để full feature)

### Scope
- Svelte app setup (Vite + DaisyUI + Tailwind)
- `src/stores/` — auth, notifications
- `src/api/` module — apiFetch wrapper + request/approval/database modules
- Tất cả pages: Login, Dashboard, Maker, Checker, Admin
- Components: SqlEditor, StatusBadge, RequestCard, Navbar, Toast, ConfirmModal, AuditTimeline

### Deliverables
- Web app đầy đủ chức năng theo `system-design.md` §5
- WebSocket realtime notification
- Responsive layout với DaisyUI

---

## Dependency Graph

```
Unit 1: Foundation
    |
    v
Unit 2: Auth
    |
    +---------> Unit 4: Frontend (có thể song song)
    |
    v
Unit 3: Core Business
    |
    v
Build & Test (tổng hợp)
```

---

## Execution Order

**Sequential (1 developer)**:  
Unit 1 → Unit 2 → Unit 3 → Unit 4 → Build & Test

**Parallel (2 developers)**:  
Dev A: Unit 1 → Unit 2 → Unit 3  
Dev B: (chờ Unit 2) → Unit 4 (song song với Unit 3)
