---
inclusion: auto
description: "Thiết kế hệ thống SECS — kiến trúc, API design, UX flow, security, và NFR requirements"
---

# Thiết kế hệ thống SECS — Đầy đủ, Hiệu quả, An toàn, Dễ dùng

Tài liệu này tổng hợp toàn bộ yêu cầu chức năng, phi chức năng, UX, và compliance
cho hệ thống SQL Execution Control System. Dùng làm nguồn tham chiếu khi thiết kế
và implement từng tính năng.

---

## 1. Tại sao cần hệ thống này — Bối cảnh compliance

Hệ thống SECS thuộc nhóm **Database Change Management (DCM)** — một yêu cầu bắt buộc
trong các công ty tài chính để đáp ứng:

- **SOX (Sarbanes-Oxley)**: Yêu cầu lưu giữ audit trail đầy đủ cho mọi thay đổi dữ liệu
  tài chính, tối thiểu 7 năm. Mọi thay đổi phải có người ủy quyền riêng biệt với người thực hiện.
- **DORA (EU Digital Operational Resilience Act, hiệu lực 2025)**: Tổ chức tài chính phải
  chứng minh khả năng kiểm soát và truy vết mọi thao tác trên hệ thống production.
- **Internal audit**: Kiểm soát viên nội bộ cần truy vết được ai làm gì, khi nào, tại sao.

**Rủi ro nếu không có SECS**: Một lệnh `UPDATE accounts SET balance = 0 WHERE 1=1` chạy
nhầm trên PROD có thể gây thiệt hại không thể phục hồi và vi phạm quy định nghiêm trọng.

---

## 2. Yêu cầu chức năng — Functional Requirements

### 2.1 Core: Approval Workflow (Maker-Checker)

**Maker (người tạo request):**
- [ ] Tạo SQL request ở trạng thái DRAFT, có thể edit tự do
- [ ] Chọn database đích và schema cụ thể (chỉ thấy DB mình có quyền)
- [ ] Nhập mô tả lý do thực thi (bắt buộc — để Checker hiểu ngữ cảnh)
- [ ] Xem preview phân tích SQL: loại statement, DDL/DML, số statement, ước tính impact
- [ ] Submit để Checker review (DRAFT → PENDING, sau đó immutable)
- [ ] Hủy request đang PENDING của mình (PENDING → CANCELLED)
- [ ] Xem toàn bộ history của request: ai comment gì, approve/reject lúc nào
- [ ] Clone một request cũ để tạo request mới tương tự (tiết kiệm thời gian)
- [ ] Nhận notification realtime khi request được approve/reject

**Checker (người review):**
- [ ] Xem danh sách request PENDING — sorted by submitted_at, filter by database
- [ ] Xem chi tiết SQL với syntax highlighting
- [ ] Xem SQL analysis: DDL warning, số rows có thể bị ảnh hưởng (ước tính)
- [ ] Thêm comment/ghi chú trước khi quyết định (audit trail)
- [ ] Approve — trigger thực thi tự động
- [ ] Reject kèm lý do bắt buộc (min 10 ký tự) — Maker nhận notification
- [ ] Yêu cầu Maker chỉnh sửa (request clarification) — không reject hẳn
- [ ] Nhận notification realtime khi có request mới cần review

**Admin:**
- [ ] Quản lý users: xem, deactivate, gán/thu role
- [ ] Quản lý database connections: CRUD, test kết nối
- [ ] Cấp/thu quyền user trên từng database
- [ ] Xem audit log toàn hệ thống với filter và export
- [ ] Cấu hình timeout mặc định cho từng database
- [ ] Xem dashboard tổng quan: số request theo status, theo user, theo database

### 2.2 SQL Execution Engine

