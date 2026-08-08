# Domain Entities — Unit 1: Foundation

## 1. User

```
User {
    id: Uuid (PK, gen_random_uuid())
    oidc_subject: String (unique — identifier từ SSO provider)
    email: String (unique)
    display_name: String
    is_active: bool (soft delete)
    created_at: DateTime<Utc>
    updated_at: DateTime<Utc>
}
```

**Relationships**:
- User ←→ UserDbPermission (many-to-many với DatabaseConnection qua permission table)
- User → SqlRequest (created_by, approved_by)
- User → AuditLog (actor_id)
- User ←→ ApproverUnit (many-to-many qua approver_unit_members)

**Roles** (DB constraint: `user_roles_role_check`):
- `ADMIN` — Quản lý users, database connections, cấu hình hệ thống
- `MAKER` — Tạo và gửi SQL request
- `CHECKER` — Review và approve/reject request
- `APPROVER` — Quản lý Approval Matrix và Approver Units; có thể approve Data Queries

**Notes**:
- Không có password — auth qua OIDC
- `oidc_subject` = `sub` claim từ JWT của SSO provider
- Soft delete: `is_active = false` thay vì DELETE để giữ referential integrity
- Một user có thể có nhiều role đồng thời

---

## 2. DatabaseConnection

```
DatabaseConnection {
    id: Uuid (PK)
    name: String (unique, hiển thị tên cho user, vd: "ORACLE_PROD_ACCOUNTS")
    db_type: DbType (enum: PostgreSQL | MySQL | Oracle | MongoDB)
    host: String
    port: i32
    database_name: String
    schema_name: Option<String>
    encrypted_credentials: String (AES-256-GCM encrypted JSON)
    timeout_secs: i32 (default: 30, max: 300)
    is_active: bool
    created_at: DateTime<Utc>
    updated_at: DateTime<Utc>
}

DbType { PostgreSQL, MySQL, Oracle, MongoDB }
```

**Relationships**:
- DatabaseConnection ←→ User (many-to-many qua UserDbPermission)
- DatabaseConnection → SqlRequest (database_id)

**Security**: `encrypted_credentials` chứa JSON với host/port/user/password — encrypt AES-256-GCM trước khi lưu, không bao giờ log hoặc trả về trong API response.

---

## 3. SqlRequest

```
SqlRequest {
    id: Uuid (PK)
    description: String (bắt buộc — lý do thực thi, min 1 char)
    sql_content: String (bắt buộc — câu SQL, min 1 char)
    status: RequestStatus
    database_id: Uuid (FK → DatabaseConnection)
    created_by: Uuid (FK → User — Maker)
    approved_by: Option<Uuid> (FK → User — Checker, NULL khi chưa approve)
    rejection_reason: Option<String> (bắt buộc khi REJECTED, min 10 chars)
    executed_at: Option<DateTime<Utc>>
    rows_affected: Option<i64>
    execution_error: Option<String>
    created_at: DateTime<Utc>
    updated_at: DateTime<Utc>
    submitted_at: Option<DateTime<Utc>>
}

RequestStatus {
    Draft,
    Pending,
    Approved,
    Rejected,
    Executed,
    Failed,
    Cancelled,
}
```

**DB Constraint**:
```sql
CHECK (approved_by IS NULL OR approved_by != created_by)
-- enforce 4-eyes tại DB level
```

**Notes**:
- `sql_content` không trả về trong list API (chỉ trong detail)
- State transitions được enforce cả ở application layer (can_transition_to) và DB constraint

---

## 4. AuditLog

```
AuditLog {
    id: Uuid (PK)
    request_id: Option<Uuid> (FK → SqlRequest, NULL cho admin actions)
    action: AuditAction
    actor_id: Uuid (FK → User)
    detail: Option<String> (rejection reason, error message, v.v.)
    execution_result: Option<i64> (rows_affected)
    ip_address: Option<String>
    timestamp: DateTime<Utc>
}

AuditAction {
    Created,
    Submitted,
    Approved,
    Rejected,
    Executed,
    Failed,
    Cancelled,
    Executing,
    RoleChanged,
    UserStatusChanged,
    CredentialRotated,
}
```

**Immutability**: App DB user chỉ có INSERT + SELECT — không có UPDATE/DELETE trên bảng này.

---

## 5. UserDbPermission

```
UserDbPermission {
    id: Uuid (PK)
    user_id: Uuid (FK → User)
    database_id: Uuid (FK → DatabaseConnection)
    granted_by: Uuid (FK → User — Admin)
    granted_at: DateTime<Utc>
}
-- UNIQUE(user_id, database_id)
```

---

## 6. State Machine — RequestStatus

```
                    [Maker submit]
DRAFT ─────────────────────────────► PENDING
  ▲                                      │
  │ (clone để tạo lại)          ┌────────┼────────┐
  │                              │        │        │
  │                    [Maker cancel]  [Checker]  [Checker]
  │                              │     approve    reject
  │                              ▼        ▼        ▼
  │                         CANCELLED  APPROVED  REJECTED
  │                                       │
  │                               [System execute]
  │                                ┌──────┴──────┐
  │                                ▼             ▼
  │                            EXECUTED        FAILED
  │
  └── (chỉ DRAFT có thể edit SQL content)
```

**can_transition_to() rules**:
```rust
(Draft, Pending) ✅
(Pending, Approved) ✅  -- chỉ Checker, approved_by != created_by
(Pending, Rejected) ✅  -- chỉ Checker, cần rejection_reason min 10 chars
(Pending, Cancelled) ✅ -- chỉ Maker của request đó
(Approved, Executed) ✅ -- system tự động
(Approved, Failed) ✅   -- system khi execution lỗi
-- Tất cả transitions khác: INVALID
```
