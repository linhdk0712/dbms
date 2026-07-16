# Code Summary — Unit 3: Core Business Logic

**Unit**: unit-3-core  
**Ngày**: 2026-07-11  
**Trạng thái**: ✅ COMPLETED — build pass, 41/41 tests pass

---

## Tổng quan

Unit 3 triển khai toàn bộ business logic: DB access layer, SQL executor (PostgreSQL), credential encryption, tất cả API handlers (Maker, Checker, Admin, Dashboard), và WebSocket realtime notification. Đây là unit lớn nhất — chứa toàn bộ Maker-Checker workflow.

---

## Files đã tạo

### Backend — DB Layer
| File | Mô tả |
|------|-------|
| `db/mod.rs` | Export tất cả sub-modules |
| `db/requests.rs` | CRUD cho sql_requests: create, get, list, submit, approve, reject, cancel, mark_executed/failed. `list_pending(pool, checker_user_id)` excludes checker's own requests (4-eyes, Req 13.1/13.2) |
| `db/audit.rs` | Insert audit log (append-only), fetch by request_id/actor_id. `insert_admin_action()` cho admin actions không gắn request (request_id = NULL). Actions: CREATED, SUBMITTED, APPROVED, REJECTED, EXECUTED, FAILED, CANCELLED, EXECUTING, ROLE_CHANGED, USER_STATUS_CHANGED, CREDENTIAL_ROTATED |
| `db/databases.rs` | CRUD cho database_connections: create, get, list, update, update_credentials |
| `db/permissions.rs` | check_permission, grant, revoke, list_user_permissions, list_database_users |

### Backend — Executor
| File | Mô tả |
|------|-------|
| `executor/mod.rs` | `DatabaseExecutor` trait, `ExecutionResult`, `ExecutorError`, `DbCredentials` |
| `executor/postgres.rs` | PostgreSQL executor: connect, execute multi-statement in transaction, test_connection |
| `executor/encryption.rs` | AES-256-GCM encrypt/decrypt credentials (6 unit tests, PBT roundtrip) |

### Backend — API Handlers
| File | Mô tả |
|------|-------|
| `api/requests.rs` | Maker: POST create, GET list, GET detail, POST submit, POST cancel |
| `api/approvals.rs` | Checker: GET pending, GET detail, POST approve (+ async SQL exec), POST reject. Response includes `database_name`, `approval_levels_required`, `approval_count` for multi-level approval progress tracking. |
| `api/databases.rs` | Admin: POST create, GET list, GET detail, PATCH update, POST test-new-connection (raw params), POST test-connection (saved DB) |
| `api/users.rs` | Admin: GET list, GET/POST/DELETE permissions |
| `api/dashboard.rs` | GET dashboard stats (aggregate counts) |

### Backend — WebSocket
| File | Mô tả |
|------|-------|
| `ws/mod.rs` | WebSocket upgrade, JWT auth from query param, broadcast → client forwarding |

### Backend — Updated
| File | Thay đổi |
|------|----------|
| `main.rs` | Thêm `mod executor`, `mod ws` |
| `api/mod.rs` | Wire tất cả routes (public + protected), ~20 endpoints |
| `Cargo.toml` | Thêm `urlencoding` |

---

## API Endpoints Hoàn chỉnh

### Public (no auth)
| Method | Path | Handler |
|--------|------|---------|
| GET | `/health` | health_check |
| GET | `/auth/login` | OIDC redirect |
| GET | `/auth/callback` | OIDC callback |
| GET | `/ws?token=JWT` | WebSocket upgrade |

### Protected (Bearer token)
| Method | Path | Role | Handler |
|--------|------|------|---------|
| GET | `/auth/me` | Any | current user info |
| POST | `/auth/logout` | Any | logout (stateless) |
| GET | `/api/v1/dashboard` | Any | aggregate stats |
| POST | `/api/v1/requests` | Maker | create request |
| GET | `/api/v1/requests` | Maker | list my requests |
| GET | `/api/v1/requests/:id` | Maker | request detail |
| POST | `/api/v1/requests/:id/submit` | Maker | submit DRAFT→PENDING |
| POST | `/api/v1/requests/:id/cancel` | Maker | cancel PENDING→CANCELLED |
| GET | `/api/v1/approvals/pending` | Checker | pending list |
| GET | `/api/v1/approvals/:id` | Checker | approval detail |
| POST | `/api/v1/approvals/:id/approve` | Checker | approve + execute |
| POST | `/api/v1/approvals/:id/reject` | Checker | reject |
| POST | `/api/v1/databases` | Admin | create DB connection |
| GET | `/api/v1/databases` | Admin | list DB connections |
| GET | `/api/v1/databases/:id` | Admin | DB detail |
| PATCH | `/api/v1/databases/:id` | Admin | update DB |
| POST | `/api/v1/databases/test-new` | Admin | test connection (raw params, chưa lưu) |
| POST | `/api/v1/databases/:id/test` | Admin | test connection (đã lưu) |
| GET | `/api/v1/users` | Admin | list users |
| GET | `/api/v1/users/:id/permissions` | Admin | user permissions |
| POST | `/api/v1/users/:id/permissions` | Admin | grant permission |
| DELETE | `/api/v1/users/:id/permissions/:db_id` | Admin | revoke permission |

---

## Business Rules Enforced

| Rule | Enforcement |
|------|-------------|
| BR-001 (4-eyes) | `approve()`: check `created_by != claims.user_id`; `list_pending()`: exclude checker's own requests |
| BR-002 (immutable) | Handled by state machine (no update route for PENDING) |
| BR-003 (state machine) | `can_transition_to()` in submit/cancel/approve/reject |
| BR-004 (rejection reason) | `reject()`: validate `rejection_reason.len() >= 10` |
| BR-005 (audit mandatory) | Audit log INSERT in every state change handler |
| BR-006 (permission check) | `create_request()`: check_permission before insert |
| BR-007 (transaction) | `PgExecutor::execute()`: multi-stmt in BEGIN/COMMIT |
| BR-008 (timeout) | `tokio::time::timeout` wrapping execute + connect |
| BR-009 (encrypt credentials) | `encryption.rs`: AES-256-GCM with random nonce |
| BR-010 (soft delete) | User model uses `is_active`, callback checks active |

---

## Kiểm thử

**41/41 tests pass**, gồm:
- Unit 1 tests: 14 (state machine, PBT, error codes, config)
- Unit 2 tests: 21 (JWT, middleware, extractors)
- Unit 3 tests: 6 (encryption roundtrip, nonce uniqueness, wrong key, tampered data, unicode, empty)

---

## SQL Execution Architecture

```
approve() handler
    ├── Validate 4-eyes + state
    ├── Update status → APPROVED (sync)
    ├── Audit log APPROVED (sync)
    ├── Response 200 immediately
    └── tokio::spawn → execute_sql_async()
                           ├── Fetch database_connection
                           ├── Decrypt credentials (AES-256-GCM)
                           ├── PgExecutor::execute(sql, timeout)
                           │     ├── Connect with TLS
                           │     ├── BEGIN TRANSACTION
                           │     ├── Execute each statement
                           │     ├── COMMIT (or ROLLBACK on error)
                           │     └── Close connection
                           ├── mark_executed() or mark_failed()
                           ├── Audit log EXECUTED/FAILED
                           └── Broadcast notification
```

---

## Next: Unit 4 — Frontend

- Svelte app đầy đủ pages
- API client module (single source of truth)
- WebSocket real-time notifications
- DaisyUI components
