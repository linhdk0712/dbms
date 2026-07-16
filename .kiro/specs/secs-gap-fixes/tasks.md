# Implementation Plan: SECS Gap Fixes

## Overview

Triển khai 16 lỗ hổng (gaps) trong hệ thống SECS theo chiến lược phân phase: Critical → High → Medium → Low. Mỗi task xây dựng dần trên task trước, đảm bảo không có code orphaned. Backend dùng Rust (Axum + SQLx), Frontend dùng Svelte.

## Tasks

- [x] 1. Database migration và model changes
  - [x] 1.1 Tạo migration 007_gap_fixes.sql
    - Thêm `approval_levels_required` vào `database_connections` (DEFAULT 1, CHECK 1-5)
    - Thêm `approval_levels_required` vào `sql_requests` (DEFAULT 1)
    - Tạo bảng `request_approvals` với unique constraint (request_id, approver_id)
    - Tạo index `idx_request_approvals_request_id`
    - ALTER `audit_log.request_id` DROP NOT NULL
    - _Requirements: 10.1, 10.3, 12.4_

  - [x] 1.2 Thêm model `RequestApproval` và cập nhật models
    - Tạo `backend/src/models/request_approval.rs` với struct `RequestApproval` (FromRow)
    - Thêm field `created_by: Option<Uuid>` vào `Notification` struct trong `models/notification.rs`
    - Thêm `AuditAction` variants: `Executing`, `RoleChanged`, `UserStatusChanged`, `CredentialRotated`
    - Thêm `MakerDatabaseResponse` DTO vào `models/database.rs`
    - Thêm `UpdateCredentialsBody` struct với validation
    - Cập nhật `models/mod.rs` export module mới
    - _Requirements: 1.3, 2.1, 9.1, 10.3, 11.1, 12.1_

  - [x] 1.3 Cập nhật `DashboardStats` DTO
    - Thêm fields: `pending_for_review`, `total_reviewed`, `total_approved_by_me`, `total_rejected_by_me` (Option<i64>)
    - Thêm `DashboardStats::empty()` constructor
    - _Requirements: 7.1, 7.2, 7.3, 7.5_

- [x] 2. Phase 1 — Critical: TLS Enforcement & Timeout Cap
  - [x] 2.1 PostgreSQL TLS enforcement
    - Sửa `backend/src/executor/postgres.rs`: thay `sslmode=prefer` → `sslmode=require` trong `build_connection_string`
    - Áp dụng cho cả `execute` và `test_connection`
    - _Requirements: 5.1, 5.2, 5.3, 5.4_

  - [ ]* 2.2 Write property test for PostgreSQL TLS (Property 6)
    - **Property 6: PostgreSQL connection string enforces TLS**
    - Tạo test trong `backend/tests/prop_executor.rs`
    - Generate random (host, port, database_name, username, password), verify `sslmode=require` present và `sslmode=prefer` absent
    - **Validates: Requirements 5.1, 5.2**

  - [x] 2.3 Oracle TLS enforcement
    - Sửa `backend/src/executor/oracle.rs`: thay EZConnect → TNS descriptor format với TCPS
    - Format: `(DESCRIPTION=(ADDRESS=(PROTOCOL=TCPS)(HOST=..)(PORT=..))(CONNECT_DATA=(SERVICE_NAME=..))(SECURITY=(SSL_SERVER_DN_MATCH=YES)))`
    - _Requirements: 6.1, 6.2, 6.3_

  - [ ]* 2.4 Write property test for Oracle TLS (Property 7)
    - **Property 7: Oracle connection string uses TCPS protocol**
    - Tạo test trong `backend/tests/prop_executor.rs`
    - Generate random (host, port, service_name), verify `PROTOCOL=TCPS` và `SSL_SERVER_DN_MATCH=YES`
    - **Validates: Requirements 6.1, 6.2**

  - [x] 2.5 Implement timeout cap trong `execute_sql_async`
    - Sửa executor logic: `effective_timeout = min(db_timeout_secs, config.sql_execution_timeout_secs)`
    - Thêm `sql_execution_timeout_secs` vào `Config` struct nếu chưa có
    - _Requirements: 15.1, 15.2, 15.3_

  - [ ]* 2.6 Write property test for timeout cap (Property 21)
    - **Property 21: Effective timeout is minimum of database and global cap**
    - Tạo test trong `backend/tests/prop_timeout.rs`
    - Generate random (db_timeout: u64, global_cap: u64), verify result == min(db_timeout, global_cap)
    - **Validates: Requirements 15.1, 15.3**

