# Requirements Document

## Introduction

Tài liệu yêu cầu cho việc sửa 16 lỗ hổng (gaps) được phát hiện trong hệ thống SECS (SQL Execution Control System) thông qua code review so sánh implementation với business requirements. Các gaps được phân loại theo mức độ ưu tiên: Critical, High, Medium, Low.

## Glossary

- **SECS**: SQL Execution Control System — hệ thống kiểm soát thực thi SQL trên production
- **Maker**: Người dùng có vai trò tạo và gửi SQL request
- **Checker**: Người dùng có vai trò review và approve/reject SQL request
- **Admin**: Người dùng quản trị hệ thống, quản lý users, databases, cấu hình
- **Request**: Một yêu cầu thực thi SQL (SqlRequest) trong hệ thống
- **Four_Eyes_Principle**: Quy tắc yêu cầu Maker và Checker của cùng một request phải là 2 người khác nhau
- **API_Handler**: HTTP request handler trong backend Axum
- **Database_Connection**: Cấu hình kết nối đến database đích (Oracle, PostgreSQL, MySQL, MongoDB)
- **Audit_Log**: Bản ghi lịch sử mọi thao tác thay đổi trạng thái trong hệ thống
- **WebSocket_Channel**: Kênh broadcast realtime notification tới client
- **OIDC_Session_Store**: In-memory HashMap lưu session tạm thời khi user đang thực hiện OIDC login
- **Dashboard_API**: Endpoint trả về thống kê tổng hợp cho người dùng
- **Credential_Rotation**: Hành động thay đổi username/password kết nối database đích
- **TLS**: Transport Layer Security — mã hóa kết nối mạng
- **IP_Address**: Địa chỉ IP của client thực hiện hành động
- **Pending_List**: Danh sách SQL requests đang chờ Checker duyệt

## Requirements

### Requirement 1: Maker-Accessible Database List Endpoint

**User Story:** As a Maker, I want to see the list of databases I have permission on when creating a new request, so that I can select the target database without getting a 403 error.

#### Acceptance Criteria

1. WHEN a Maker calls the database list endpoint, THE SECS SHALL return only Database_Connection records where `is_active = true` AND the calling user has a corresponding entry in user_db_permissions
2. THE SECS SHALL expose a new endpoint `GET /api/v1/databases/my-permissions` accessible to any authenticated user with Maker role
3. THE SECS SHALL return database metadata (id, name, db_type, host, port, database_name, schema_name) without exposing encrypted_credentials, timeout_secs, or internal timestamps in the response
4. WHEN a Maker has no database permissions, THE SECS SHALL return an empty JSON array with HTTP 200
5. THE Frontend SHALL call the `GET /api/v1/databases/my-permissions` endpoint instead of the Admin-only `GET /api/v1/databases` endpoint in the NewRequest page database dropdown
6. IF an unauthenticated user or a user without Maker role calls `GET /api/v1/databases/my-permissions`, THEN THE SECS SHALL return HTTP 403 with an error message indicating insufficient permissions
7. THE SECS SHALL return the database list ordered by database name ascending, with a maximum of 100 entries per response

---

### Requirement 2: Credential Update API

**User Story:** As an Admin, I want to rotate database connection credentials, so that I can maintain security without recreating the entire database connection.

#### Acceptance Criteria

1. WHEN an Admin sends new credentials via `PATCH /api/v1/databases/:id/credentials`, THE SECS SHALL encrypt the new credentials using AES-256-GCM and update the stored encrypted_credentials field
2. IF the credential update request is missing the username field, the password field, or either field is an empty string, THEN THE SECS SHALL reject the request with HTTP 400 and return an error message indicating which validation constraint was violated
3. THE SECS SHALL enforce that username is between 1 and 128 characters and password is between 1 and 256 characters in the credential update request
4. THE SECS SHALL return HTTP 204 No Content on successful credential rotation without exposing the new credentials in the response body
5. IF the database connection does not exist for the given :id, THEN THE SECS SHALL return HTTP 404
6. THE SECS SHALL record the credential rotation event in the system audit log including the database connection ID, the actor (Admin user ID), and timestamp, without logging the credential values
7. IF a non-Admin user sends a request to `PATCH /api/v1/databases/:id/credentials`, THEN THE SECS SHALL return HTTP 403

---

### Requirement 3: Role Management UI