- [ ] Thực thi bất đồng bộ — không block HTTP request sau khi approve
- [ ] Wrap trong transaction — rollback toàn bộ nếu một statement lỗi
- [ ] Timeout theo cấu hình (mặc định 30s, tối đa 300s)
- [ ] Ghi lại số rows affected sau khi execute thành công
- [ ] Ghi lại error message chi tiết khi FAILED (nhưng không log ra console nếu có sensitive data)
- [ ] Hỗ trợ multi-statement: `INSERT ...; UPDATE ...; DELETE ...;`
- [ ] Hiển thị execution result cho cả Maker và Checker

### 2.3 SQL Analysis (Static Analysis trước khi submit)

- [ ] Phát hiện DDL: DROP, TRUNCATE, ALTER, CREATE, RENAME, COMMENT ON
- [ ] Phân loại statement: SELECT / DML (INSERT, UPDATE, DELETE) / DDL / Other
- [ ] Đếm số statements (tách bởi `;`)
- [ ] Warn nếu không có WHERE clause trong UPDATE/DELETE (có thể ảnh hưởng toàn bộ bảng)
- [ ] Warn nếu có `LIMIT` / `TOP` quá lớn
- [ ] Detect pattern nguy hiểm: `WHERE 1=1`, `DELETE FROM table` không có WHERE

### 2.4 Notification System

- [ ] Realtime qua WebSocket (đã kết nối trong app)
- [ ] Notification bell ở navbar với badge số chưa đọc
- [ ] Notification log: lịch sử 30 ngày, đánh dấu đã đọc
- [ ] Nội dung notification đủ context: tên request, tên người, action, database
  - Tốt: "Nguyễn Văn A đã approve request #123 — UPDATE accounts trên DB PROD_ORACLE"
  - Xấu: "Request được approve"

### 2.5 Dashboard & Báo cáo

**Maker dashboard:**
- [ ] Số request theo status (stats cards)
- [ ] Danh sách request gần đây (10 items)
- [ ] Số request đang PENDING chờ duyệt
- [ ] Average time to approval (SLA tracking)

**Checker dashboard:**
- [ ] Số request đang chờ duyệt (nổi bật nhất)
- [ ] Oldest pending request (alert nếu > 24h)
- [ ] Request throughput tuần này vs tuần trước

**Admin dashboard:**
- [ ] Tổng số request theo trạng thái
- [ ] Top Makers (theo số request)
- [ ] Top databases (theo tần suất)
- [ ] Failed executions trong 7 ngày
- [ ] Active users hôm nay

### 2.6 Audit Log & Báo cáo

- [ ] Export audit log ra CSV / Excel (cho kiểm toán viên)
- [ ] Filter theo: date range, user, database, action, status
- [ ] Immutable: không ai có thể xóa hoặc sửa audit log (kể cả Admin)
- [ ] Retention policy: giữ tối thiểu 7 năm (theo SOX)
- [ ] Mỗi record bao gồm: timestamp, actor, action, request_id, detail, IP address

---

## 3. Yêu cầu phi chức năng — Non-Functional Requirements (NFR)

### 3.1 Performance

| Metric | Target | Ghi chú |
|--------|--------|---------|
| API response time (p95) | < 300ms | Không tính SQL execution |
| Page load time | < 2s | First Contentful Paint |
| WebSocket latency | < 500ms | Notification delivery |
| DB query time (p99) | < 100ms | Queries trên PostgreSQL nội bộ |
| Concurrent users | 100+ | Nội bộ công ty |
| SQL execution timeout | Cấu hình được, mặc định 30s | |

**Cách đạt được:**
- Connection pool PostgreSQL: min 2, max 20 connections
- Index đầy đủ trên các cột filter thường dùng: `status`, `created_by`, `database_id`, `timestamp`
- Pagination tất cả list API (mặc định 20 items/page, tối đa 100)
- Lazy load audit trail (chỉ fetch khi mở detail panel)
- Svelte skeleton loading để tránh layout shift khi fetch

### 3.2 Availability & Reliability

| Metric | Target |
|--------|--------|
| Uptime | 99.5% (nội bộ, không phải 24/7 production) |
| Recovery Time Objective (RTO) | < 30 phút |
| Recovery Point Objective (RPO) | < 1 giờ (daily backup PostgreSQL) |
| Graceful degradation | App vẫn chạy nếu WebSocket disconnect |

