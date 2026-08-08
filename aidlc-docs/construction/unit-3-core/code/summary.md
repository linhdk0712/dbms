# Code Summary — Unit 3: Core Business Logic

**Unit**: unit-3-core  
**Ngày**: 2026-07-11  
**Trạng thái**: ✅ COMPLETED — build pass, 41/41 tests pass

---

## Tổng quan

Unit 3 triển khai toàn bộ business logic: DB access layer, SQL executor (PostgreSQL), credential encryption, tất cả API handlers (Maker, Checker, Admin, Dashboard), WebSocket realtime notification, và AI SQL analysis module (LLM-powered). Đây là unit lớn nhất — chứa toàn bộ Maker-Checker workflow.

---

## Files đã tạo

### Backend — DB Layer
| File | Mô tả |
|------|-------|
| `db/mod.rs` | Export tất cả sub-modules |
| `db/requests.rs` | CRUD cho sql_requests: create, get, list, submit, approve, reject, cancel, mark_executed/failed, `save_validation_result()` (persist JSONB validation result để reload không cần validate lại). `list_pending(pool, checker_user_id)` excludes checker's own requests (4-eyes, Req 13.1/13.2) |
| `db/audit.rs` | Insert audit log (append-only), fetch by request_id/actor_id. `insert_admin_action()` cho admin actions không gắn request (request_id = NULL). Actions: CREATED, SUBMITTED, APPROVED, REJECTED, EXECUTED, FAILED, CANCELLED, EXECUTING, ROLE_CHANGED, USER_STATUS_CHANGED, CREDENTIAL_ROTATED |
| `db/databases.rs` | CRUD cho database_connections: create, get, list, update, update_credentials |
| `db/permissions.rs` | check_permission, grant, revoke, list_user_permissions, list_database_users |
| `db/dashboard.rs` | Dashboard stats queries: `get_admin_stats()` (global counts), `count_pending_for_review(checker_id)`, `count_approved_by_checker(checker_id)`, `count_rejected_by_checker(checker_id)`, `get_maker_stats(maker_id)` (filtered counts) |
| `db/change_requests.rs` | CRUD cho bảng change_requests (CR integration): create (upsert on cr_number), get_by_id, get_by_cr_number, list_all (paginated), count_all, update_from_sync (refresh metadata từ ManageEngine), delete, count_requests (đếm sql_requests gắn CR) |
| `db/approver_units.rs` | CRUD cho approver_units: list_all, list_active, create, update, delete. User-Unit assignments: get_user_units (names), get_user_unit_details (full), assign_user (ON CONFLICT DO NOTHING), unassign_user |

### Backend — AI Module
| File | Mô tả |
|------|-------|
| `ai/mod.rs` | Module root: exports `llm_client`, `prompt_builder`, `schema_introspector`. Định nghĩa `AiAnalysisResponse`, `Recommendation`, `SecurityIssue`, `CostEstimation` response models. `AiError` enum (ServiceUnavailable, Timeout, ParseError, RateLimited, Disabled, SchemaError). `AiConfig` struct + `AiProvider` enum (OpenAI / AwsBedrock). `AiRateLimiter` struct — sliding window rate limiter per user (max N requests/minute, dùng `Arc<Mutex<HashMap<Uuid, Vec<Instant>>>>`). `AiCache` struct — in-memory cache kết quả AI analysis per request (key = request_id + hash(sql_content), TTL-based expiry, auto-cleanup khi >500 entries). Config load từ env vars (`AI_ENABLED`, `AI_PROVIDER`, `AI_OPENAI_API_KEY`, `AI_OPENAI_MODEL`, `AI_OPENAI_BASE_URL`, `AI_BEDROCK_REGION`, `AI_BEDROCK_MODEL_ID`, `AI_ANALYSIS_TIMEOUT_SECS`, `AI_MAX_TOKENS`, `AI_RATE_LIMIT_PER_USER`) |
| `ai/llm_client.rs` | `LlmClient` struct: HTTP client cho OpenAI Chat Completions API. `analyze(system_prompt, user_prompt)` → `Result<String, AiError>`. JSON response format, low temperature (0.1). AWS Bedrock placeholder. |
| `ai/prompt_builder.rs` | Xây dựng prompt tối ưu cho LLM: `build_system_prompt()` (role + JSON output format), `build_user_prompt(sql, schema, db_type)` (schema context + sanitized SQL + analysis instructions), `sanitize_sql_literals(sql)` (thay string literals bằng `'***'` để không leak data nhạy cảm). 5 unit tests cho sanitization, 3 tests cho format_row_count, 2 tests cho prompt building. |
| `ai/schema_introspector.rs` | Introspect schema từ database đích: `extract_table_names(sql)` (regex-based table name extraction), `introspect_postgres()` (information_schema + pg_class + pg_index), `introspect_mysql()` (information_schema.COLUMNS + STATISTICS), `introspect_oracle()` (placeholder), `introspect_mongodb()` (schema-less, returns empty). Models: `SchemaContext`, `TableInfo`, `ColumnInfo`, `IndexInfo`. 9 unit tests cho table extraction. |