- [x] 3. Checkpoint — Phase 1 complete
  - Ensure all tests pass, ask the user if questions arise.

- [x] 4. Phase 2 — High: IP Extractor, Credential Rotation, EXECUTING Audit
  - [x] 4.1 Implement IP address extraction
    - Tạo `backend/src/api/ip_extractor.rs` với hàm `extract_client_ip`
    - Precedence: X-Forwarded-For (first) > X-Real-IP > ConnectInfo > "unknown"
    - Truncate tại 45 chars
    - Export từ `api/mod.rs`
    - _Requirements: 8.1, 8.2, 8.3_

  - [ ]* 4.2 Write property test for IP extraction (Property 9)
    - **Property 9: IP address extraction follows precedence**
    - Tạo test trong `backend/tests/prop_ip.rs`
    - Generate random header combinations, verify precedence order và max 45 chars
    - **Validates: Requirements 8.1, 8.2, 8.3**

  - [x] 4.3 Implement credential update endpoint
    - Thêm `PATCH /api/v1/databases/:id/credentials` handler trong `api/databases.rs`
    - Validate body (username 1-128, password 1-256, non-empty)
    - Encrypt credentials với AES-256-GCM
    - Ghi audit log "CREDENTIAL_ROTATED" (không log credential values)
    - Return 204 No Content
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

  - [ ]* 4.4 Write property tests for credentials (Properties 3, 4, 5)
    - **Property 3: Credential encryption round-trip**
    - **Property 4: Credential validation rejects invalid inputs**
    - **Property 5: Audit log never contains credential values**
    - Tạo tests trong `backend/tests/prop_encryption.rs` và `backend/tests/prop_validation.rs`
    - **Validates: Requirements 2.1, 2.2, 2.3, 2.6**

  - [x] 4.5 Implement EXECUTING audit state
    - Sửa `execute_sql_async`: insert audit entry "EXECUTING" TRƯỚC khi gọi executor
    - Actor_id = approver's user_id
    - Truyền IP address đã capture từ approval action
    - _Requirements: 11.1, 11.2, 11.3_

  - [ ]* 4.6 Write property test for EXECUTING audit (Property 16)
    - **Property 16: EXECUTING audit entry precedes execution result**
    - Tạo test trong `backend/tests/prop_audit.rs`
    - Verify audit "EXECUTING" timestamp <= "EXECUTED"/"FAILED" timestamp
    - **Validates: Requirements 11.1, 11.3**

  - [x] 4.7 Implement Maker database list endpoint
    - Thêm `GET /api/v1/databases/my-permissions` handler trong `api/databases.rs`
    - Query JOIN user_db_permissions, filter is_active=true, ORDER BY name ASC, LIMIT 100
    - Return `Vec<MakerDatabaseResponse>` (không chứa credentials, timeout, timestamps)
    - Thêm DB query function trong `db/databases.rs`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6, 1.7_

  - [ ]* 4.8 Write property tests for Maker DB list (Properties 1, 2)
    - **Property 1: Database permission filter returns only permitted active databases**
    - **Property 2: Response DTO excludes sensitive fields**
    - Tạo tests trong `backend/tests/prop_databases.rs`
    - **Validates: Requirements 1.1, 1.3, 1.7**