**Cách đạt được:**
- Auto-reconnect WebSocket phía client (exponential backoff, max 5 lần)
- `sqlx migrate run` tự động khi startup — không cần manual migration
- Health check endpoint `GET /health` — trả 200 nếu DB pool OK
- Structured logging với `tracing` — dễ debug khi có incident

### 3.3 Security

| Yêu cầu | Biện pháp |
|---------|-----------|
| Authentication | SSO/OIDC — không có local password |
| Authorization | JWT per-request, role check trong middleware |
| 4-eyes principle | Enforce DB constraint + application layer |
| Credential storage | AES-256-GCM encrypt trước khi lưu PostgreSQL |
| SQL content | Không log, không trả về trong list API response (chỉ trong detail) |
| Audit log | Append-only, database user không có DELETE privilege trên bảng audit_log |
| TLS | Bắt buộc cho tất cả kết nối đến database đích |
| CORS | Chỉ allow origin của frontend, không allow * |
| Rate limiting | Max 10 requests/minute trên `/auth/login` |
| Input validation | Tất cả request body phải qua validator trước khi xử lý |
| Session | JWT max 8 giờ, không có refresh token (re-login qua SSO) |
| IP logging | Ghi IP address vào audit log cho mọi action |
| **HTTP Security Headers** | **`Content-Security-Policy: default-src 'self'`, `Strict-Transport-Security: max-age=31536000; includeSubDomains`, `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: strict-origin-when-cross-origin`** |
| **Alerting** | **Alert khi: nhiều lần auth fail liên tiếp (> 5 lần/phút), authorization violation, pending requests > 20** |

### 3.4 Maintainability

- **Code coverage**: tối thiểu 70% cho business logic (`db/` và `executor/`)
- **Linting**: `cargo clippy -- -D warnings` phải pass — bắt buộc trong CI
- **Formatting**: `cargo fmt` + Prettier cho frontend — không merge nếu diff
- **Dependency audit**: `cargo audit` mỗi tuần để phát hiện CVE
- **Migration versioning**: Tất cả schema change qua `sqlx migrate`, không sửa trực tiếp DB
- **Environment parity**: Docker Compose cho local = gần giống production nhất có thể

### 3.5 Usability (UX Requirements)

- **Time to complete core task**: Maker tạo và submit request trong < 2 phút
- **Error messages**: Rõ ràng, actionable — không phải "Internal Server Error"
- **Loading states**: Tất cả async action có skeleton hoặc spinner
- **Confirmation dialogs**: Bắt buộc cho Approve và Reject (action không thể hoàn tác)
- **Responsive**: Hoạt động trên màn hình 1280px trở lên (internal tool, không cần mobile)
- **Accessibility**: WCAG 2.1 AA — keyboard navigation, screen reader labels
- **Feedback ngay lập tức**: Toast notification sau mọi action thành công/thất bại

### 3.6 Observability

- **Structured logs**: JSON format với `tracing` — indexable bởi log aggregator
- **Request tracing**: Mỗi HTTP request có `request_id` duy nhất trong log
- **Error tracking**: Log đầy đủ stack trace cho lỗi 5xx
- **Metrics** (tùy chọn phase 2): Prometheus metrics endpoint `/metrics`
  - `secs_requests_total` — số request theo status
  - `secs_execution_duration_seconds` — thời gian thực thi SQL
  - `secs_pending_requests_count` — số request đang chờ duyệt (alert nếu > 20)

---

## 4. Tính năng nâng cao (Phase 2)

Các tính năng này không cần thiết cho v1 nhưng quan trọng về dài hạn:

### 4.1 Dry Run / Explain

Trước khi Checker approve, cho phép chạy `EXPLAIN` (PostgreSQL/MySQL) hoặc tương đương
để ước tính số rows bị ảnh hưởng. Hiển thị kết quả trong ReviewDetail để Checker
đưa ra quyết định có thông tin hơn.