### Backend — Executor
| File | Mô tả |
|------|-------|
| `executor/mod.rs` | `DatabaseExecutor` trait, `ExecutionResult`, `ValidationResult`, `ExecutorError`, `DbCredentials`, `execute_sql()`, `test_db_connection()`, `validate_sql()` (EXPLAIN-based validation cho PostgreSQL/MySQL/Oracle/MongoDB). Kết quả validation được persist vào `sql_requests.validation_result` (JSONB) — migration 013. |
| `executor/postgres.rs` | PostgreSQL executor: connect, execute multi-statement in transaction, test_connection |
| `executor/mysql.rs` | MySQL executor: connect, execute multi-statement in transaction, test_connection |
| `executor/oracle.rs` | Oracle executor: TCPS connection, spawn_blocking, test_connection |
| `executor/mongodb.rs` | MongoDB executor: TLS connection, JSON commands, session TX |
| `executor/sql_analyzer.rs` | DDL detection (Warning/Danger severity) |
| `executor/encryption.rs` | AES-256-GCM encrypt/decrypt credentials (6 unit tests, PBT roundtrip) |

### Backend — API Handlers
| File | Mô tả |
|------|-------|
| `api/requests.rs` | Maker: POST create (inline CR resolution — parse `cr_url`, lookup/create CR, pass `cr_id`), GET list, GET detail, POST submit, POST cancel |
| `api/analysis.rs` | AI SQL Analysis: POST analyze — LLM-powered phân tích SQL (security issues, recommendations, cost estimation). Access: Maker (own requests) + Checker (PENDING requests) |
| `api/approvals.rs` | Checker: GET pending, GET detail, POST approve (+ **synchronous** SQL exec — result returned in response), POST reject. Response includes `database_name`, `approval_levels_required`, `approval_count` for multi-level approval progress tracking, and `validation_result` (persisted EXPLAIN-based validation, if available). Approve response includes `execution_result` with `success`/`rows_affected` or `error`. |
| `api/change_requests.rs` | CR Integration: POST create (parse URL + fetch ManageEngine metadata), GET list (paginated), GET detail, POST sync (re-fetch metadata), DELETE (Admin only). Includes `check_maintenance_window()` helper for enforce time window trước khi execute SQL. |
| `api/databases.rs` | Admin: POST create, GET list, GET detail, PATCH update, POST test-new-connection (raw params), POST test-connection (saved DB) |
| `api/users.rs` | Admin: GET list, GET detail, PATCH update (active status + roles), GET/POST/DELETE permissions |
| `api/dashboard.rs` | GET dashboard stats — thin handler delegating to `db::dashboard` module |
| `api/schema_browser.rs` | Schema Browser: GET tables (list tables/collections from target DB), GET columns (list columns for a table). Supports PostgreSQL, MySQL, Oracle (placeholder), MongoDB. Permission-gated per database. |
| `api/approver_units.rs` | Approver Units: GET list (all authenticated), POST create (Admin), PUT update (Admin), DELETE (Admin). User assignments: GET user units, POST assign (Admin), DELETE unassign (Admin). Validation: name 1-50 chars, duplicate check. |

### Backend — WebSocket
| File | Mô tả |
|------|-------|
| `ws/mod.rs` | WebSocket upgrade, JWT auth from query param, broadcast → client forwarding |

### Backend — Rate Limiting
| File | Mô tả |
|------|-------|
| `auth/global_rate_limit.rs` | `GlobalRateLimiter` — sliding window counter per user_id (200 req/min/user). Áp dụng cho tất cả protected routes sau auth middleware. In-memory `HashMap<Uuid, Vec<Instant>>`, auto-cleanup khi >5000 entries. |

