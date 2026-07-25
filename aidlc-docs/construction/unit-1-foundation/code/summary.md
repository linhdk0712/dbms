# Code Summary — Unit 1: Foundation

**Unit**: unit-1-foundation  
**Ngày**: 2026-07-11  
**Trạng thái**: ✅ COMPLETED — build pass, 14/14 tests pass

---

## Tổng quan

Unit 1 cung cấp nền tảng cho toàn hệ thống SECS: database schema, domain models, application infrastructure (config, error, state), server startup, và health check endpoint. Frontend skeleton cũng được khởi tạo.

---

## Files đã tạo

### Backend — Infrastructure
| File | Mô tả |
|------|-------|
| `backend/Cargo.toml` | Dependencies (axum, sqlx, tokio, jsonwebtoken, aes-gcm, proptest...) |
| `backend/src/main.rs` | Startup sequence: config → tracing (dual: stdout JSON + file daily rotation) → pool → migrate → router → serve |
| `backend/src/config.rs` | `Config::from_env()` — fail fast nếu thiếu required vars |
| `backend/src/error.rs` | `AppError` + `IntoResponse` — 4xx cụ thể, 5xx generic |
| `backend/src/state.rs` | `AppState` — db pool, config, notification broadcast channel |

### Backend — Domain Models
| File | Nội dung |
|------|----------|
| `models/user.rs` | `User`, `Role` (Admin/Maker/Checker), `UserDbPermission` |
| `models/request.rs` | `SqlRequest`, `RequestStatus` + `can_transition_to()` state machine |
| `models/database.rs` | `DatabaseConnection`, `DbType`, `DatabaseConnectionResponse` (credential-safe) |
| `models/approval.rs` | `AuditLog`, `AuditAction` |
| `models/notification.rs` | `Notification`, `NotificationType` cho WebSocket |

### Backend — API
| File | Mô tả |
|------|-------|
| `api/mod.rs` | Router builder + CORS + security headers middleware (SECURITY-04) |
| `api/health.rs` | `GET /health` — `SELECT 1` kiểm tra DB pool |

### Database Migrations
| File | Bảng |
|------|------|
| `001_create_users.sql` | `users`, `user_roles` |
| `002_create_database_connections.sql` | `database_connections` |
| `003_create_sql_requests.sql` | `sql_requests` + CHECK four-eyes + rejection_reason |
| `004_create_audit_log.sql` | `audit_log` (append-only) |
| `005_create_user_db_permissions.sql` | `user_db_permissions` |
| `006_create_indexes.sql` | Indexes cho các query phổ biến |
| `007_gap_fixes.sql` | `request_approvals` (new), ALTER `database_connections` + `sql_requests` + `audit_log` |
| `008_expand_audit_actions.sql` | Expand audit_log action constraint: thêm ROLE_CHANGED, USER_STATUS_CHANGED, CREDENTIAL_ROTATED |
| `009_create_change_requests.sql` | `change_requests` (CR metadata cache từ ManageEngine), ALTER `sql_requests` thêm `cr_id` FK |

### Frontend Skeleton
| File | Mô tả |
|------|-------|
| `frontend/package.json` | Svelte + Vite + Tailwind + DaisyUI |
| `frontend/vite.config.js` | Dev proxy `/api`, `/health`, `/ws` → backend :3000 |
| `frontend/tailwind.config.js` | DaisyUI plugin, themes light/dark |
| `frontend/src/App.svelte` | Root component skeleton — health check badge |

---

## Business Rules đã enforce

| Rule | Cách enforce |
|------|--------------|
| BR-001 (4-eyes) | DB constraint `CHECK (approved_by IS NULL OR approved_by != created_by)` |
| BR-003 (state machine) | `RequestStatus::can_transition_to()` |
| BR-004 (rejection reason) | DB constraint `length(rejection_reason) >= 10` khi REJECTED |
| BR-005 (audit mandatory) | Bảng `audit_log` với FK tới request |
| BR-009 (credential encrypt) | `encrypted_credentials` + `#[serde(skip_serializing)]` |
| BR-010 (soft delete) | `users.is_active` thay vì DELETE |
| SECURITY-04 (HTTP headers) | Middleware `security_headers` inject CSP, HSTS, X-Frame-Options... |

---

## Kiểm thử

**14/14 tests pass**, gồm:
- Unit tests: state transitions, terminal states, error status codes, config fail-fast
- **PBT (proptest)**: 
  - `prop_terminal_states_have_no_outgoing_transitions` — terminal states không có outgoing edge
  - `prop_request_status_serde_roundtrip` — serialize/deserialize round-trip

---

## Ghi chú môi trường build

- Máy dev không có MSVC linker → dùng toolchain **`stable-x86_64-pc-windows-gnu`**
- Cài **WinLibs MinGW-w64 gcc** để compile `ring` (dependency của rustls/jsonwebtoken)
- Lệnh build: `cargo +stable-x86_64-pc-windows-gnu build`
- Lệnh test: `cargo +stable-x86_64-pc-windows-gnu test`

---

## Warnings hiện tại (dự kiến)

17 warning dạng `never used` / `dead_code` trên models, error variants, config fields. Đây là **bình thường** cho foundation unit — các thành phần này sẽ được wire vào ở Unit 2 (Auth) và Unit 3 (Core Business). Không suppress bằng `#[allow(dead_code)]` để tránh che giấu dead code thật sự sau này.

---

## Next: Unit 2 — Authentication

- `auth/jwt.rs`, `auth/oidc.rs`, `api/auth.rs`
- Auth middleware inject `Claims`
- RBAC extractors (`RequireMaker`, `RequireChecker`, `RequireAdmin`)