**User Story:** As an Admin, I want to view and modify user roles from the Users management page, so that I can assign or revoke Maker/Checker/Admin roles through the UI.

#### Acceptance Criteria

1. THE Frontend SHALL display the current roles of each user in the Users list view
2. WHEN an Admin selects a user, THE Frontend SHALL display an editable role assignment panel with checkboxes for ADMIN, MAKER, and CHECKER roles
3. WHEN an Admin saves role changes, THE Frontend SHALL call `PATCH /api/v1/users/:id` with the updated roles array
4. IF the role update succeeds, THEN THE Frontend SHALL refresh the user list and display a success notification
5. IF the role update fails due to validation (e.g., removing own ADMIN role), THEN THE Frontend SHALL display the error message from the API

---

### Requirement 4: User Activate/Deactivate UI

**User Story:** As an Admin, I want to activate or deactivate users from the UI, so that I can manage user access without direct API calls.

#### Acceptance Criteria

1. THE Frontend SHALL display a toggle button or action for activate/deactivate next to each user in the Users list
2. WHEN an Admin clicks deactivate, THE Frontend SHALL show a confirmation dialog before proceeding
3. WHEN an Admin confirms deactivation, THE Frontend SHALL call `PATCH /api/v1/users/:id` with `is_active: false`
4. IF an Admin attempts to deactivate their own account, THEN THE Frontend SHALL display an error message and prevent the action
5. WHEN the status update succeeds, THE Frontend SHALL update the user badge to reflect the new active state

---

### Requirement 5: PostgreSQL TLS Enforcement

**User Story:** As a security officer, I want all connections to target PostgreSQL databases to use mandatory TLS encryption, so that data in transit cannot be intercepted.

#### Acceptance Criteria

1. THE SECS SHALL use `sslmode=require` instead of `sslmode=prefer` when building PostgreSQL connection strings for target database execution via the `execute` method
2. THE SECS SHALL use `sslmode=require` when building PostgreSQL connection strings for `test_connection` calls to target PostgreSQL databases
3. IF the target PostgreSQL database does not support TLS, THEN THE SECS SHALL fail the connection with an error message indicating that TLS is required but the server did not establish an encrypted connection, and SHALL NOT fall back to an unencrypted plaintext connection
4. IF `sslmode=require` is set and the TLS handshake fails for any reason, THEN THE SECS SHALL return an `ExecutorError::ConnectionFailed` error indicating the TLS negotiation failure reason as reported by the database driver

---

### Requirement 6: Oracle TLS Enforcement

**User Story:** As a security officer, I want all connections to target Oracle databases to use encrypted connections, so that data in transit is protected.

#### Acceptance Criteria

1. THE SECS SHALL configure Oracle connections with TCPS (TLS) protocol by specifying encryption parameters in the connect string
2. WHEN connecting to an Oracle database, THE SECS SHALL use the TNS connect descriptor format with `(SECURITY=(SSL_SERVER_DN_MATCH=YES))` or equivalent TCPS protocol specification
3. IF the target Oracle database does not support TLS, THEN THE SECS SHALL fail the connection with a descriptive error message rather than connecting without encryption

---

### Requirement 7: Role-Aware Dashboard

**User Story:** As a user, I want the dashboard to show statistics relevant to my role, so that I see actionable information instead of global stats I cannot act on.

#### Acceptance Criteria

1. WHILE a Maker is viewing the dashboard, THE Dashboard_API SHALL return statistics filtered to the Maker's own requests only: total, pending, approved, executed, failed, rejected, and cancelled counts
2. WHILE a Checker is viewing the dashboard, THE Dashboard_API SHALL return the count of pending requests available for review (excluding requests created by the Checker themselves) and the Checker's own approval history counts (total reviewed, approved, and rejected)
3. WHILE an Admin is viewing the dashboard, THE Dashboard_API SHALL return global system-wide statistics: total, pending, approved, executed, failed, rejected, and cancelled counts across all users
4. IF a user has multiple roles, THEN THE Dashboard_API SHALL return statistics for the highest-privilege role in the order Admin > Checker > Maker
5. IF the authenticated user has no assigned role, THEN THE Dashboard_API SHALL return an empty statistics response with all counts set to zero

---

### Requirement 8: IP Address in Audit Logs

**User Story:** As a security auditor, I want each audit log entry to include the IP address of the actor, so that I can trace actions back to specific network locations.