### Backend — Updated
| File | Thay đổi |
|------|----------|
| `main.rs` | Thêm `mod executor`, `mod ws` |
| `api/mod.rs` | Wire tất cả routes (public + protected), includes approver_units module. Global rate limiter applied via `Extension(GlobalRateLimiter)` + `from_fn(global_rate_limit)` trên protected routes |
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
| POST | `/api/v1/requests/:id/analyze` | Maker/Checker | AI SQL analysis (LLM) |
| POST | `/api/v1/change-requests` | Maker/Checker | create CR from URL |
| GET | `/api/v1/change-requests` | Maker/Checker | list CRs (paginated) |
| GET | `/api/v1/change-requests/:id` | Maker/Checker | CR detail + request count |
| POST | `/api/v1/change-requests/:id/sync` | Maker/Checker | re-sync metadata from ManageEngine |
| DELETE | `/api/v1/change-requests/:id` | Admin | delete CR (unlinks requests) |
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
| GET | `/api/v1/databases/:id/tables` | Any (perm) | list tables/collections from target DB |
| GET | `/api/v1/databases/:id/columns` | Any (perm) | list columns for a table in target DB |
| GET | `/api/v1/approver-units` | Any | list all approver units |
| POST | `/api/v1/approver-units` | Admin | create approver unit |
| PUT | `/api/v1/approver-units/:id` | Admin | update approver unit |
| DELETE | `/api/v1/approver-units/:id` | Admin | delete approver unit |
| GET | `/api/v1/users/:user_id/approver-units` | Any | get user's approver units |
| POST | `/api/v1/users/:user_id/approver-units` | Admin | assign unit to user |
| DELETE | `/api/v1/users/:user_id/approver-units/:unit_id` | Admin | unassign unit from user |

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

**60/60 tests pass**, gồm:
- Unit 1 tests: 14 (state machine, PBT, error codes, config)
- Unit 2 tests: 21 (JWT, middleware, extractors)
- Unit 3 tests: 25 (encryption: 6, prompt_builder: 10, schema_introspector: 9)

---

## SQL Execution Architecture

```
approve() handler
    ├── Validate 4-eyes + state
    ├── Insert approval record
    ├── Check total approvals vs required
    ├── If fully approved:
    │     ├── Update status → APPROVED (sync)
    │     ├── Audit log APPROVED (sync)
    │     ├── Broadcast APPROVED notification
    │     └── execute_sql_sync() — blocks until complete
    │           ├── Fetch database_connection
    │           ├── Decrypt credentials (AES-256-GCM)
    │           ├── Audit log EXECUTING
    │           ├── execute_sql(sql, timeout)
    │           │     ├── Connect with TLS
    │           │     ├── BEGIN TRANSACTION
    │           │     ├── Execute each statement
    │           │     ├── COMMIT (or ROLLBACK on error)
    │           │     └── Close connection
    │           ├── mark_executed() or mark_failed()
    │           ├── Audit log EXECUTED/FAILED
    │           ├── Broadcast notification
    │           └── Return Ok(result) or Err(error_msg)
    └── Response 200 with execution_result included
```

**Note**: SQL execution là synchronous — HTTP response chờ cho đến khi execution
hoàn tất và trả kết quả trực tiếp (`execution_result.success` + `rows_affected`
hoặc `error`). Caller biết ngay trạng thái mà không cần poll.

---

## SQL Validation Architecture (EXPLAIN-based)

```
validate_sql(db_type, sql, credentials, host, port, db_name, schema, timeout)
    ├── PostgreSQL: EXPLAIN <statement>
    │     ├── Connect (sslmode=require, DISABLE_TARGET_DB_TLS env override)
    │     ├── Split multi-statement by ';'
    │     ├── EXPLAIN each statement
    │     ├── Analyze plan: detect Seq Scan, high cost (>10000)
    │     └── Return ValidationResult { valid, execution_plan, warnings, error }
    ├── MySQL: EXPLAIN <statement>
    │     ├── Connect (ssl-mode=REQUIRED, DISABLE_TARGET_DB_TLS env override)
    │     ├── Split multi-statement
    │     ├── Parse EXPLAIN output (select_type, table, type, key, rows, Extra)
    │     ├── Detect full table scan (type=ALL), high row estimates (>100K)
    │     └── Return ValidationResult
    ├── Oracle: EXPLAIN PLAN FOR <statement>
    │     ├── Connect via TCPS (spawn_blocking)
    │     ├── Split multi-statement
    │     ├── EXPLAIN PLAN SET STATEMENT_ID + query PLAN_TABLE
    │     ├── Detect TABLE ACCESS FULL, high cost (>10000)
    │     └── Return ValidationResult (requires feature 'oracle-support')
    └── MongoDB: JSON structure validation only
          ├── Parse as JSON object or array
          ├── Validate each element is a JSON object
          └── Return ValidationResult (no schema check — schema-less)
```

**Mục đích**: Cho phép Checker hoặc hệ thống kiểm tra SQL trước khi thực thi —
phát hiện lỗi syntax, bảng không tồn tại, và cảnh báo performance (seq scan, high cost)
mà không thay đổi dữ liệu trên database đích.

---

## Next: Unit 4 — Frontend

- Svelte app đầy đủ pages
- API client module (single source of truth)
- WebSocket real-time notifications
- DaisyUI components