```
[Dry Run Result]
Estimated rows affected: 1,523
Execution plan: Sequential Scan on accounts (cost=0.00..45.2 rows=1523)
⚠️ Full table scan detected — missing index on status column
```

### 4.2 Scheduled Execution

Cho phép Checker approve nhưng lên lịch thực thi vào thời điểm cụ thể
(ví dụ: 2:00 AM khi tải thấp):
- `APPROVED` → `SCHEDULED` → `EXECUTED`
- Cần cron job / `tokio` periodic task để pick up scheduled requests

### 4.3 Template Library

Maker có thể lưu SQL request thường dùng thành template:
- Template có placeholder: `UPDATE orders SET status = '{{new_status}}' WHERE order_id = '{{order_id}}'`
- Khi dùng template, Maker chỉ cần điền giá trị — không cần viết SQL từ đầu
- Giảm lỗi, tăng tốc độ, Checker dễ review hơn vì quen với pattern

### 4.4 Multi-level Approval

Một số SQL nhạy cảm (DDL, ảnh hưởng > 10,000 rows) yêu cầu 2 Checker approve:
- `PENDING` → `PARTIALLY_APPROVED` (1/2 checkers) → `APPROVED` (2/2 checkers)
- Mỗi Checker approve độc lập, không thấy decision của người kia trước
- Configurable theo database hoặc SQL type

### 4.5 Email Notification

Bổ sung kênh email bên cạnh WebSocket:
- Checker nhận email khi có request PENDING mới
- Maker nhận email khi request được approve/reject
- Daily digest cho Checker: số request pending

### 4.6 Request Comments / Discussion Thread

Thay vì chỉ approve hoặc reject, cho phép thread discussion:
- Checker: "Câu này ảnh hưởng bảng gì? Có cần backup trước không?"
- Maker: "Đã backup rồi, bảng accounts_archive"
- Checker: "OK, approve"
- Toàn bộ conversation lưu vào audit trail

### 4.7 Role per Database (Granular RBAC)

Hiện tại role là global (MAKER/CHECKER/ADMIN toàn hệ thống).
Nâng cấp: một user có thể là MAKER trên DB_ORACLE_PROD và CHECKER trên DB_PG_TEST.
Phức tạp hơn nhưng phù hợp với tổ chức có nhiều team.

### 4.8 Bulk Operations

Admin có thể approve/reject nhiều request cùng lúc (batch approval):
- Chỉ cho phép nếu tất cả request thuộc cùng một database
- Require double confirmation

---

## 5. UX Design Principles — Thiết kế giao diện

### 5.1 Layout tổng thể

```
┌─────────────────────────────────────────────────────┐
│  NAVBAR: Logo | Nav links | Notifications 🔔 | User  │
├──────────┬──────────────────────────────────────────┤
│          │                                          │
│ SIDEBAR  │  MAIN CONTENT                           │
│ (drawer) │  - Page title                           │
│          │  - Action bar (filters, buttons)        │
│ Dashboard│  - Content (table / form / detail)      │
│ Requests │                                         │
│ Checker  │                                         │
│ Admin ▼  │                                         │
│   Users  │                                         │
│   DBs    │                                         │
│          │                                         │
└──────────┴──────────────────────────────────────────┘
```

Dùng `drawer` component của DaisyUI cho sidebar. Sidebar collapse được trên màn hình nhỏ.

### 5.2 ReviewDetail — Trang quan trọng nhất với Checker

