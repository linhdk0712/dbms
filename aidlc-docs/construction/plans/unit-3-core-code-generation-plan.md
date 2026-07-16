# Code Generation Plan — Unit 3: Core Business Logic

**Unit**: unit-3-core  
**Date**: 2026-07-11  
**Status**: Executing

---

## Unit Context

- **Workspace Root**: `d:\Apps\dbms\`
- **Backend path**: `d:\Apps\dbms\backend\`
- **Dependencies**: Unit 1 (Foundation) + Unit 2 (Auth)
- **Business Rules**: BR-001 ~ BR-010

---

## Steps

### Batch 1: DB Layer

- [x] Step 1: `db/requests.rs` — CRUD operations cho sql_requests
- [x] Step 2: `db/audit.rs` — insert audit log, fetch by request_id
- [x] Step 3: `db/databases.rs` — CRUD cho database_connections
- [x] Step 4: `db/permissions.rs` — grant/revoke permissions, check permission

### Batch 2: Executor

- [x] Step 5: `executor/mod.rs` — `DatabaseExecutor` trait, `ExecutionResult`, `ExecutorError`
- [x] Step 6: `executor/postgres.rs` — PostgreSQL executor impl
- [ ] Step 7: `executor/mysql.rs` — MySQL executor impl (skeleton)
- [x] Step 8: `executor/encryption.rs` — AES-256-GCM encrypt/decrypt credentials

### Batch 3: API Handlers (Requests + Approvals)

- [x] Step 9: `api/requests.rs` — POST create, GET list, GET detail, POST submit, POST cancel
- [x] Step 10: `api/approvals.rs` — GET pending list, GET detail, POST approve, POST reject

### Batch 4: API Handlers (Admin + Dashboard)

- [x] Step 11: `api/databases.rs` — POST create, GET list, GET detail, PATCH update, POST test-connection
- [x] Step 12: `api/users.rs` — GET list, GET detail, POST permissions, DELETE permissions
- [x] Step 13: `api/dashboard.rs` — GET dashboard stats

### Batch 5: WebSocket

- [x] Step 14: `ws/mod.rs` — WebSocket upgrade, subscribe broadcast, filter notifications

### Batch 6: Wire All Together

- [x] Step 15: Update `api/mod.rs` — wire all routes
- [x] Step 16: Update `main.rs` — add executor modules
- [x] Step 17: Build & verify
- [x] Step 18: Tests (41/41 pass)
- [x] Step 19: Code Summary Documentation

---

## File Output Summary

```
d:\Apps\dbms\backend\src\
├── db/
│   ├── mod.rs          (CẬP NHẬT)
│   ├── users.rs        (đã có)
│   ├── requests.rs     (MỚI)
│   ├── audit.rs        (MỚI)
│   ├── databases.rs    (MỚI)
│   └── permissions.rs  (MỚI)
├── executor/
│   ├── mod.rs          (MỚI)
│   ├── postgres.rs     (MỚI)
│   ├── mysql.rs        (MỚI — skeleton)
│   └── encryption.rs   (MỚI)
├── api/
│   ├── mod.rs          (CẬP NHẬT)
│   ├── auth.rs         (đã có)
│   ├── health.rs       (đã có)
│   ├── requests.rs     (MỚI)
│   ├── approvals.rs    (MỚI)
│   ├── databases.rs    (MỚI)
│   ├── users.rs        (MỚI)
│   └── dashboard.rs    (MỚI)
├── ws/
│   └── mod.rs          (MỚI)
└── main.rs             (CẬP NHẬT)
```