#### Acceptance Criteria

1. WHEN any state-changing action is performed through an API handler (CREATED, SUBMITTED, APPROVED, REJECTED, CANCELLED), THE SECS SHALL extract the client IP address using the following precedence: X-Forwarded-For header (first IP in the comma-separated list), then X-Real-IP header, then direct connection address, and pass the resolved IP to the audit log insert function
2. THE SECS SHALL store the IP address as a string of at most 45 characters in the `ip_address` column of the audit_log table for every audit entry created through API handlers
3. IF the IP address cannot be determined because all extraction methods yield absent or unparseable values, THEN THE SECS SHALL store the literal string "unknown" rather than NULL to indicate the extraction was attempted
4. WHEN a background task records an audit entry for asynchronous execution results (EXECUTED, FAILED), THE SECS SHALL store the IP address that was captured from the actor at the time of the triggering approval action

---

### Requirement 9: WebSocket Role-Based Filtering

**User Story:** As a Maker, I want to receive notifications only for my own requests, so that I am not distracted by notifications about other users' requests.

#### Acceptance Criteria

1. THE WebSocket_Channel SHALL include the `created_by` field (the user ID of the request creator) in every Notification message to enable server-side filtering
2. WHILE a user with only the Maker role is connected, THE WebSocket_Channel SHALL forward only notifications where the `created_by` field matches the connected user's ID
3. WHILE a user with only the Checker role is connected, THE WebSocket_Channel SHALL forward notifications of type NEW_REQUEST (new pending requests) and notifications for requests that the Checker has previously approved or rejected
4. WHILE a user with both Maker and Checker roles is connected, THE WebSocket_Channel SHALL forward the union of notifications matching criterion 2 and criterion 3 (notifications for requests created by that user, plus NEW_REQUEST notifications, plus notifications for requests that user has approved or rejected)
5. WHILE a user with the Admin role is connected, THE WebSocket_Channel SHALL forward all notifications without filtering
6. IF a notification cannot be matched to any filtering rule due to a missing `created_by` field, THEN THE WebSocket_Channel SHALL discard the notification and not forward it to non-Admin clients

---

### Requirement 10: Multi-Level Approval Support

**User Story:** As a compliance officer, I want the system to support configurable multi-level approval when required, so that high-risk SQL operations receive additional scrutiny.

#### Acceptance Criteria

1. THE SECS SHALL add an `approval_levels_required` field to the Database_Connection configuration with a default value of 1 and an allowed range of 1 to 5
2. WHEN a request targets a database with `approval_levels_required > 1`, THE SECS SHALL require that many distinct Checkers approve before execution proceeds
3. THE SECS SHALL track each approval level with separate records (approver_id, approved_at, level) in an approvals tracking structure
4. WHILE a request has received fewer approvals than required, THE SECS SHALL keep the request in PENDING status and SHALL NOT trigger SQL execution
5. WHEN the number of recorded approvals for a request equals the `approval_levels_required` of the target database, THE SECS SHALL transition the request to APPROVED status and trigger SQL execution
6. THE Four_Eyes_Principle SHALL apply to each approval level: no single user can provide more than one approval for the same request, and no approver can be the Maker
7. IF a Checker attempts to approve a request they have already approved or that they created, THEN THE SECS SHALL reject the approval with an error message indicating the constraint violation and SHALL NOT modify the existing approval records
8. THE SECS SHALL evaluate approval completeness based on the `approval_levels_required` value captured at the time the request entered PENDING status, so that subsequent configuration changes do not affect in-flight requests
9. THE Frontend SHALL display the current approval progress showing the count of approvals received out of the total required (e.g., "1/2 approvals") for multi-level requests

---

### Requirement 11: EXECUTING Audit State

**User Story:** As a security auditor, I want the system to log an EXECUTING state before SQL runs on the target database, so that the audit trail shows the exact moment execution began.

#### Acceptance Criteria

1. WHEN the SECS begins SQL execution on a target database (after approval), THE SECS SHALL insert an audit log entry with action "EXECUTING" before the actual SQL is sent to the target database
2. THE SECS SHALL insert the EXECUTING audit entry with the actor_id of the Checker who approved the request
3. THE SECS SHALL ensure the EXECUTING audit entry has a timestamp that is earlier than or equal to the EXECUTED or FAILED audit entry timestamp