```
┌─── Request #abc-123 ───────────────────────────────────────┐
│ [PENDING] UPDATE accounts — DB: ORACLE_PROD_ACCOUNTS       │
│ Submitted by: Nguyễn Văn A | 2h ago                       │
├────────────────────────────────────────────────────────────┤
│ DESCRIPTION                                                │
│ "Cập nhật trạng thái tài khoản cho batch import ngày 10/7" │
├──────────────────────────────────────────────────────────┤
│ SQL CONTENT                    │ ANALYSIS                 │
│ ┌──────────────────────────┐   │ ✅ DML only              │
│ │ UPDATE accounts          │   │ ✅ 1 statement           │
│ │ SET status = 'ACTIVE'    │   │ ⚠️ WHERE clause present  │
│ │ WHERE import_batch = 123 │   │ Est. rows: ~500          │
│ └──────────────────────────┘   │                         │
├──────────────────────────────────────────────────────────┤
│ AUDIT TRAIL                                               │
│ ◉ 14:32 Nguyễn Văn A — CREATED                          │
│ ◉ 14:35 Nguyễn Văn A — SUBMITTED                        │
│ ○ Waiting for approval...                                │
├──────────────────────────────────────────────────────────┤
│ ADD COMMENT (optional)                                   │
│ [_____________________________________]                  │
│                                                          │
│        [REJECT ✗]              [APPROVE ✓]              │
└──────────────────────────────────────────────────────────┘
```

### 5.3 DaisyUI Components mapping

| Tính năng | Component DaisyUI |
|-----------|------------------|
| Status request | `badge` với màu theo status |
| Danh sách requests | `table` với `overflow-x-auto` |
| Loading state | `skeleton` |
| Approve/Reject confirm | `modal` (dialog native) |
| Realtime notification | `toast` (bottom-end) |
| Notification bell | `indicator` trên `btn` |
| Audit trail | `timeline timeline-vertical` |
| DDL warning | `alert alert-warning` |
| Dashboard stats | `stats` |
| Sidebar | `drawer` |
| SQL analysis result | `diff` (so sánh before/after nếu có) |
| Multi-step form | `steps` |
| Error message | `alert alert-error` |

### 5.4 Toast Notification Pattern (DaisyUI)

```svelte
<!-- src/components/Toast.svelte -->
<script>
    import { notifications } from '../stores/notifications.js';

    // Auto-dismiss sau 4 giây
    const toastTypeClass = {
        success: 'alert-success',
        error: 'alert-error',
        warning: 'alert-warning',
        info: 'alert-info',
    };
</script>

<div class="toast toast-end toast-bottom z-50">
    {#each $notifications as notif (notif.id)}
        <div class="alert {toastTypeClass[notif.type]} shadow-lg">
            <span>{notif.message}</span>
        </div>
    {/each}
</div>
```

### 5.5 Audit Trail với DaisyUI Timeline

```svelte
<!-- Trong ReviewDetail.svelte -->
<ul class="timeline timeline-vertical">
    {#each auditTrail as entry}
        <li>
            {#if !$first(entry)}
                <hr class="{entry.action === 'REJECTED' ? 'bg-error' :
                             entry.action === 'APPROVED' ? 'bg-success' : ''}" />
            {/if}
            <div class="timeline-start text-xs opacity-60">
                {formatTime(entry.timestamp)}
            </div>
            <div class="timeline-middle">
                <!-- Icon theo action -->
            </div>
            <div class="timeline-end timeline-box text-sm">
                <span class="font-semibold">{entry.actor_name}</span>
                — {actionLabel[entry.action]}
                {#if entry.detail}
                    <p class="text-xs opacity-70 mt-1">"{entry.detail}"</p>
                {/if}
            </div>
            <hr />
        </li>
    {/each}
</ul>
```

---

## 6. SQL Safety — Phân tích và kiểm soát SQL

### 6.1 Các mức độ nguy hiểm

| Mức | Pattern | Xử lý |
|-----|---------|-------|
| 🔴 CRITICAL | `DROP TABLE`, `TRUNCATE TABLE`, `DELETE` không có WHERE | Block submit, yêu cầu confirm đặc biệt |
| 🟠 HIGH | `ALTER TABLE`, `UPDATE` không có WHERE, `DELETE` có WHERE nhưng không có LIMIT | Hiển thị warning đỏ cho Checker |
| 🟡 MEDIUM | DDL khác (CREATE, RENAME), số rows ước tính > 10,000 | Hiển thị warning vàng cho Checker |
| 🟢 LOW | DML thông thường có WHERE clause | Thông báo bình thường |

