# DAB - SQL Execution Control System (SECS)

> **Document Type**: Design Architecture Blueprint  
> **Version**: 1.0  
> **Status**: Draft  
> **System**: SQL Execution Control System (SECS)  
> **Stack**: Rust/Axum · Svelte · PostgreSQL · Keycloak (OIDC)

---

## Table of Contents

1. [BUSINESS CONTEXT](#1-business-context)
   - 1.1 [Introduction](#11-introduction)
   - 1.2 [Objectives & Goals](#12-objectives--goals)
   - 1.3 [Scopes](#13-scopes)
   - 1.4 [Requirements](#14-requirements)
2. [PROPOSED SOLUTION](#2-proposed-solution)
   - 2.1 [Key Design Principles](#21-key-design-principles)
   - 2.2 [Current State Architecture](#22-current-state-architecture)
   - 2.3 [High-level Architecture](#23-high-level-architecture)
3. [Data Flow Diagram](#3-data-flow-diagram)
   - 3.1 [Detailed Design](#31-detailed-design)
4. [Infrastructure Design and Operation](#4-infrastructure-design-and-operation)
5. [Security Design](#5-security-design)

---

# 1. BUSINESS CONTEXT

## 1.1 Introduction

### Overview

SQL Execution Control System (SECS) là hệ thống quản lý và kiểm soát thực thi câu lệnh SQL trên môi trường production, được thiết kế cho các tổ chức tài chính có yêu cầu tuân thủ cao về kiểm soát thay đổi dữ liệu.

Hệ thống áp dụng nguyên tắc **4-eyes principle** (maker-checker): mọi câu lệnh DML/DDL tác động đến database production phải được ít nhất một người có thẩm quyền khác phê duyệt trước khi thực thi. Điều này loại bỏ rủi ro nhân viên vận hành thao tác trực tiếp không có kiểm soát.

SECS cũng cung cấp luồng **Data Query** (chỉ SELECT) với approval matrix linh hoạt theo từng database/schema/table, cho phép truy xuất dữ liệu có kiểm soát mà không yêu cầu kết nối trực tiếp vào database.

### Problem Statements

| # | Vấn đề | Hậu quả |
|---|--------|---------|
| 1 | Nhân viên vận hành kết nối trực tiếp vào database production để thực thi SQL mà không có quy trình kiểm duyệt | Rủi ro mất hoặc sai lệch dữ liệu không phát hiện được |
| 2 | Không có audit trail đầy đủ về ai đã thực thi câu lệnh gì, lúc nào, và vì lý do gì | Không đáp ứng yêu cầu audit của SOX, nội kiểm |
| 3 | Credentials database production được chia sẻ trực tiếp cho nhiều người dùng | Không thể truy vết nguyên nhân khi xảy ra sự cố |
| 4 | Không có cơ chế cảnh báo khi câu lệnh DDL (DROP, TRUNCATE, ALTER) được thực thi | Có thể xảy ra sự cố không phục hồi được |
| 5 | Không có cơ chế liên kết với quy trình Change Request (CR) trong ITSM | Vi phạm quy trình quản lý thay đổi của tổ chức |

---

## 1.2 Objectives & Goals

**Mục tiêu chính:**

- Triển khai quy trình Maker-Checker đối với mọi thao tác SQL trên database production, đảm bảo 100% câu lệnh DML/DDL có ít nhất một lượt phê duyệt độc lập trước khi thực thi.
- Xây dựng audit trail không thể sửa đổi (append-only) cho mọi thay đổi trạng thái, lưu giữ tối thiểu 7 năm theo yêu cầu SOX.
- Loại bỏ việc chia sẻ credentials database trực tiếp — toàn bộ thực thi thông qua service account với quyền tối thiểu.
- Tích hợp với SSO hiện có của tổ chức (Keycloak/OIDC) — không quản lý password riêng.
- Cung cấp luồng Data Query có kiểm soát thay thế việc truy vấn trực tiếp database.

**Chỉ số thành công (KPIs):**

| KPI | Mục tiêu |
|-----|---------|
| Tỉ lệ SQL thực thi trực tiếp không qua SECS | 0% |
| Thời gian phản hồi API (P95) | < 500ms |
| Uptime | ≥ 99.5% |
| Thời gian audit log retention | ≥ 7 năm |
| Thời gian rollback khi execution failure | < 1 phút (auto-rollback) |

---

## 1.3 Scopes

### In-scope

- Workflow Maker-Checker cho các câu lệnh DML: `INSERT`, `UPDATE`, `DELETE`
- Workflow Maker-Checker cho các câu lệnh DDL với cảnh báo đặc biệt: `DROP`, `TRUNCATE`, `ALTER`, `CREATE`
- Multi-level approval (tối đa 5 cấp) có thể cấu hình theo từng database connection
- Workflow Data Query (SELECT-only) với approval matrix theo database/schema/table
- Quản lý kết nối đến 4 loại database đích: PostgreSQL, MySQL, Oracle, MongoDB
- Thực thi SQL nguyên tử trong transaction (tự động rollback khi lỗi)
- Timeout cấu hình được (mặc định 30s, tối đa 300s) cho từng connection
- Phân quyền user theo từng database connection cụ thể (không có global permission)
- Audit log append-only cho mọi hành động (CREATED, SUBMITTED, APPROVED, REJECTED, EXECUTED, FAILED, CANCELLED)
- Tích hợp Change Request với ManageEngine ServiceDesk Plus
- Phân tích SQL bằng AI (optional feature flag) để hỗ trợ Checker review
- Xác thực qua OIDC/SSO (Keycloak)
- Role-based access control: Admin, Maker, Checker, Approver
- Dashboard theo role
- Schema browser (xem danh sách table/column từ database đích)

### Out-of-scope

- Quản lý tài khoản user (do SSO provider đảm nhiệm)
- Backup và restore dữ liệu của database đích
- Monitoring hiệu năng của database đích
- Thực thi stored procedure với tham số động
- Query builder GUI (người dùng tự nhập SQL)
- Deployment pipeline tự động
- Mobile application

### Impacted User Journey / Business Process

```
Trước SECS:
  Nhân viên vận hành → kết nối trực tiếp DB production → thực thi SQL → (không có kiểm soát)

Sau SECS:
  Maker → soạn SQL trong SECS → submit → Checker review → approve → SECS tự động thực thi
                                                         → reject  → Maker nhận thông báo

Luồng Data Query:
  Maker → tạo SELECT query → submit → Approver Unit (ITO/ITS/COMPLIANCE) duyệt → Assigned user thực thi
```

**Business process bị ảnh hưởng:**
- Quy trình vận hành database production (thay thế hoàn toàn)
- Quy trình audit nội bộ (tích hợp trực tiếp)
- Quy trình Change Management (liên kết CR từ ManageEngine)
- Quy trình onboarding user mới (gán role + DB permission qua Admin UI)

### Impacted Systems

| Hệ thống | Loại tác động | Mô tả |
|----------|--------------|-------|
| Keycloak (SSO) | Tích hợp inbound | SECS dùng Keycloak làm OIDC provider cho authentication |
| ManageEngine ServiceDesk Plus | Tích hợp outbound | SECS gọi API để lấy thông tin Change Request |
| Database đích (Oracle/PostgreSQL/MySQL/MongoDB) | Tích hợp outbound | SECS thực thi SQL thông qua service account |
| Hệ thống giám sát (Prometheus/Grafana) | Tích hợp outbound | SECS expose metrics |

---

## 1.4 Requirements

### Functional Requirement

**FR-01: Maker — SQL Request Workflow**
- Maker tạo SQL request với mô tả, câu SQL, và database đích
- Maker chỉ thấy database mình được cấp quyền
- Sau khi submit, Maker không thể sửa nội dung SQL — phải hủy và tạo mới
- Maker có thể clone request đã có để tạo request mới
- Maker có thể cancel request đang ở trạng thái PENDING
- Maker có thể liên kết request với Change Request từ ManageEngine

**FR-02: Checker — Approval Workflow**
- Checker xem danh sách request đang chờ duyệt
- Checker xem đầy đủ nội dung SQL và phân tích DDL trước khi quyết định
- Checker approve hoặc reject (kèm lý do, bắt buộc tối thiểu 10 ký tự)
- Checker không thể approve request do chính mình tạo (4-eyes principle)
- Hệ thống tự động thực thi SQL ngay sau khi đủ số lượt approve

**FR-03: Multi-level Approval**
- Admin cấu hình số lượt approve required cho từng database (1–5 lượt)
- Request chỉ được thực thi khi đủ approval_levels_required lượt approve
- Mỗi lượt approve phải là người khác nhau (không approve 2 lần)

**FR-04: Data Query Workflow**
- Maker tạo SELECT query request với ticket ID bắt buộc
- DBA/Admin cấu hình approval rules theo database/schema/table
- Approval matrix có thể gồm nhiều approver unit (ITO, ITS, COMPLIANCE) theo thứ tự
- Assigned users (do Maker chỉ định) thực thi query sau khi approved
- Query có thời hạn (expires_at) — hết hạn không thể thực thi
- Mọi lần thực thi được ghi vào data_query_executions

**FR-05: SQL Execution**
- Thực thi trong transaction, rollback toàn bộ nếu một statement lỗi
- Timeout cấu hình được theo từng connection
- Ghi kết quả (rows_affected hoặc error) vào audit log trước khi trả response
- DDL statements được phát hiện và gắn cờ cảnh báo (Warning/Danger)

**FR-06: Admin**
- CRUD database connections (name, type, host, port, credentials)
- Test connection trước khi lưu
- Cấp/thu hồi quyền user trên từng database
- Quản lý approver units và gán user vào unit
- Activate/deactivate user account

**FR-07: AI SQL Analysis (optional)**
- Khi `AI_ENABLED=true`: tự động phân tích SQL để hỗ trợ Checker
- Phát hiện rủi ro: SQL không có WHERE clause, DDL statements, v.v.
- Rate limit: 5 request/user/phút

### Non-functional Requirement

| ID | Thuộc tính | Yêu cầu |
|----|-----------|---------|
| NFR-01 | Availability | Uptime ≥ 99.5% (ứng dụng nội bộ) |
| NFR-02 | Performance | Response time P95 < 500ms cho API; SQL execution timeout max 300s |
| NFR-03 | Security | Mọi kết nối TLS; credentials mã hóa AES-256-GCM; JWT TTL 8h; rate limit login 10 req/min/IP |
| NFR-04 | Auditability | 100% thay đổi trạng thái ghi audit log; lưu giữ ≥ 7 năm; append-only |
| NFR-05 | Scalability | Stateless backend — scale horizontal; connection pool PostgreSQL |
| NFR-06 | Maintainability | Cargo clippy -D warnings = 0; cargo fmt; migration versioned |
| NFR-07 | Body limit | Request body ≤ 100KB |
| NFR-08 | CORS | Chỉ accept origin từ FRONTEND_URL đã cấu hình |

### Data Requirement

- **Audit log retention**: ≥ 7 năm (SOX compliance)
- **Credentials storage**: Mã hóa AES-256-GCM, không bao giờ trả về qua API
- **SQL content**: Lưu toàn bộ, không log ra console, không trả về trong list API (chỉ trong detail API khi có quyền)
- **User data**: Sync từ OIDC claims (sub, email, display_name) — không lưu password
- **Change Request data**: Cache metadata từ ManageEngine (không là system of record)

---

# 2. PROPOSED SOLUTION

## 2.1 Key Design Principles

| Nguyên tắc | Áp dụng |
|-----------|---------|
| **4-eyes principle** | Maker và Checker phải là hai người khác nhau; kiểm tra ở cả application layer (business rule) và database constraint (`CHECK (approved_by != created_by)`) |
| **Immutability after submission** | SQL content không thể thay đổi sau khi submit — phải cancel và tạo mới |
| **Append-only audit log** | App DB user chỉ có quyền INSERT + SELECT trên `audit_log`, không có UPDATE/DELETE |
| **Atomic execution** | Toàn bộ SQL request thực thi trong một transaction — rollback toàn bộ nếu bất kỳ statement nào lỗi |
| **Least privilege** | Service account trên database đích chỉ có quyền tối thiểu; không có global permission — phân quyền explicit từng user cho từng database |
| **Defense in depth** | Business rules enforce ở cả application layer và database constraint |
| **Credentials never in logs** | Encrypted credentials không bao giờ được log hoặc trả về trong API response |
| **Stateless backend** | JWT-based session, không lưu session state trong backend — sẵn sàng horizontal scale |
| **Fail secure** | Khi có lỗi hoặc thiếu thông tin → từ chối, không cho phép thực thi |

---

## 2.2 Current State Architecture

Trước khi triển khai SECS, môi trường hiện tại hoạt động theo mô hình:

```
Nhân viên vận hành
        │
        │  Kết nối trực tiếp (SQL Client: DBeaver, SQLPlus, v.v.)
        │  Dùng shared credentials
        ▼
┌───────────────────────────────────────────┐
│          Database Production              │
│  Oracle / PostgreSQL / MySQL / MongoDB    │
└───────────────────────────────────────────┘

Vấn đề:
  - Không có kiểm soát (ai có thể thực thi gì)
  - Không có audit trail (ai đã thực thi gì, khi nào)
  - Credentials được chia sẻ trực tiếp
  - Không có rollback tự động khi lỗi
  - Không liên kết với quy trình Change Management
```

---

## 2.3 High-level Architecture

### SECS Backend

```
┌──────────────────────────────────────────────────────────────────┐
│                        SECS Backend (Rust/Axum)                   │
│                                                                    │
│  ┌─────────────┐   ┌──────────────┐   ┌───────────────────────┐  │
│  │  api/ layer │   │   auth/      │   │      ws/              │  │
│  │  (handlers) │   │  JWT + OIDC  │   │  WebSocket notif.     │  │
│  └──────┬──────┘   └──────┬───────┘   └───────────────────────┘  │
│         │                 │                                        │
│  ┌──────▼─────────────────▼──────────────────────────────────┐   │
│  │                     db/ layer                              │   │
│  │  users · requests · approvals · databases · permissions    │   │
│  │  audit · change_requests · data_queries                    │   │
│  └──────────────────────────┬──────────────────────────────── ┘  │
│                             │ sqlx                                 │
│  ┌──────────────────────────▼──────────────────────────────── ┐  │
│  │                  PostgreSQL (nội bộ)                        │  │
│  │  secs_db — lưu trạng thái ứng dụng, audit log, config      │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                    │
│  ┌────────────────────────────────────────────────────────────┐   │
│  │                  executor/ layer                           │   │
│  │  DatabaseExecutor trait                                    │   │
│  │  PgExecutor · MySqlExecutor · OracleExecutor · MongoExec.  │   │
│  └───────┬────────────┬────────────┬────────────┬────────────┘   │
└──────────┼────────────┼────────────┼────────────┼────────────────┘
           │            │            │            │
           ▼            ▼            ▼            ▼
      PostgreSQL      MySQL        Oracle       MongoDB
      (đích)          (đích)       (đích)       (đích)
```

**Phân tầng (Separation of Concerns):**

| Layer | Trách nhiệm |
|-------|-------------|
| `api/` | Parse HTTP request, gọi `db/` layer, trả response — không chứa business logic |
| `auth/` | JWT validation, OIDC flow (PKCE + CSRF), rate limit, role extractors |
| `db/` | Query PostgreSQL nội bộ, enforce business rules, state transitions |
| `executor/` | Thực thi SQL trên database đích qua TLS — không biết về HTTP hay PostgreSQL nội bộ |
| `models/` | Domain types, state machine — không có I/O hay side effects |

### SECS Frontend

```
┌───────────────────────────────────────────────────────────┐
│                   SECS Frontend (Svelte + Vite)            │
│                                                            │
│  App.svelte (router + auth guard + WS connect)             │
│                                                            │
│  pages/                                                    │
│  ├── Login.svelte          (SSO redirect)                  │
│  ├── AuthCallback.svelte   (OIDC callback)                 │
│  ├── Dashboard.svelte      (role-aware stats)              │
│  ├── maker/                                                │
│  │   ├── NewRequest.svelte                                 │
│  │   ├── MyRequests.svelte                                 │
│  │   └── RequestDetail.svelte                             │
│  ├── checker/                                              │
│  │   ├── PendingList.svelte                               │
│  │   └── ReviewDetail.svelte                              │
│  └── admin/                                               │
│      ├── Users.svelte                                     │
│      └── Databases.svelte                                 │
│                                                            │
│  stores/                   api/                            │
│  ├── auth.js (JWT)         ├── index.js (apiFetch)         │
│  └── notifications.js (WS) └── endpoints.js               │
└───────────────────────────────────────────────────────────┘
```

---

# 3. Data Flow Diagram

## Luồng chính: SQL Request Workflow

```
Maker                    SECS Backend              PostgreSQL (nội bộ)      DB Đích
  │                           │                           │                    │
  │── POST /requests ─────────►                           │                    │
  │   {sql, description,      │── INSERT sql_requests ───►│                    │
  │    database_id}           │   status=DRAFT            │                    │
  │◄── 201 Created ───────────│◄── OK ────────────────────│                    │
  │                           │                           │                    │
  │── POST /requests/:id/submit►                          │                    │
  │                           │── UPDATE status=PENDING ──►                    │
  │                           │── INSERT audit SUBMITTED ─►                    │
  │◄── 200 OK ────────────────│◄── OK ────────────────────│                    │
  │                           │                           │                    │
Checker                       │                           │                    │
  │── GET /approvals/pending──►                           │                    │
  │◄── [...requests] ─────────│                           │                    │
  │                           │                           │                    │
  │── POST /approvals/:id/approve►                        │                    │
  │                           │── 4-eyes check ──────────►│                    │
  │                           │── UPDATE status=APPROVED ─►                    │
  │                           │── INSERT audit APPROVED ──►                    │
  │                           │                           │                    │
  │                           │── execute_sql() ──────────────────────────────►│
  │                           │   (transaction wrap)      │                    │
  │                           │                           │          ◄── result│
  │                           │── UPDATE status=EXECUTED ─►                    │
  │                           │── INSERT audit EXECUTED ──►                    │
  │◄── 200 {rows_affected} ───│                           │                    │
```

## Luồng: Data Query Workflow

```
Maker → POST /data-queries → submit → 
  Approver Unit (theo rule) → approve per level →
  Assigned User → POST /data-queries/:id/execute → 
  SECS chạy SELECT → trả kết quả → ghi vào data_query_executions
```

## Luồng: Authentication (OIDC PKCE)

```
Browser → GET /auth/login → redirect Keycloak → user login →
  callback với code → GET /auth/callback → 
  SECS exchange code for tokens (PKCE) → verify at_hash →
  upsert user (sub, email) → issue JWT (8h) → redirect frontend
```

---

## 3.1 Detailed Design

### Detailed Flow and Exception Handling

**State Machine — SqlRequest:**

```
DRAFT ──submit──► PENDING ──approve (đủ levels)──► APPROVED ──execute OK──► EXECUTED
                     │                                 │
                   cancel                           execute FAIL──► FAILED
                     │
                   CANCELLED
                     │
                  reject
                     │
                  REJECTED
```

**Exception Handling:**

| Tình huống | Xử lý |
|-----------|-------|
| Approve request của chính mình | HTTP 403 — vi phạm 4-eyes principle |
| SQL execution timeout | Hủy statement, UPDATE status=FAILED, INSERT audit FAILED với timeout message |
| SQL execution error | Rollback transaction, UPDATE status=FAILED, INSERT audit FAILED với error message |
| DB connection failure | HTTP 502, INSERT audit FAILED với connection error |
| DDL statement detected | Trả về cảnh báo severity (Warning/Danger) trong response để Checker xem |
| Approve khi đã EXECUTED | HTTP 409 Conflict |
| Submit khi đã PENDING | HTTP 409 Conflict |

---

### Integration Detailed Design

#### Message Specifications

Tất cả API request/response sử dụng **JSON** (`Content-Type: application/json`).

**Request tạo SQL Request:**
```json
POST /api/v1/requests
{
  "description": "Cập nhật trạng thái đơn hàng #12345 sau khi xử lý thủ công",
  "sql_content": "UPDATE orders SET status = 'COMPLETED' WHERE id = 12345",
  "database_id": "uuid-of-database-connection",
  "cr_id": "uuid-of-change-request"  // optional
}
```

**Response detail SQL Request:**
```json
{
  "id": "uuid",
  "description": "...",
  "sql_content": "...",  // chỉ có trong detail, không có trong list
  "status": "PENDING",
  "database": { "id": "...", "name": "ORACLE_PROD", "db_type": "oracle" },
  "created_by": { "id": "...", "display_name": "Nguyen Van A", "email": "..." },
  "approval_levels_required": 1,
  "current_approval_count": 0,
  "ddl_analysis": {
    "has_ddl": false,
    "statements": []
  },
  "created_at": "2024-01-15T08:00:00Z",
  "submitted_at": "2024-01-15T08:05:00Z"
}
```

**Approve request:**
```json
POST /api/v1/approvals/:id/approve
{}  // body rỗng — không cần thêm thông tin

// Response
{
  "id": "uuid",
  "status": "EXECUTED",
  "rows_affected": 1,
  "executed_at": "2024-01-15T09:00:00Z"
}
```

**Reject request:**
```json
POST /api/v1/approvals/:id/reject
{
  "reason": "SQL thiếu điều kiện WHERE, có thể ảnh hưởng toàn bộ bảng"
}
```

#### Redis

Hệ thống **không sử dụng Redis**. Rate limiting được thực hiện in-memory (`DashMap`) trên từng instance backend. Phù hợp với deployment single-instance nội bộ.

#### RabbitMQ

Hệ thống **không sử dụng message queue**. Thực thi SQL là synchronous — Checker gọi approve và chờ kết quả thực thi trong cùng HTTP request. Điều này đơn giản hóa kiến trúc và phù hợp với volume thấp của môi trường nội bộ.

#### API Specifications

**Base URL**: `http(s)://<host>:<port>`  
**Authentication**: `Authorization: Bearer <jwt_token>`  
**API Version**: `/api/v1/`

| Method | Endpoint | Role | Mô tả |
|--------|----------|------|-------|
| GET | `/health` | Public | Health check |
| GET | `/auth/login` | Public | Redirect Keycloak (rate limit: 10/min/IP) |
| GET | `/auth/callback` | Public | OIDC callback |
| GET | `/auth/me` | Auth | Thông tin user hiện tại |
| POST | `/auth/logout` | Auth | Xóa OIDC session |
| GET | `/api/v1/dashboard` | Auth | Stats theo role |
| POST | `/api/v1/requests` | Maker | Tạo SQL request |
| GET | `/api/v1/requests` | Maker | Danh sách request của mình |
| GET | `/api/v1/requests/:id` | Maker | Chi tiết request |
| PATCH | `/api/v1/requests/:id` | Maker | Sửa request (chỉ DRAFT) |
| POST | `/api/v1/requests/:id/submit` | Maker | Submit request |
| POST | `/api/v1/requests/:id/cancel` | Maker | Hủy request |
| POST | `/api/v1/requests/:id/clone` | Maker | Clone request |
| POST | `/api/v1/requests/:id/analyze` | Maker | AI analysis (nếu enabled) |
| GET | `/api/v1/requests/:id/audit` | Auth | Audit log của request |
| GET | `/api/v1/approvals/pending` | Checker | Danh sách chờ duyệt |
| GET | `/api/v1/approvals/:id` | Checker | Chi tiết để review |
| POST | `/api/v1/approvals/:id/validate` | Checker | Validate SQL (EXPLAIN) |
| POST | `/api/v1/approvals/:id/approve` | Checker | Approve và execute |
| POST | `/api/v1/approvals/:id/reject` | Checker | Reject với lý do |
| POST | `/api/v1/approvals/:id/retry` | Checker | Retry FAILED request |
| GET | `/api/v1/databases` | Admin | Danh sách database connections |
| POST | `/api/v1/databases` | Admin | Tạo database connection |
| GET | `/api/v1/databases/:id` | Admin | Chi tiết database |
| PATCH | `/api/v1/databases/:id` | Admin | Cập nhật database |
| POST | `/api/v1/databases/:id/test` | Admin | Test connection |
| POST | `/api/v1/databases/test-new` | Admin | Test connection trước khi lưu |
| PATCH | `/api/v1/databases/:id/credentials` | Admin | Rotate credentials |
| GET | `/api/v1/databases/my-permissions` | Maker | Danh sách DB mình có quyền |
| GET | `/api/v1/databases/:id/tables` | Auth | Schema browser — tables |
| GET | `/api/v1/databases/:id/columns` | Auth | Schema browser — columns |
| GET | `/api/v1/users` | Admin | Danh sách user |
| GET | `/api/v1/users/:id` | Admin | Chi tiết user |
| PATCH | `/api/v1/users/:id` | Admin | Cập nhật roles/status |
| GET | `/api/v1/users/:id/permissions` | Admin | DB permissions của user |
| POST | `/api/v1/users/:id/permissions` | Admin | Cấp quyền |
| DELETE | `/api/v1/users/:id/permissions/:db_id` | Admin | Thu hồi quyền |
| GET | `/api/v1/users/:id/approver-units` | Admin | Approver units của user |
| POST | `/api/v1/users/:id/approver-units` | Admin | Gán vào unit |
| DELETE | `/api/v1/users/:id/approver-units/:unit_id` | Admin | Bỏ khỏi unit |
| POST | `/api/v1/change-requests` | Maker | Tạo Change Request |
| GET | `/api/v1/change-requests` | Maker | Danh sách CR |
| GET | `/api/v1/change-requests/:id` | Maker | Chi tiết CR |
| POST | `/api/v1/change-requests/:id/sync` | Maker | Sync từ ManageEngine |
| DELETE | `/api/v1/change-requests/:id` | Maker | Xóa CR |
| POST | `/api/v1/data-queries` | Maker | Tạo data query |
| GET | `/api/v1/data-queries` | Auth | Danh sách data queries |
| GET | `/api/v1/data-queries/:id` | Auth | Chi tiết |
| PUT | `/api/v1/data-queries/:id` | Maker | Cập nhật (chỉ DRAFT) |
| POST | `/api/v1/data-queries/:id/submit` | Maker | Submit |
| POST | `/api/v1/data-queries/:id/execute` | Assigned | Thực thi query |
| GET | `/api/v1/data-queries/:id/executions` | Auth | Lịch sử thực thi |
| GET | `/api/v1/data-query-approvals/pending` | Approver | Danh sách chờ duyệt |
| POST | `/api/v1/data-queries/:id/approve` | Approver | Approve |
| POST | `/api/v1/data-queries/:id/reject` | Approver | Reject |
| GET | `/api/v1/data-query-rules` | Admin | Approval rules |
| POST | `/api/v1/data-query-rules` | Admin | Tạo rule |
| PUT | `/api/v1/data-query-rules/:id` | Admin | Cập nhật rule |
| DELETE | `/api/v1/data-query-rules/:id` | Admin | Xóa rule |
| GET | `/api/v1/approver-units` | Admin | Danh sách approver units |
| POST | `/api/v1/approver-units` | Admin | Tạo unit |
| PUT | `/api/v1/approver-units/:id` | Admin | Cập nhật unit |
| DELETE | `/api/v1/approver-units/:id` | Admin | Xóa unit |

**HTTP Error Codes:**

| Code | Tình huống |
|------|-----------|
| 400 | Validation error (body không hợp lệ) |
| 401 | Thiếu hoặc hết hạn JWT token |
| 403 | Không có quyền (sai role, 4-eyes violation, không có DB permission) |
| 404 | Resource không tìm thấy |
| 409 | Conflict (trạng thái không cho phép hành động này) |
| 429 | Rate limit exceeded |
| 500 | Internal server error |
| 502 | Không kết nối được database đích |

---

### Data Design

#### Entity and Domain Modeling

```
┌────────────┐         ┌──────────────────┐
│   users    │         │database_connections│
│ id (PK)    │         │ id (PK)           │
│ oidc_sub   │         │ name              │
│ email      │         │ db_type           │
│ display_nm │         │ host, port        │
│ is_active  │         │ database_name     │
└─────┬──────┘         │ encrypted_creds   │
      │                │ timeout_secs      │
      │  ┌─────────────│ approval_levels   │
      │  │             └──────────────────-┘
      │  │
      ▼  ▼
┌──────────────────────┐
│    sql_requests      │◄──────── change_requests
│ id (PK)              │          (optional FK cr_id)
│ description          │
│ sql_content          │
│ status               │
│ database_id (FK)     │
│ created_by (FK)      │
│ approved_by (FK)     │
│ approval_levels_req  │
└──────────┬───────────┘
           │  1:N
     ┌─────┴──────────┐        ┌──────────────────┐
     │   audit_log    │        │request_approvals  │
     │ id (PK)        │        │ id (PK)           │
     │ request_id(FK) │        │ request_id (FK)   │
     │ action         │        │ approver_id (FK)  │
     │ actor_id (FK)  │        │ level             │
     │ detail         │        │ approved_at       │
     │ exec_result    │        └──────────────────-┘
     │ ip_address     │
     │ timestamp      │
     └────────────────┘

user_roles (user_id FK, role)
user_db_permissions (user_id FK, database_id FK, granted_by FK)
user_approver_units (user_id FK, unit_id FK)
approver_units (id PK, name, is_active)

data_query_requests → data_query_assigned_users
                   → data_query_approval_tracking
                   → data_query_executions
data_query_approval_rules → data_query_approval_levels
```

**Domain Rules được enforce tại DB layer:**
- `four_eyes_check`: `CHECK (approved_by IS NULL OR approved_by != created_by)`
- `rejection_reason_required`: `CHECK (status != 'REJECTED' OR length(rejection_reason) >= 10)`
- Audit log: app DB user chỉ có INSERT + SELECT (không có UPDATE/DELETE)
- `UNIQUE(user_id, database_id)` trên `user_db_permissions`
- `UNIQUE(request_id, approver_id)` trên `request_approvals`

#### Schema Detailed Design

**Tổng quan schema:** 16 bảng, 13 migration files (001–013).

#### Table Detailed Design

---

**Table: `users`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK, default gen_random_uuid() | Identifier nội bộ |
| oidc_subject | TEXT | NOT NULL, UNIQUE | `sub` claim từ OIDC provider |
| email | TEXT | NOT NULL, UNIQUE | Email từ OIDC |
| display_name | TEXT | NOT NULL | Tên hiển thị |
| is_active | BOOLEAN | NOT NULL, default TRUE | Soft delete |
| created_at | TIMESTAMPTZ | NOT NULL, default NOW() | |
| updated_at | TIMESTAMPTZ | NOT NULL, default NOW() | |

**Table: `user_roles`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| user_id | UUID | FK → users(id) CASCADE | |
| role | TEXT | CHECK IN ('ADMIN','MAKER','CHECKER','APPROVER') | |
| granted_by | UUID | FK → users(id) nullable | Admin cấp |
| granted_at | TIMESTAMPTZ | NOT NULL, default NOW() | |
| — | — | UNIQUE(user_id, role) | |

---

**Table: `database_connections`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| name | TEXT | NOT NULL, UNIQUE | Tên hiển thị, VD: ORACLE_PROD |
| db_type | TEXT | CHECK IN ('postgresql','mysql','oracle','mongodb') | |
| host | TEXT | NOT NULL | |
| port | INTEGER | CHECK (0 < port < 65536) | |
| database_name | TEXT | NOT NULL | |
| schema_name | TEXT | nullable | Oracle/PostgreSQL schema |
| encrypted_credentials | TEXT | NOT NULL | AES-256-GCM JSON {user, password} |
| timeout_secs | INTEGER | default 30, CHECK (0 < x ≤ 300) | Execution timeout |
| approval_levels_required | INT | NOT NULL, default 1, CHECK (1–5) | Số lượt approve |
| is_active | BOOLEAN | NOT NULL, default TRUE | |
| created_at, updated_at | TIMESTAMPTZ | NOT NULL | |

---

**Table: `sql_requests`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| description | TEXT | NOT NULL, length ≥ 1 | Mô tả mục đích |
| sql_content | TEXT | NOT NULL, length ≥ 1 | Câu SQL cần thực thi |
| status | TEXT | CHECK IN (DRAFT/PENDING/APPROVED/REJECTED/EXECUTED/FAILED/CANCELLED) | |
| database_id | UUID | FK → database_connections | |
| created_by | UUID | FK → users | Maker |
| approved_by | UUID | FK → users, nullable | Checker cuối cùng |
| approval_levels_required | INT | NOT NULL, default 1 | Snapshot tại thời điểm submit |
| rejection_reason | TEXT | nullable | Bắt buộc khi REJECTED (≥ 10 ký tự) |
| cr_id | UUID | FK → change_requests, nullable | Liên kết CR |
| executed_at | TIMESTAMPTZ | nullable | |
| rows_affected | BIGINT | nullable | Kết quả thực thi |
| execution_error | TEXT | nullable | Lỗi khi FAILED |
| submitted_at | TIMESTAMPTZ | nullable | |
| created_at, updated_at | TIMESTAMPTZ | NOT NULL | |
| — | — | CONSTRAINT four_eyes_check | approved_by ≠ created_by |
| — | — | CONSTRAINT rejection_reason_required | |

---

**Table: `audit_log`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| request_id | UUID | FK → sql_requests, **nullable** | Null cho admin actions |
| action | TEXT | CHECK IN (CREATED/SUBMITTED/APPROVED/REJECTED/EXECUTED/FAILED/CANCELLED/ROLE_CHANGED/USER_STATUS_CHANGED/CREDENTIAL_ROTATED) | |
| actor_id | UUID | FK → users, NOT NULL | User thực hiện |
| detail | TEXT | nullable | Lý do reject, error message |
| execution_result | BIGINT | nullable | Rows affected khi EXECUTED |
| ip_address | TEXT | nullable | IP của actor |
| timestamp | TIMESTAMPTZ | NOT NULL, default NOW() | |

> App DB user chỉ có quyền INSERT + SELECT — không có UPDATE/DELETE. Lưu giữ ≥ 7 năm.

---

**Table: `user_db_permissions`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| user_id | UUID | FK → users CASCADE | |
| database_id | UUID | FK → database_connections CASCADE | |
| granted_by | UUID | FK → users NOT NULL | Admin cấp |
| granted_at | TIMESTAMPTZ | NOT NULL | |
| — | — | UNIQUE(user_id, database_id) | Mỗi user có 1 bản ghi/DB |

---

**Table: `request_approvals`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| request_id | UUID | FK → sql_requests | |
| approver_id | UUID | FK → users | |
| level | INT | NOT NULL | Approval level (1, 2, …) |
| approved_at | TIMESTAMPTZ | NOT NULL | |
| — | — | UNIQUE(request_id, approver_id) | Mỗi người chỉ approve 1 lần |

---

**Table: `change_requests`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| cr_number | VARCHAR(100) | NOT NULL, UNIQUE | Change ID từ ManageEngine |
| cr_url | TEXT | NOT NULL | URL người dùng nhập |
| title | TEXT | nullable | Từ ManageEngine API |
| status | TEXT | nullable | Từ ManageEngine |
| scheduled_start_time | TIMESTAMPTZ | nullable | Maintenance window start |
| scheduled_end_time | TIMESTAMPTZ | nullable | Maintenance window end |
| synced_at | TIMESTAMPTZ | nullable | Lần cuối sync |
| created_by | UUID | FK → users | |
| created_at | TIMESTAMPTZ | NOT NULL | |

---

**Table: `data_query_requests`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| title | TEXT | NOT NULL | |
| database_id | UUID | FK → database_connections | |
| schema_name | TEXT | NOT NULL | |
| sql_query | TEXT | NOT NULL | SELECT query — immutable sau APPROVED |
| tables_accessed | TEXT[] | NOT NULL, default '{}' | Bảng được truy cập |
| columns_accessed | TEXT[] | NOT NULL, default '{}' | Cột được truy cập |
| query_mode | TEXT | CHECK IN ('BASIC','ADVANCED') | |
| ticket_id | TEXT | NOT NULL | Ticket bắt buộc |
| status | TEXT | CHECK IN (DRAFT/PENDING_APPROVAL/APPROVED/REJECTED/EXPIRED/REVOKED) | |
| created_by | UUID | FK → users | |
| current_approval_level | INT | NOT NULL, default 0 | |
| total_approval_levels | INT | NOT NULL, default 1 | |
| expires_at | TIMESTAMPTZ | nullable | Hết hạn query |
| approved_at, revoked_at | TIMESTAMPTZ | nullable | |
| revoke_reason | TEXT | nullable | |
| created_at, updated_at | TIMESTAMPTZ | NOT NULL | |

---

**Table: `data_query_assigned_users`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| query_request_id | UUID | FK → data_query_requests CASCADE | |
| user_id | UUID | FK → users CASCADE | |
| assigned_at | TIMESTAMPTZ | NOT NULL | |
| — | — | UNIQUE(query_request_id, user_id) | |

---

**Table: `data_query_approval_rules`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| database_id | UUID | FK → database_connections CASCADE | |
| schema_name | TEXT | nullable | null = tất cả schema |
| table_name | TEXT | nullable | null = tất cả table |
| created_by | UUID | FK → users | DBA/Admin |
| is_active | BOOLEAN | NOT NULL, default TRUE | |
| created_at, updated_at | TIMESTAMPTZ | NOT NULL | |
| — | — | UNIQUE INDEX (database_id, COALESCE(schema,'\*'), COALESCE(table,'\*')) WHERE is_active | |

---

**Table: `data_query_approval_levels`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| rule_id | UUID | FK → data_query_approval_rules CASCADE | |
| level_order | INT | NOT NULL | Thứ tự phê duyệt |
| approver_unit | TEXT | NOT NULL | Tên unit (tham chiếu approver_units.name) |
| required | BOOLEAN | NOT NULL, default TRUE | |
| — | — | UNIQUE(rule_id, level_order) | |

---

**Table: `data_query_approval_tracking`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| query_request_id | UUID | FK → data_query_requests CASCADE | |
| level_order | INT | NOT NULL | |
| approver_unit | TEXT | NOT NULL | |
| status | TEXT | CHECK IN ('PENDING','APPROVED','REJECTED','SKIPPED') | |
| approved_by | UUID | FK → users, nullable | |
| comment | TEXT | nullable | |
| acted_at | TIMESTAMPTZ | nullable | |
| — | — | UNIQUE(query_request_id, level_order) | |

---

**Table: `data_query_executions`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| query_request_id | UUID | FK → data_query_requests | |
| executed_by | UUID | FK → users | Assigned user |
| rows_returned | BIGINT | NOT NULL, default 0 | |
| execution_time_ms | BIGINT | NOT NULL, default 0 | |
| client_ip | TEXT | nullable | |
| status | TEXT | CHECK IN ('SUCCESS','FAILED','TIMEOUT') | |
| error_message | TEXT | nullable | |
| executed_at | TIMESTAMPTZ | NOT NULL | |

---

**Table: `approver_units`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| name | TEXT | NOT NULL, UNIQUE | Ví dụ: ITO, ITS, COMPLIANCE |
| description | TEXT | nullable | |
| is_active | BOOLEAN | NOT NULL, default TRUE | |
| created_at | TIMESTAMPTZ | NOT NULL | |

---

**Table: `user_approver_units`**

| Column | Type | Constraint | Mô tả |
|--------|------|-----------|-------|
| id | UUID | PK | |
| user_id | UUID | FK → users CASCADE | |
| unit_id | UUID | FK → approver_units CASCADE | |
| granted_by | UUID | FK → users, nullable | |
| granted_at | TIMESTAMPTZ | NOT NULL | |
| — | — | UNIQUE(user_id, unit_id) | |

---

# 4. Infrastructure Design and Operation

## Infrastructure Architecture

### Deployment Architecture

```
                    ┌──────────────────────────────────────────┐
                    │          Internal Network / VPN           │
                    └──────────────────────┬───────────────────┘
                                           │
                                 ┌─────────▼──────────┐
                                 │   Reverse Proxy     │
                                 │  (Nginx / Traefik)  │
                                 │  TLS termination    │
                                 └─────────┬──────────┘
                               ┌───────────┴────────────┐
                               │                        │
                    ┌──────────▼──────┐      ┌──────────▼──────┐
                    │  SECS Frontend  │      │  SECS Backend   │
                    │  Svelte/Nginx   │      │  Rust/Axum      │
                    │  :80 (static)   │      │  :3000          │
                    └─────────────────┘      └──────┬──────────┘
                                                    │
                        ┌───────────────────────────┼──────────────────────────┐
                        │                           │                          │
             ┌──────────▼──────────┐   ┌────────────▼─────────┐   ┌──────────▼──────┐
             │     PostgreSQL      │   │  Keycloak (OIDC)     │   │  ManageEngine   │
             │     (secs_db)       │   │  :8080               │   │  ServiceDesk+   │
             │  State + Audit log  │   │  SSO Provider        │   │  (optional)     │
             └─────────────────────┘   └──────────────────────┘   └─────────────────┘
                                                    │
                        ┌───────────────────────────┼──────────────────┐
                        │                           │                  │
             ┌──────────▼──────┐       ┌────────────▼──────┐  ┌───────▼──────┐
             │   PostgreSQL    │       │      MySQL         │  │   Oracle     │
             │   (target)      │       │   (target)         │  │  (target)    │
             └─────────────────┘       └───────────────────┘  └──────────────┘
                                                    │
                                       ┌────────────▼──────┐
                                       │    MongoDB         │
                                       │    (target)        │
                                       └───────────────────┘
```

**Cấu hình triển khai Docker Compose (development/on-premise):**

| Service | Image | Port | Mô tả |
|---------|-------|------|-------|
| `secs_postgres` | postgres:16-alpine | 5432 | Database nội bộ |
| `secs_keycloak` | keycloak:24.0 | 8080 | OIDC provider |
| `secs_backend` | Dockerfile | 3000 | Rust/Axum API |
| `secs_frontend` | Dockerfile + nginx | 80/443 | Svelte SPA |
| `secs_mysql_target` | mysql:8.0 | 3306 | MySQL test target |
| `secs_mongodb_target` | mongo:7.0 | 27017 | MongoDB test target (ReplicaSet) |

### Network Architecture

- Backend và PostgreSQL nội bộ không expose ra ngoài — chỉ giao tiếp trong internal network
- Frontend serve static files qua Nginx, proxy API request về backend
- Keycloak chỉ cần accessible từ browser (redirect) và backend (token exchange)
- Database đích chỉ accessible từ backend (không expose ra ngoài)
- Toàn bộ traffic production phải qua HTTPS (TLS termination tại reverse proxy)

---

## Infrastructure Sizing

| Component | Dev/Staging | Production |
|-----------|-------------|------------|
| Backend (Rust) | 0.5 vCPU, 512MB RAM | 2 vCPU, 2GB RAM |
| PostgreSQL nội bộ | 1 vCPU, 1GB RAM | 4 vCPU, 8GB RAM, SSD |
| Keycloak | 1 vCPU, 1GB RAM | 2 vCPU, 2GB RAM |
| Frontend (Nginx) | 0.1 vCPU, 128MB RAM | 0.5 vCPU, 256MB RAM |

Với volume: ~100 user nội bộ, ~200 request/ngày, sizing production trên là đủ với biên dự phòng.

---

## Database Infrastructure

**PostgreSQL nội bộ (`secs_db`):**

- Version: PostgreSQL 16
- Charset: UTF-8
- Timezone: UTC (tất cả TIMESTAMPTZ lưu UTC)
- Connection pool: `sqlx` PgPool, max connections = 10 (mặc định)
- App DB user (`secs_user`): quyền tối thiểu — DML trên tất cả bảng, **KHÔNG** có quyền DELETE trên `audit_log`
- 13 migration files, quản lý bằng `sqlx migrate`

**Indexes được tạo:**

| Index | Bảng | Mục đích |
|-------|------|---------|
| idx_sql_requests_status | sql_requests | Filter theo status |
| idx_sql_requests_created_by | sql_requests | Maker xem request của mình |
| idx_sql_requests_database_id | sql_requests | Filter theo database |
| idx_audit_log_request_id | audit_log | Lấy audit của 1 request |
| idx_audit_log_timestamp | audit_log | Truy vấn theo thời gian |
| idx_user_db_permissions_user_id | user_db_permissions | Check quyền nhanh |
| idx_user_roles_user_id | user_roles | Lookup roles |
| idx_dqr_status | data_query_requests | Filter active queries |
| idx_dqe_date | data_query_executions | Audit trail theo ngày |

---

## Archiving

- **Audit log**: Giữ nguyên trong database ≥ 7 năm (SOX requirement). App DB user không có quyền DELETE.
- **Completed requests**: Giữ nguyên — không xóa. Dữ liệu lịch sử phục vụ audit.
- **Chiến lược partition (nếu volume tăng)**: Partition `audit_log` theo năm bằng PostgreSQL table partitioning.
- **Logs file**: Daily rotation với `tracing-appender`, giữ 90 ngày.

---

## Backup and Restore

**PostgreSQL nội bộ:**

| Loại backup | Tần suất | Retention | Công cụ |
|------------|---------|-----------|---------|
| Full backup | Hàng ngày | 30 ngày | pg_dump / pgBackRest |
| WAL archiving | Liên tục | 7 ngày | pgBackRest / WAL-G |
| Point-in-time recovery | — | 7 ngày | Từ WAL archive |

**Quy trình restore:**
1. Dừng backend service
2. Restore từ pg_dump hoặc PITR
3. Chạy lại `sqlx migrate run` để verify schema
4. Khởi động lại backend
5. Verify bằng `/health` endpoint

**RTO**: < 1 giờ | **RPO**: < 24 giờ (full backup) / < 5 phút (với WAL archiving)

---

## Disaster Recovery

| Scenario | Giải pháp | RTO |
|---------|----------|-----|
| Backend crash | Docker restart policy `unless-stopped` | < 1 phút |
| PostgreSQL corruption | Restore từ backup + WAL | < 1 giờ |
| Keycloak failure | Restore từ backup; user vẫn login bằng JWT còn hạn (8h) | < 2 giờ |
| Datacenter failure | Deploy sang datacenter dự phòng từ backup | < 4 giờ |

DR drill: test restore procedure mỗi quý.

---

## Compliance

| Yêu cầu | Giải pháp |
|---------|----------|
| SOX — Audit trail | `audit_log` append-only, 7 năm retention, không thể xóa qua app |
| Separation of duties | 4-eyes principle enforce tại DB constraint |
| Access control | RBAC + explicit DB permission, không có global access |
| Credential management | AES-256-GCM encryption, không log, không trả về qua API |
| Change Management | Liên kết Change Request bắt buộc (khi MANAGEENGINE_ENABLED=true) |

---

## Monitoring and Logging

**Application Logging:**

- Format: JSON (stdout) + file rotation (daily) tại `LOG_DIR`
- Level: `info` cho production, `debug` cho module `secs_backend`
- Mỗi HTTP request log: method, URI, status code, latency_ms (qua TraceLayer)
- Không log SQL content chứa dữ liệu nhạy cảm
- Không log credentials

**Health Check:**

```
GET /health
Response: { "status": "ok", "db": "connected" }
```

Kiểm tra: PostgreSQL connection pool còn active.

**Metrics (nếu tích hợp Prometheus):**

| Metric | Mô tả |
|--------|-------|
| http_requests_total | Tổng số request theo method/path/status |
| http_request_duration_seconds | Latency histogram |
| sql_execution_total | Tổng lần thực thi SQL theo status |
| sql_execution_duration_seconds | Thời gian thực thi SQL |
| db_pool_connections | Số connection PostgreSQL đang active |

---

# 5. Security Design

## System Classification

**Phân loại hệ thống**: **Critical Internal System**

- Hệ thống kiểm soát thực thi SQL trực tiếp trên database production
- Mọi thao tác đều tác động đến dữ liệu production
- Yêu cầu tuân thủ: SOX, nội kiểm tài chính
- Classification: **CONFIDENTIAL** (dữ liệu SQL, audit log) — **RESTRICTED** (credentials database đích)

---

## Data Classification

| Loại dữ liệu | Phân loại | Xử lý |
|-------------|-----------|-------|
| Credentials database đích | RESTRICTED | AES-256-GCM encrypted at rest; không log; không trả về qua API |
| SQL content | CONFIDENTIAL | Không trả về trong list API; chỉ trong detail với quyền |
| Audit log | CONFIDENTIAL | Append-only; lưu 7 năm |
| JWT token | CONFIDENTIAL | HS256, TTL 8h; không log |
| User email, tên | INTERNAL | Trả về trong API khi cần; không log ra file |
| Database metadata (host, port, name) | INTERNAL | Có thể xem qua admin API |

---

## Compliance

| Framework | Yêu cầu | Giải pháp |
|-----------|---------|----------|
| SOX | Separation of duties | 4-eyes principle (maker ≠ checker), enforce tại DB constraint |
| SOX | Audit trail | `audit_log` immutable, 7 năm, với actor + timestamp + IP |
| SOX | Access control | RBAC + explicit DB permission, không có implicit access |
| Internal | Change Management | CR integration với ManageEngine |
| Security | Credential protection | AES-256-GCM, ENCRYPTION_KEY qua env, không hardcode |

---

## Network Security

- **TLS bắt buộc** cho tất cả kết nối đến database đích (trừ dev với `DISABLE_TARGET_DB_TLS=true`)
- **CORS**: Chỉ chấp nhận origin từ `FRONTEND_URL` cấu hình
- **Trusted Proxies**: Chỉ trust `X-Forwarded-For` header từ IP/CIDR trong `TRUSTED_PROXIES`
- **Network segmentation**: Backend và DB nội bộ không expose ra ngoài — chỉ qua reverse proxy
- **Body limit**: Tối đa 100KB/request để ngăn abuse

---

## Application Security

**Authentication:**
- OIDC/OAuth2 với PKCE flow — không có password riêng trong SECS
- JWT HS256, TTL 8 giờ, issuer kiểm tra
- `at_hash` claim validation để ngăn token substitution

**Authorization:**
- Role-based: Admin / Maker / Checker / Approver — kiểm tra ở từng endpoint
- Database permission: Phải được cấp explicit cho từng database (không có global)
- 4-eyes principle: Enforce tại cả application layer và DB constraint

**Input Validation:**
- `validator` crate cho request body
- `sqlx` parameterized queries — không interpolate user input vào SQL nội bộ
- Body size limit 100KB

**Rate Limiting:**
- Login: 10 request/min/IP (`RateLimiter` in-memory)
- Global: `GlobalRateLimiter` trên tất cả protected routes

**Security Headers:**

```rust
// Áp dụng cho tất cả responses
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Referrer-Policy: strict-origin-when-cross-origin
Content-Security-Policy: default-src 'self'
```

---

## Data Security

- **Credentials encryption**: AES-256-GCM với 32-byte key từ env `ENCRYPTION_KEY` — không bao giờ log hoặc trả về qua API
- **SQL content**: Không trả về trong list endpoint — chỉ trong detail endpoint
- **JWT secret**: HS256, minimum 32 chars, từ env `JWT_SECRET`
- **Audit log immutability**: DB user không có quyền DELETE/UPDATE trên `audit_log`
- **Soft delete**: User account dùng `is_active=false` thay vì DELETE — giữ audit trail toàn vẹn

---

## Integration Security

**Keycloak (OIDC):**
- PKCE (Proof Key for Code Exchange) — ngăn authorization code interception
- CSRF state parameter validation
- `at_hash` claim validation
- Client secret trong env, không hardcode

**ManageEngine (nếu enabled):**
- API Auth Token trong env `MANAGEENGINE_AUTH_TOKEN`
- HTTPS bắt buộc cho API calls
- Data từ ManageEngine được treat là cache — không là system of record

**Database đích:**
- Kết nối qua TLS (sslmode=require / ssl-mode=REQUIRED / TCPS)
- Service account với quyền tối thiểu (chỉ DML, không DDL admin)
- Credentials encrypted AES-256-GCM, chỉ decrypt tại thời điểm thực thi

---

## Security Monitoring

### Infrastructure Logging

| Sự kiện | Log level | Nơi log |
|---------|----------|---------|
| Startup / shutdown | INFO | stdout + file |
| DB pool health | INFO | stdout + file |
| HTTP request (method, URI, status, latency) | INFO | stdout + file |
| Rate limit triggered | WARN | stdout + file |
| Connection pool exhausted | ERROR | stdout + file |

### Application Logging (SECS)

| Sự kiện | Log level | Audit log DB |
|---------|----------|-------------|
| User login (OIDC callback) | INFO | — |
| SQL request created | INFO | ✓ CREATED |
| SQL request submitted | INFO | ✓ SUBMITTED |
| SQL request approved | INFO | ✓ APPROVED |
| SQL request rejected | INFO | ✓ REJECTED |
| SQL executed successfully | INFO | ✓ EXECUTED (rows_affected) |
| SQL execution failed | ERROR | ✓ FAILED (error_message) |
| SQL execution timeout | WARN | ✓ FAILED (timeout) |
| SQL request cancelled | INFO | ✓ CANCELLED |
| User role changed | INFO | ✓ ROLE_CHANGED |
| User status changed | INFO | ✓ USER_STATUS_CHANGED |
| Credentials rotated | INFO | ✓ CREDENTIAL_ROTATED |
| 4-eyes violation attempt | WARN | — |
| Rate limit triggered | WARN | — |

> Tất cả audit log entry đều ghi: `actor_id`, `ip_address`, `timestamp`, `detail`.

---

## User Management

**Vòng đời user:**

```
SSO Login → upsert users (sub, email, display_name) →
  Admin gán role(s) → Admin cấp DB permission(s) →
  User sử dụng hệ thống

Deactivate: Admin set is_active=false →
  User không thể login (JWT validate reject inactive user) →
  Audit log và lịch sử request giữ nguyên (soft delete)
```

**Quy tắc:**
- Không tự đăng ký — tài khoản được tạo khi lần đầu login SSO
- Không có role mặc định — Admin phải gán explicit
- Role có thể nhiều cùng lúc (ví dụ: vừa MAKER vừa CHECKER, nhưng không thể tự approve)
- Rút quyền DB permission: request đang PENDING không bị ảnh hưởng

---

## Security Control

| Control | Loại | Triển khai |
|---------|------|-----------|
| Authentication | Preventive | OIDC/SSO + JWT |
| Authorization | Preventive | RBAC + DB permission check |
| 4-eyes principle | Preventive | DB constraint + application check |
| Rate limiting | Preventive | In-memory per-IP |
| Credential encryption | Preventive | AES-256-GCM |
| TLS | Preventive | TLS required for all DB connections |
| Audit logging | Detective | Append-only audit_log, 7 năm |
| SQL DDL detection | Detective | sql_analyzer.rs — cảnh báo cho Checker |
| Body size limit | Preventive | 100KB max |
| Security headers | Preventive | HSTS, CSP, X-Frame-Options |
| CORS | Preventive | Whitelist origin |
| Trusted proxies | Preventive | Whitelist IP/CIDR |

---

## Security Risk Assessment

| Risk | Likelihood | Impact | Giải pháp |
|------|-----------|--------|----------|
| Credential theft (DB đích) | Medium | Critical | AES-256-GCM encryption; không log; rotate via API |
| Insider threat (tự approve) | Medium | High | 4-eyes principle enforce tại DB constraint |
| SQL injection vào PostgreSQL nội bộ | Low | High | sqlx parameterized queries — không dynamic SQL |
| Token hijacking | Low | High | JWT TTL 8h; HTTPS required |
| Brute force login | Medium | Medium | Rate limit 10/min/IP; Keycloak brute-force protection |
| Audit log tampering | Low | Critical | App DB user không có DELETE/UPDATE trên audit_log |
| Unauthorized DB access | Medium | Critical | Explicit permission required; no global access |
| DDL executed without awareness | Low | High | DDL detection cảnh báo Checker; Checker phải explicit approve |

---

## Architecture Decision Records

**ADR-001: Synchronous SQL Execution**
- **Quyết định**: Thực thi SQL synchronously trong cùng HTTP request (không dùng queue)
- **Lý do**: Volume thấp (nội bộ), đơn giản kiến trúc, Checker nhận kết quả ngay lập tức
- **Trade-off**: Nếu SQL chạy lâu (gần timeout), HTTP connection giữ open lâu

**ADR-002: JWT thay vì server-side session**
- **Quyết định**: Stateless JWT HS256, TTL 8h
- **Lý do**: Backend stateless — sẵn sàng horizontal scale; không cần Redis
- **Trade-off**: Không thể revoke token ngay lập tức (phải chờ hết 8h)

**ADR-003: Append-only audit log tại DB level**
- **Quyết định**: App DB user không có quyền DELETE/UPDATE trên `audit_log`
- **Lý do**: Đảm bảo immutability ngay tại tầng database — application bug không thể xóa audit
- **Trade-off**: Cần DBA account để xử lý data issues (nếu có)

**ADR-004: AES-256-GCM cho credentials**
- **Quyết định**: Encrypt credentials database đích bằng AES-256-GCM trước khi lưu PostgreSQL
- **Lý do**: PostgreSQL nội bộ có thể bị dump — credentials phải được bảo vệ ngay cả khi DB bị compromise
- **Trade-off**: ENCRYPTION_KEY phải được bảo vệ riêng (env variable / secrets manager)

**ADR-005: Không dùng Redis hay RabbitMQ**
- **Quyết định**: Rate limiting in-memory; không có message queue
- **Lý do**: Deployment đơn giản, volume nội bộ thấp, giảm operational complexity
- **Trade-off**: Rate limiting reset khi backend restart; không scale ngang cho rate limit

**ADR-006: Keycloak làm OIDC provider**
- **Quyết định**: Dùng Keycloak self-hosted thay vì tự implement auth
- **Lý do**: Tích hợp SSO công ty, battle-tested, hỗ trợ nhiều protocol
- **Trade-off**: Thêm một service cần maintain; Keycloak downtime ảnh hưởng login (JWT còn hạn vẫn hoạt động)

**ADR-007: Approver Units thay vì hardcode roles**
- **Quyết định**: `approver_units` là bảng cấu hình (admin-defined), không hardcode ITO/ITS/COMPLIANCE
- **Lý do**: Linh hoạt với cấu trúc tổ chức thay đổi; không cần deploy lại khi thêm unit mới
- **Trade-off**: Thêm độ phức tạp trong UI quản lý

---

*Tài liệu này được tạo ngày: 2025 | Phiên bản: 1.0 | Hệ thống: SECS v1.0*
