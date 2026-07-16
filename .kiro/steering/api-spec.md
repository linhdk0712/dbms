---
inclusion: auto
description: "API Specification cho SECS Backend — endpoints, request/response schemas, authentication, error codes"
---

# API Specification — SECS Backend

Tài liệu tập trung mô tả toàn bộ API endpoints. Frontend PHẢI kiểm tra spec ở đây
trước khi implement bất kỳ tính năng nào cần gọi API.

**Base URL**: `/api/v1`  
**Auth**: `Authorization: Bearer <JWT>` (trừ endpoints ghi rõ Public)  
**Error format**: `{ "error": "<message>" }`  
**Success format**: `{ "data": {...} }` hoặc `204 No Content`  
**Pagination**: `{ "data": [...], "total": int, "page": int, "per_page": int }`

---

## Auth Endpoints

### POST /api/v1/auth/login [Public]
Redirect đến OIDC provider.
- **Response**: `302` redirect đến OIDC authorization URL

### GET /api/v1/auth/callback [Public]
OIDC callback sau khi user đăng nhập thành công.
- **Query**: `?code=AUTH_CODE&state=STATE`
- **Response 200**:
```json
{
  "data": {
    "token": "jwt_string",
    "user": {
      "id": "uuid",
      "email": "string",
      "display_name": "string",
      "roles": ["MAKER", "CHECKER"]
    }
  }
}
```
- **Error 400**: State không hợp lệ

### POST /api/v1/auth/logout
Invalidate session phía client.
- **Response**: `204 No Content`

### GET /api/v1/auth/me
Thông tin user hiện tại.
- **Response 200**:
```json
{
  "data": {
    "id": "uuid",
    "email": "string",
    "display_name": "string",
    "roles": ["MAKER"],
    "is_active": true
  }
}
```

---

## Request Endpoints

### POST /api/v1/requests
Tạo SQL request mới.
- **Roles**: MAKER
- **Body**:
```json
{
  "description": "string (min 1 char)",
  "sql_content": "string (min 1 char)",
  "database_id": "uuid",
  "submit": false
}
```
- **Response 201**:
```json
{
  "data": {
    "id": "uuid",
    "description": "string",
    "status": "DRAFT | PENDING",
    "database_id": "uuid",
    "created_by": "uuid",
    "created_at": "ISO8601",
    "submitted_at": "ISO8601 | null"
  }
}
```
- **Error 403**: Không có quyền trên database này
- **Error 400**: Validation failed

### GET /api/v1/requests
Danh sách requests (tự lọc theo role).
- **Roles**: MAKER, CHECKER, ADMIN
- **Query params**:
  - `status` (optional): DRAFT, PENDING, APPROVED, REJECTED, EXECUTED, FAILED, CANCELLED
  - `database_id` (optional): uuid
  - `page` (optional, default: 1)
  - `per_page` (optional, default: 20, max: 100)
- **Response 200**:
```json
{
  "data": [
    {
      "id": "uuid",
      "description": "string",
      "status": "PENDING",
      "database_name": "string",
      "database_type": "oracle",
      "created_by_name": "string",
      "created_at": "ISO8601",
      "submitted_at": "ISO8601 | null"
    }
  ],
  "total": 42,
  "page": 1,
  "per_page": 20
}
```
Note: `sql_content` KHÔNG trả về trong list.

### GET /api/v1/requests/:id
Chi tiết request bao gồm sql_content và audit trail.
- **Roles**: MAKER (owner), CHECKER, ADMIN
- **Response 200**:
```json
{
  "data": {
    "id": "uuid",
    "description": "string",
    "sql_content": "string",
    "status": "PENDING",
    "database": {
      "id": "uuid",
      "name": "ORACLE_PROD_ACCOUNTS",
      "db_type": "oracle"
    },
    "analysis": {
      "contains_ddl": false,
      "ddl_keywords": [],
      "statement_count": 1,
      "is_dml_only": true,
      "dangerous_patterns": [
        {
          "severity": "CRITICAL | HIGH | MEDIUM | LOW",
          "message": "string",
          "suggestion": "string"
        }
      ]
    },
    "created_by": { "id": "uuid", "display_name": "string" },
    "approved_by": { "id": "uuid", "display_name": "string" } | null,
    "rejection_reason": "string | null",
    "rows_affected": "int | null",
    "execution_error": "string | null",
    "created_at": "ISO8601",
    "submitted_at": "ISO8601 | null",
    "executed_at": "ISO8601 | null",
    "audit_trail": [
      {
        "action": "CREATED | SUBMITTED | APPROVED | REJECTED | EXECUTED | FAILED | CANCELLED",
        "actor": { "id": "uuid", "display_name": "string" },
        "timestamp": "ISO8601",
        "detail": "string | null",
        "execution_result": "int | null"
      }
    ]
  }
}
```

### PUT /api/v1/requests/:id
Sửa request (chỉ khi DRAFT).
- **Roles**: MAKER (owner)
- **Body**:
```json
{
  "description": "string (optional)",
  "sql_content": "string (optional)",
  "database_id": "uuid (optional)"
}
```
- **Response 200**: Updated request object
- **Error 400**: ImmutableRequest (nếu không phải DRAFT)

### POST /api/v1/requests/:id/submit
Submit request DRAFT -> PENDING.
- **Roles**: MAKER (owner)
- **Body**: None
- **Response 200**: Request object với status=PENDING
- **Error 400**: InvalidStateTransition