### 6.2 Dangerous Pattern Detection

```rust
// src/executor/sql_analyzer.rs
pub struct DangerousPattern {
    pub severity: Severity,
    pub message: String,
    pub suggestion: String,
}

pub fn detect_dangerous_patterns(sql: &str) -> Vec<DangerousPattern> {
    let mut warnings = vec![];
    let upper = sql.to_uppercase();

    // DELETE không có WHERE
    if upper.contains("DELETE FROM") || upper.contains("DELETE\n") {
        if !upper.contains("WHERE") {
            warnings.push(DangerousPattern {
                severity: Severity::Critical,
                message: "DELETE statement without WHERE clause — will delete ALL rows".into(),
                suggestion: "Add a WHERE clause to limit scope".into(),
            });
        }
    }

    // UPDATE không có WHERE
    if upper.contains("UPDATE ") && upper.contains(" SET ") && !upper.contains("WHERE") {
        warnings.push(DangerousPattern {
            severity: Severity::Critical,
            message: "UPDATE without WHERE clause — will update ALL rows".into(),
            suggestion: "Add WHERE clause to limit affected rows".into(),
        });
    }

    // TRUNCATE
    if upper.contains("TRUNCATE") {
        warnings.push(DangerousPattern {
            severity: Severity::Critical,
            message: "TRUNCATE removes ALL rows and cannot be rolled back in some databases".into(),
            suggestion: "Consider using DELETE with WHERE instead".into(),
        });
    }

    warnings
}
```

---

## 7. API Design — REST Endpoints

### 7.1 Quy tắc thiết kế API

- **Versioning**: `/api/v1/` — chuẩn bị cho backward compat khi thay đổi
- **Pagination**: Tất cả list endpoint trả `{ data: [], total: int, page: int, per_page: int }`
- **Error format**: `{ "error": "message", "code": "ERROR_CODE" }`
- **Success format**: `{ "data": {...} }` hoặc status 204 No Content
- **Auth**: Bearer token trong `Authorization` header
- **Idempotency**: POST `/requests/:id/approve` idempotent — approve 2 lần không lỗi

### 7.2 Request Lifecycle endpoints

```
POST   /api/v1/requests                    → Tạo DRAFT
GET    /api/v1/requests?status=&page=      → List (filter by role tự động)
GET    /api/v1/requests/:id                → Chi tiết + audit trail
PUT    /api/v1/requests/:id                → Sửa (chỉ khi DRAFT)
POST   /api/v1/requests/:id/submit         → DRAFT → PENDING
POST   /api/v1/requests/:id/cancel         → PENDING → CANCELLED
POST   /api/v1/requests/:id/approve        → PENDING → APPROVED (Checker)
POST   /api/v1/requests/:id/reject         → PENDING → REJECTED (Checker, body: {reason})
POST   /api/v1/requests/:id/clone          → Tạo DRAFT mới từ request cũ
GET    /api/v1/requests/:id/audit          → Audit trail của request
```

### 7.3 Dashboard endpoint

```
GET    /api/v1/dashboard                   → Stats theo role của user đang login
```

**Response format cho `GET /api/v1/dashboard`:**

```json
{
  "data": {
    "role": "MAKER",
    "stats": {
      "draft": 2,
      "pending": 3,
      "approved": 1,
      "executed": 45,
      "rejected": 5,
      "failed": 1
    },
    "recent_requests": [
      {
        "id": "uuid",
        "description": "Cập nhật trạng thái tài khoản",
        "status": "PENDING",
        "database_name": "ORACLE_PROD",
        "submitted_at": "2026-07-11T07:35:00Z"
      }
    ],
    "avg_approval_time_minutes": 42
  }
}
```

> **Lưu ý**: Response thay đổi theo role. Checker thấy `pending_count`, `oldest_pending_age_hours`. Admin thấy tổng hợp toàn hệ thống.

### 7.3 Response mẫu cho Request detail