- [x] 5. Checkpoint — Phase 2 complete
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Phase 2 (cont.) — Multi-level Approval & Role Audit
  - [x] 6.1 Implement multi-level approval logic
    - Thêm DB functions trong `db/requests.rs`: `insert_approval`, `count_approvals`, `has_already_approved`
    - Sửa approve handler trong `api/approvals.rs`:
      - Check duplicate approval (409)
      - Insert vào `request_approvals`
      - Count approvals vs `sql_requests.approval_levels_required`
      - Transition to APPROVED chỉ khi count == required
      - Broadcast partial approval notification nếu count < required
    - Snapshot `approval_levels_required` từ database_connections vào sql_requests khi submit
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.8_

  - [ ]* 6.2 Write property tests for multi-level approval (Properties 13, 14, 15)
    - **Property 13: Multi-level approval state machine correctness**
    - **Property 14: Four-eyes principle per approval level**
    - **Property 15: Approval threshold snapshot isolation**
    - Tạo tests trong `backend/tests/prop_approval.rs`
    - **Validates: Requirements 10.2, 10.4, 10.5, 10.6, 10.7, 10.8**

  - [x] 6.3 Implement role change audit logging
    - Sửa `update_user` handler trong `api/users.rs`
    - Lấy old_roles và old_active TRƯỚC khi update
    - Ghi "ROLE_CHANGED" audit nếu roles thực sự thay đổi (detail chứa old + new)
    - Ghi "USER_STATUS_CHANGED" audit nếu is_active thay đổi
    - Tạo entries riêng biệt cho mỗi thay đổi
    - Thêm `db::audit::insert_admin_action` function (request_id = NULL)
    - Không ghi audit nếu no-op (submitted values == current values)
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 12.6_

  - [ ]* 6.4 Write property tests for role audit (Properties 17, 18)
    - **Property 17: Role change audit captures old and new state**
    - **Property 18: No-op update produces no audit entry**
    - Tạo tests trong `backend/tests/prop_audit.rs`
    - **Validates: Requirements 12.1, 12.6**

- [x] 7. Checkpoint — Multi-level approval & role audit complete
  - Ensure all tests pass, ask the user if questions arise.

- [x] 8. Phase 3 — Medium: Dashboard, WS, Pending List, OIDC
  - [x] 8.1 Implement role-aware dashboard
    - Sửa `get_stats` handler trong `api/dashboard.rs`
    - Implement `determine_effective_role` (Admin > Checker > Maker)
    - Implement `get_admin_stats` (global counts)
    - Implement `get_checker_stats` (pending excluding own, approval history)
    - Implement `get_maker_stats` (own requests only)
    - Return empty stats khi no role
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

  - [ ]* 8.2 Write property test for dashboard stats (Property 8)
    - **Property 8: Role-based dashboard stats correctness**
    - Tạo test trong `backend/tests/prop_dashboard.rs`
    - Generate random request sets, verify per-role count logic
    - **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5**

  - [x] 8.3 Implement WebSocket role-based filtering
    - Sửa `backend/src/ws/mod.rs`:
      - Lưu `claims` khi upgrade connection
      - Implement `should_forward` function
      - Admin → always forward
      - Maker → forward nếu created_by == user_id
      - Checker → forward nếu NEW_REQUEST hoặc đã review trước đó
      - Discard notification nếu missing created_by cho non-Admin
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6_

  - [ ]* 8.4 Write property tests for WS filtering (Properties 10, 11, 12)
    - **Property 10: WebSocket Maker filter correctness**
    - **Property 11: WebSocket Checker filter correctness**
    - **Property 12: WebSocket Admin passthrough**
    - Tạo tests trong `backend/tests/prop_ws.rs`
    - **Validates: Requirements 9.2, 9.3, 9.5**

  - [x] 8.5 Implement pending list exclude Checker's own
    - Sửa `db::requests::list_pending` nhận `checker_user_id` parameter
    - Thêm `WHERE created_by != $1` vào query
    - Cập nhật handler gọi hàm này truyền claims.user_id
    - _Requirements: 13.1, 13.2_

  - [ ]* 8.6 Write property test for pending list exclusion (Property 19)
    - **Property 19: Pending list excludes Checker's own requests**
    - Tạo test trong `backend/tests/prop_requests.rs`
    - Generate random requests + checker_id, verify exclusion
    - **Validates: Requirements 13.1**

  - [x] 8.7 Implement OIDC session TTL & cleanup
    - Thêm config fields: `oidc_session_ttl_secs` (default 300, range 60-900), `oidc_cleanup_interval_secs` (default 60, range 10-300)
    - Thay đổi session store: `HashMap<String, TimestampedSession>` với `created_at: Instant`
    - Implement background task `oidc_session_cleanup` (spawn trong main.rs)
    - Sửa OIDC callback: check session existence, return 401 nếu expired/missing
    - _Requirements: 14.1, 14.2, 14.3, 14.4_

  - [ ]* 8.8 Write property test for OIDC cleanup (Property 20)
    - **Property 20: OIDC session cleanup correctness**
    - Tạo test trong `backend/tests/prop_oidc.rs`
    - Generate random sessions + ages, verify cleanup logic
    - **Validates: Requirements 14.2, 14.3**