---

### Requirement 12: Role Change Audit Logging

**User Story:** As a security auditor, I want all administrative user management actions to be recorded in the audit log, so that I have a complete trail of privilege changes.

#### Acceptance Criteria

1. WHEN an Admin changes a user's roles via the update user endpoint, THE SECS SHALL create an audit log entry with action "ROLE_CHANGED", actor_id set to the Admin's user ID, and the detail field containing a structured representation of the old roles and new roles (e.g., "old_roles: [MAKER], new_roles: [MAKER, CHECKER]")
2. WHEN an Admin deactivates or activates a user via the update user endpoint, THE SECS SHALL create an audit log entry with action "USER_STATUS_CHANGED", actor_id set to the Admin's user ID, and the detail field containing the old status and new status (e.g., "old_status: active, new_status: inactive")
3. IF the Admin submits an update that changes both roles and active status in a single request, THEN THE SECS SHALL create separate audit log entries for each change — one "ROLE_CHANGED" and one "USER_STATUS_CHANGED"
4. THE SECS SHALL store user management audit entries using a system-generated UUID as request_id (since these actions are not associated with a specific SQL request), and the audit_log table's request_id foreign key constraint SHALL be updated to allow this approach (either by making request_id nullable or by referencing a dedicated system-level placeholder record)
5. THE SECS SHALL include the IP address of the Admin performing the action in the ip_address field of each user management audit entry, extracted from the HTTP request headers (using X-Forwarded-For when behind a reverse proxy, falling back to the direct connection IP)
6. IF the update user request results in no actual change to roles or status (submitted values are identical to current values), THEN THE SECS SHALL NOT create an audit log entry for that unchanged field

---

### Requirement 13: Exclude Checker's Own Requests from Pending List

**User Story:** As a Checker, I want the pending list to exclude requests I created, so that I only see requests I can actually approve (respecting the four-eyes principle).

#### Acceptance Criteria

1. WHEN a Checker retrieves the pending requests list, THE SECS SHALL exclude requests where `created_by` equals the Checker's user_id
2. THE SECS SHALL apply this filter at the database query level in the `list_pending` function to avoid exposing non-actionable data

---

### Requirement 14: OIDC Session TTL

**User Story:** As a system administrator, I want OIDC pending sessions to expire after a configurable time, so that abandoned login attempts do not consume memory indefinitely.

#### Acceptance Criteria

1. THE SECS SHALL provide a configurable OIDC session TTL with a default value of 300 seconds and a valid range of 60 to 900 seconds
2. THE SECS SHALL run a periodic cleanup task at a configurable interval with a default of 60 seconds and a valid range of 10 to 300 seconds, removing all sessions from the OIDC_Session_Store whose age exceeds the configured TTL
3. WHILE the cleanup task is running, THE SECS SHALL retain any OIDC session whose age has not yet exceeded the configured TTL
4. IF a callback arrives with a state parameter that does not match any session in the OIDC_Session_Store (due to expiration or prior cleanup), THEN THE SECS SHALL return HTTP 401 Unauthorized

---

### Requirement 15: Apply or Remove sql_execution_timeout_secs Config

**User Story:** As a system administrator, I want the global `sql_execution_timeout_secs` config to be used as a maximum cap on per-database timeout, so that no single execution can exceed the system limit.

#### Acceptance Criteria

1. WHEN executing SQL on a target database, THE SECS SHALL use the lesser of the database-specific `timeout_secs` and the global `sql_execution_timeout_secs` as the effective timeout
2. THE SECS SHALL apply this cap in the `execute_sql_async` function before passing timeout to the executor
3. IF the database-specific timeout exceeds the global cap, THEN THE SECS SHALL use the global cap value without error

---

### Requirement 16: Pending List Filtering and Search

**User Story:** As a Checker, I want to filter and search the pending requests list, so that I can quickly find relevant requests when the list is long.

#### Acceptance Criteria

1. THE Frontend SHALL provide a filter dropdown to filter pending requests by target database
2. THE Frontend SHALL provide a text search input that filters pending requests by description content (client-side filtering)
3. THE Frontend SHALL provide sort options for the pending list: by submission date (newest first, oldest first)
4. WHEN filters are applied, THE Frontend SHALL display the count of visible results out of total pending requests
5. THE Frontend SHALL persist the selected filter/sort preferences in browser session storage