```json
{
  "data": {
    "id": "uuid",
    "description": "Cập nhật trạng thái tài khoản",
    "status": "PENDING",
    "database": {
      "id": "uuid",
      "name": "ORACLE_PROD_ACCOUNTS",
      "type": "oracle"
    },
    "analysis": {
      "contains_ddl": false,
      "ddl_keywords": [],
      "statement_count": 1,
      "is_dml_only": true,
      "dangerous_patterns": []
    },
    "created_by": { "id": "uuid", "name": "Nguyễn Văn A" },
    "approved_by": null,
    "created_at": "2026-07-11T07:32:00Z",
    "submitted_at": "2026-07-11T07:35:00Z",
    "executed_at": null,
    "execution_result": null,
    "audit_trail": [
      {
        "action": "CREATED",
        "actor": { "id": "uuid", "name": "Nguyễn Văn A" },
        "timestamp": "2026-07-11T07:32:00Z",
        "detail": null
      },
      {
        "action": "SUBMITTED",
        "actor": { "id": "uuid", "name": "Nguyễn Văn A" },
        "timestamp": "2026-07-11T07:35:00Z",
        "detail": null
      }
    ]
  }
}
```

> **Lưu ý bảo mật**: `sql_content` chỉ trả về trong detail response — không trong list response.
> Điều này tránh log sensitive SQL khi log HTTP response body.

---

## 8. Database Migrations — Schema đầy đủ

Xem §14 trong `implementation-references.md` để biết chi tiết SQL.

Các điểm quan trọng về schema:

1. **`four_eyes_check`**: `CHECK (approved_by IS NULL OR approved_by != created_by)` — enforce tại DB
2. **Audit log immutability**: App database user chỉ có `INSERT` và `SELECT` trên `audit_log`, không có `UPDATE`/`DELETE`
3. **Encrypted credentials**: Column `encrypted_credentials TEXT` — không bao giờ lưu plaintext
4. **UUID primary keys**: Dùng `gen_random_uuid()` — không sequential, khó đoán
5. **Soft delete cho users**: `is_active = FALSE` thay vì xóa — giữ referential integrity với audit log
6. **Timezone**: Tất cả timestamp dùng `TIMESTAMPTZ` (UTC) — tránh bug DST

---

## 9. Deployment & Operations

### 9.1 Checklist trước khi go-live

```
□ Tất cả secrets trong environment variables, không hardcode
□ .env không được commit (kiểm tra .gitignore)
□ TLS certificate cho nginx
□ PostgreSQL backup tự động hàng ngày
□ Health check endpoint hoạt động: GET /health → 200
□ Keycloak/SSO client được cấu hình đúng redirect URI
□ Database user cho app chỉ có quyền cần thiết (không phải superuser)
□ Audit log table: revoke DELETE privilege từ app user
□ Test OIDC login flow end-to-end
□ Test 4-eyes: Maker không approve được request của mình
□ Test timeout: SQL chạy > timeout_secs bị cancel đúng cách
□ Test rollback: multi-statement SQL, statement 2 lỗi → toàn bộ rollback
```

### 9.2 Monitoring tối thiểu

- Log aggregation: ship logs từ container đến ELK / Loki
- Alert khi: error rate > 5%, pending requests > 20 (SLA breach), disk > 80%
- Weekly: chạy `cargo audit` kiểm tra CVE trong dependencies

---

## 10. Nguồn nghiên cứu

- SOX compliance cho DCM: liquibase.com/resources/guides/sox-compliance-for-database-change-management-best-practices
- DORA compliance: red-gate.com/blog/why-digital-operational-resilience-act-dora-compliance-requires-auditable-database-change-management
- Maker-Checker pattern: xtrm.com/blog/posts/maker-checker-process
- 4-eyes principle SAP implementation: community.sap.com (2026)
- Internal tool UX: dronahq.com/internal-tools-ux
- Notification UX: eleken.co/blog-posts/notification-ux
- DaisyUI components: daisyui.com/components
- Context7 + Web research (truy xuất 2026-07-11)