### POST /api/v1/requests/:id/cancel
Hủy request PENDING.
- **Roles**: MAKER (owner)
- **Body**: None
- **Response 200**: Request object với status=CANCELLED
- **Error 403**: Không phải owner
- **Error 400**: Không phải PENDING

### POST /api/v1/requests/:id/approve
Checker approve.
- **Roles**: CHECKER
- **Body**: None
- **Response 200**: Request object với status=APPROVED
- **Error 400**: FourEyesViolation, InvalidStateTransition
- **Note**: Trigger async SQL execution

### POST /api/v1/requests/:id/reject
Checker reject.
- **Roles**: CHECKER
- **Body**:
```json
{
  "rejection_reason": "string (min 10 chars)"
}
```
- **Response 200**: Request object với status=REJECTED
- **Error 400**: FourEyesViolation, rejection_reason quá ngắn

### POST /api/v1/requests/:id/clone
Clone request thành DRAFT mới.
- **Roles**: MAKER
- **Body**: None
- **Response 201**: New request object (status=DRAFT)

---

## Database Endpoints

### GET /api/v1/databases
Danh sách database connections.
- **Roles**: ADMIN (tất cả), MAKER/CHECKER (chỉ DB mình có quyền)
- **Response 200**:
```json
{
  "data": [
    {
      "id": "uuid",
      "name": "ORACLE_PROD_ACCOUNTS",
      "db_type": "oracle",
      "host": "string",
      "port": 1521,
      "database_name": "string",
      "schema_name": "string | null",
      "timeout_secs": 30,
      "is_active": true,
      "created_at": "ISO8601"
    }
  ]
}
```
Note: `encrypted_credentials` KHÔNG BAO GIỜ trả về.

### POST /api/v1/databases
Tạo database connection mới.
- **Roles**: ADMIN
- **Body**:
```json
{
  "name": "string (unique)",
  "db_type": "postgresql | mysql | oracle | mongodb",
  "host": "string",
  "port": "int (1-65535)",
  "database_name": "string",
  "schema_name": "string | null",
  "username": "string",
  "password": "string",
  "timeout_secs": "int (1-300, default: 30)"
}
```
- **Response 201**: Database object (không có credentials)

### PATCH /api/v1/databases/:id
Cập nhật database connection.
- **Roles**: ADMIN
- **Body**: Partial update (chỉ các field cần thay đổi)
- **Response 200**: Updated database object

### POST /api/v1/databases/:id/test
Test kết nối.
- **Roles**: ADMIN
- **Response 200**:
```json
{
  "data": {
    "status": "ok | error",
    "latency_ms": 45,
    "message": "string (nếu error)"
  }
}
```

---

## User Endpoints

### GET /api/v1/users
Danh sách users.
- **Roles**: ADMIN
- **Response 200**: Paginated list of users

### PATCH /api/v1/users/:id
Cập nhật user (activate/deactivate, gán role).
- **Roles**: ADMIN
- **Body**:
```json
{
  "is_active": "bool (optional)",
  "roles": ["ADMIN", "MAKER", "CHECKER"] (optional)
}
```

### POST /api/v1/users/:user_id/permissions
Cấp quyền user trên database.
- **Roles**: ADMIN
- **Body**: `{ "database_id": "uuid" }`
- **Response 201**: Permission object
- **Error 409**: Permission already exists

### DELETE /api/v1/users/:user_id/permissions/:database_id
Thu hồi quyền.
- **Roles**: ADMIN
- **Response**: `204 No Content`

---

## Dashboard Endpoint

### GET /api/v1/dashboard
Thống kê theo role.
- **Roles**: Tất cả (response khác nhau theo role)
- **Response 200** (MAKER):
```json
{
  "data": {
    "role": "MAKER",
    "stats": {
      "draft": 2, "pending": 3, "approved": 1,
      "executed": 45, "rejected": 5, "failed": 1
    },
    "recent_requests": [...],
    "avg_approval_time_minutes": 42
  }
}
```
- **Response 200** (CHECKER):
```json
{
  "data": {
    "role": "CHECKER",
    "stats": { "pending": 8, "approved_today": 3, "rejected_today": 1 },
    "oldest_pending_age_hours": 4.5,
    "recent_approvals": [...]
  }
}
```
- **Response 200** (ADMIN):
```json
{
  "data": {
    "role": "ADMIN",
    "stats": {
      "total_requests": 200,
      "pending": 12, "executed": 150, "failed": 5
    },
    "top_makers": [...],
    "top_databases": [...],
    "failed_last_7_days": 3,
    "active_users_today": 15
  }
}
```

---

## WebSocket

### GET /ws?token=JWT_TOKEN
Kết nối WebSocket cho realtime notification.
- **Auth**: JWT trong query param (không dùng header vì WebSocket API limitation)
- **Messages từ server**:
```json
{
  "type": "NEW_REQUEST | REQUEST_APPROVED | REQUEST_REJECTED | REQUEST_EXECUTED | REQUEST_FAILED | REQUEST_CANCELLED",
  "request_id": "uuid",
  "database_name": "string | null",
  "actor_id": "uuid | null",
  "rows_affected": "int | null",
  "detail": "string | null"
}
```
- **Heartbeat**: Server gửi Ping mỗi 30s, client trả Pong

---

## Health Check [Public]

### GET /health
- **Response 200**: `{ "status": "ok", "db": "connected" }`
- **Response 503**: `{ "status": "error", "db": "disconnected" }`