- [x] 9. Checkpoint — Phase 3 backend complete
  - Ensure all tests pass, ask the user if questions arise.

- [x] 10. Phase 4 — Frontend Changes
  - [x] 10.1 Cập nhật NewRequest page dùng Maker endpoint
    - Sửa `frontend/src/pages/maker/NewRequest.svelte`
    - Thay đổi API call từ `GET /api/v1/databases` → `GET /api/v1/databases/my-permissions`
    - Cập nhật `frontend/src/api/endpoints.js` thêm endpoint mới
    - _Requirements: 1.5_

  - [x] 10.2 Implement Role Management UI trong Users page
    - Sửa `frontend/src/pages/admin/Users.svelte`
    - Thêm role checkboxes panel (ADMIN, MAKER, CHECKER)
    - Call `PATCH /api/v1/users/:id` với updated roles array khi save
    - Hiển thị success notification / error message
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

  - [x] 10.3 Implement User Activate/Deactivate UI
    - Thêm toggle button activate/deactivate trong Users page
    - Confirmation dialog trước khi deactivate
    - Self-deactivation check (so sánh user.id với current auth user → hiện error)
    - Call `PATCH /api/v1/users/:id` với `is_active: false/true`
    - Update badge state sau khi thành công
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

  - [x] 10.4 Implement Pending List filtering & search
    - Sửa `frontend/src/pages/checker/PendingList.svelte`
    - Thêm filter dropdown theo database (client-side)
    - Thêm text search input (filter by description, case-insensitive)
    - Thêm sort options (newest first / oldest first)
    - Hiển thị count: "Showing X of Y"
    - Persist filter/sort preferences trong sessionStorage
    - _Requirements: 16.1, 16.2, 16.3, 16.4, 16.5_

  - [ ]* 10.5 Write unit tests for frontend filtering logic
    - Test text search filter (case-insensitive match)
    - Test sort order (newest/oldest)
    - Test database filter
    - Test count display
    - **Validates: Requirements 16.1, 16.2, 16.3, 16.4**

  - [x] 10.6 Hiển thị approval progress cho multi-level requests
    - Sửa PendingList và ReviewDetail pages
    - Hiển thị "X/Y approvals" cho requests với approval_levels_required > 1
    - _Requirements: 10.9_

- [x] 11. Final checkpoint — All features complete
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation after each phase
- Property tests validate universal correctness properties defined in the design
- Unit tests validate specific examples and edge cases
- Backend implementation (Rust) precedes frontend (Svelte) to ensure API stability
- Migration (task 1.1) must run before any backend logic that depends on new columns/tables
- Frontend tasks can be parallelized among themselves since they target different pages

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2", "1.3"] },
    { "id": 2, "tasks": ["2.1", "2.3", "2.5"] },
    { "id": 3, "tasks": ["2.2", "2.4", "2.6", "4.1"] },
    { "id": 4, "tasks": ["4.2", "4.3", "4.5", "4.7"] },
    { "id": 5, "tasks": ["4.4", "4.6", "4.8", "6.1"] },
    { "id": 6, "tasks": ["6.2", "6.3"] },
    { "id": 7, "tasks": ["6.4", "8.1", "8.5", "8.7"] },
    { "id": 8, "tasks": ["8.2", "8.3", "8.6", "8.8"] },
    { "id": 9, "tasks": ["8.4"] },
    { "id": 10, "tasks": ["10.1", "10.2", "10.3", "10.4", "10.6"] },
    { "id": 11, "tasks": ["10.5"] }
  ]
}
```
