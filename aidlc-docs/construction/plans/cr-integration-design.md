# Change Request (CR) Integration — Thiết kế tính năng

## Tổng quan

Tích hợp ManageEngine ServiceDesk Plus Change Request vào hệ thống SECS, cho phép Maker gắn SQL request với một CR để traceability trong quy trình golive.

## Quyết định thiết kế

| Aspect | Decision |
|--------|----------|
| **Quan hệ** | Một CR chứa nhiều SQL requests (1-N) |
| **Bắt buộc?** | Optional — Maker có thể tạo request không gắn CR |
| **Nguồn gốc** | Maker nhập full URL từ ManageEngine ServiceDesk Plus |
| **Time enforcement** | Lấy `scheduled_start_time` / `scheduled_end_time` từ API → chặn thực thi ngoài maintenance window |
| **Quản lý CR** | CR được quản lý trên ManageEngine, SECS chỉ lưu metadata cache |

---

## Luồng hoạt động

```
1. Maker tạo/submit SQL request → nhập URL CR (optional)
2. Backend parse URL → extract change_id
3. Backend gọi ManageEngine API: GET /api/v3/changes/{change_id}
4. Lưu metadata: title, scheduled_start_time, scheduled_end_time
5. Khi Checker approve → hệ thống check:
   - Nếu CR có time window → chỉ execute khi NOW nằm trong window
   - Nếu ngoài window → block execution với lý do rõ ràng
```

---

## ManageEngine ServiceDesk Plus API Reference

### Authentication

```
Header: authtoken: {API_TOKEN}
Accept: application/vnd.manageengine.sdp.v3+json
```

### Get Change Details

```
GET {base_url}/api/v3/changes/{change_id}
```

**Response fields quan trọng:**

| Field | Type | Mô tả |
|-------|------|--------|
| `id` | long | Unique identifier |
| `title` | string | Tiêu đề CR |
| `status.name` | string | Status hiện tại (Requested, Approved, etc.) |
| `scheduled_start_time.value` | long (ms) | Thời gian bắt đầu maintenance window |
| `scheduled_end_time.value` | long (ms) | Thời gian kết thúc maintenance window |
| `approval_status.name` | string | Trạng thái approval |
| `change_type.name` | string | Loại change (Major, Minor, etc.) |
| `stage.name` | string | Stage hiện tại (Submission, Planning, Implementation, etc.) |

### Sample Response (rút gọn)

```json
{
  "response_status": { "status_code": 2000, "status": "success" },
  "change": {
    "id": "1",
    "title": "Operating System migration",
    "status": { "name": "Requested", "id": "3" },
    "approval_status": { "name": "Pending Approval", "id": "1" },
    "scheduled_start_time": {
      "display_value": "Jul 24, 2019 04:32 PM",
      "value": "1563966120000"
    },
    "scheduled_end_time": {
      "display_value": "Jul 25, 2019 04:32 PM",
      "value": "1564052520000"
    }
  }
}
```

---

## Database Schema

### Migration 009: Change Request Integration

```sql
-- Bảng change_requests — lưu metadata CR từ ManageEngine
CREATE TABLE change_requests (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cr_number               VARCHAR(100) NOT NULL,        -- Change ID từ ManageEngine (e.g. "17")
    cr_url                  TEXT NOT NULL,                 -- Full URL user nhập
    title                   TEXT,                          -- Title lấy từ ManageEngine API
    status                  TEXT,                          -- Status từ ManageEngine
    scheduled_start_time    TIMESTAMPTZ,                  -- Maintenance window start
    scheduled_end_time      TIMESTAMPTZ,                  -- Maintenance window end
    synced_at               TIMESTAMPTZ,                  -- Lần cuối sync từ ManageEngine
    created_by              UUID NOT NULL REFERENCES users(id),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_cr_number UNIQUE (cr_number)
);

-- Liên kết với sql_requests (optional, N-1)
ALTER TABLE sql_requests
ADD COLUMN cr_id UUID REFERENCES change_requests(id) ON DELETE SET NULL;

-- Indexes
CREATE INDEX idx_sql_requests_cr_id ON sql_requests(cr_id);
CREATE INDEX idx_change_requests_cr_number ON change_requests(cr_number);
```

---

## Cấu hình (thêm vào .env)

```env
# ManageEngine ServiceDesk Plus integration
MANAGEENGINE_BASE_URL=https://servicedesk.company.com
MANAGEENGINE_AUTH_TOKEN=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
MANAGEENGINE_ENABLED=true
```

---

## API Endpoints

### Endpoints mới

| Method | Path | Role | Mô tả |
|--------|------|------|--------|
| `POST` | `/api/v1/change-requests` | Maker | Tạo CR từ URL, gọi ManageEngine fetch metadata |
| `GET` | `/api/v1/change-requests` | Maker/Checker | List CRs (filter by status, pagination) |
| `GET` | `/api/v1/change-requests/:id` | Maker/Checker | Chi tiết CR + danh sách SQL requests gắn vào |
| `POST` | `/api/v1/change-requests/:id/sync` | Maker | Re-sync metadata từ ManageEngine |
| `DELETE` | `/api/v1/change-requests/:id` | Admin | Xóa CR (unlink các requests) |

### Endpoints sửa đổi

| Endpoint | Thay đổi |
|----------|----------|
| `POST /api/v1/requests` | Thêm field `cr_url: Option<String>` trong body |
| `GET /api/v1/requests/:id` | Response thêm `cr_id`, `cr_number`, `cr_title` |
| Approve flow | Thêm check maintenance window trước khi execute |

### Request/Response DTOs

**POST /api/v1/change-requests**

```json
// Request
{ "cr_url": "https://servicedesk.company.com/app/itdesk/ui/changes/12345/details" }

// Response
{
  "id": "uuid",
  "cr_number": "12345",
  "cr_url": "https://...",
  "title": "Database migration for Q3 release",
  "status": "Approved",
  "scheduled_start_time": "2026-07-25T22:00:00Z",
  "scheduled_end_time": "2026-07-26T04:00:00Z",
  "synced_at": "2026-07-21T10:00:00Z",
  "created_at": "2026-07-21T10:00:00Z"
}
```

**CreateRequestBody (sửa đổi)**

```json
{
  "description": "Update customer table",
  "sql_content": "UPDATE customers SET ...",
  "database_id": "uuid",
  "submit": true,
  "cr_url": "https://servicedesk.company.com/app/itdesk/ui/changes/12345/details"  // optional
}
```

---

## Backend Implementation

### Modules cần thêm mới

| File | Mô tả |
|------|--------|
| `src/integrations/mod.rs` | Module chung cho external integrations |
| `src/integrations/manageengine.rs` | HTTP client gọi ManageEngine API |
| `src/models/change_request.rs` | Struct `ChangeRequest` |
| `src/db/change_requests.rs` | CRUD cho bảng `change_requests` |
| `src/api/change_requests.rs` | API handlers cho CR endpoints |

### Modules cần sửa

| File | Thay đổi |
|------|----------|
| `src/models/request.rs` | Thêm `cr_id: Option<Uuid>` vào `SqlRequest` |
| `src/models/mod.rs` | Thêm `pub mod change_request;` |
| `src/db/mod.rs` | Thêm `pub mod change_requests;` |
| `src/api/mod.rs` | Thêm routes cho change-requests |
| `src/api/requests.rs` | `CreateRequestBody` thêm `cr_url`, logic link CR |
| `src/api/approvals.rs` | Check maintenance window trước execute |
| `src/config.rs` | Thêm `ManageEngineConfig` |
| `src/state.rs` | Thêm `reqwest::Client` cho HTTP calls |
| `src/error.rs` | Thêm `CrWindowViolation`, `ManageEngineError` variants |

### URL Parser

```rust
use regex::Regex;

/// Extract change_id từ ManageEngine URL.
/// Hỗ trợ patterns:
///   - .../changes/{id}/...
///   - .../changes/{id}
///   - .../WorkOrder.do?... CHANGEID={id}
fn parse_cr_url(url: &str) -> Result<String, AppError> {
    // Pattern 1: REST-style URL
    let re = Regex::new(r"/changes/(\d+)").unwrap();
    if let Some(cap) = re.captures(url) {
        return Ok(cap[1].to_string());
    }

    // Pattern 2: Legacy URL with query params
    let re2 = Regex::new(r"CHANGEID=(\d+)").unwrap();
    if let Some(cap) = re2.captures(url) {
        return Ok(cap[1].to_string());
    }

    Err(AppError::ValidationError(
        "Cannot parse Change ID from URL. Expected format: .../changes/{id}/...".into()
    ))
}
```

### ManageEngine Client

```rust
pub struct ManageEngineClient {
    http: reqwest::Client,
    base_url: String,
    auth_token: String,
}

impl ManageEngineClient {
    /// Fetch change details từ ManageEngine API
    pub async fn get_change(&self, change_id: &str) -> Result<ChangeDetail, ManageEngineError> {
        let url = format!("{}/api/v3/changes/{}", self.base_url, change_id);

        let resp = self.http
            .get(&url)
            .header("Accept", "application/vnd.manageengine.sdp.v3+json")
            .header("authtoken", &self.auth_token)
            .send()
            .await?;

        let body: ManageEngineResponse = resp.json().await?;

        if body.response_status.status_code != 2000 {
            return Err(ManageEngineError::ApiError(body.response_status.status));
        }

        Ok(body.change.into())
    }
}

#[derive(Debug)]
pub struct ChangeDetail {
    pub id: String,
    pub title: String,
    pub status: Option<String>,
    pub scheduled_start_time: Option<DateTime<Utc>>,
    pub scheduled_end_time: Option<DateTime<Utc>>,
}
```

### Maintenance Window Enforcement

```rust
/// Check tại thời điểm execute — trong approve handler (approvals.rs)
async fn check_maintenance_window(
    pool: &PgPool,
    request: &SqlRequest,
) -> Result<(), AppError> {
    let cr_id = match request.cr_id {
        Some(id) => id,
        None => return Ok(()), // Không có CR → không enforce
    };

    let cr = db::change_requests::get_by_id(pool, cr_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Chỉ enforce nếu CR có time window
    if let (Some(start), Some(end)) = (cr.scheduled_start_time, cr.scheduled_end_time) {
        let now = Utc::now();
        if now < start || now > end {
            return Err(AppError::CrWindowViolation {
                cr_number: cr.cr_number,
                window_start: start,
                window_end: end,
            });
        }
    }

    Ok(())
}
```

---

## Frontend Changes

### Components mới

| Component | Mô tả |
|-----------|--------|
| `pages/maker/ChangeRequests.svelte` | Trang quản lý CR — list, xem tiến độ |
| `pages/maker/CrDetail.svelte` | Chi tiết CR + danh sách requests |
| `components/CrBadge.svelte` | Badge hiển thị CR number + link |
| `components/CrInput.svelte` | Input URL CR với preview metadata |
| `components/MaintenanceWindow.svelte` | Hiển thị time window cho Checker |

### Sửa đổi components hiện có

| Component | Thay đổi |
|-----------|----------|
| `pages/maker/NewRequest.svelte` | Thêm CrInput (optional) |
| `pages/maker/MyRequests.svelte` | Hiển thị CrBadge nếu có |
| `pages/maker/RequestDetail.svelte` | Hiển thị thông tin CR |
| `pages/checker/ReviewDetail.svelte` | Hiển thị CR info + maintenance window warning |
| `components/Sidebar.svelte` | Thêm menu "Change Requests" |

### UX Flow

1. **Tạo request**: Maker nhập URL ManageEngine vào ô "Change Request (optional)"
   - Khi blur/submit → backend validate URL + fetch title
   - Hiển thị preview: "CR #12345 — Database migration for Q3" ✅
   - Nếu URL invalid → hiển thị error inline

2. **Review (Checker)**: Hiển thị thông tin CR bao gồm:
   - CR number + link clickable
   - Maintenance window (start — end)
   - Warning nếu hiện tại ngoài window: "⚠️ Ngoài maintenance window. Thực thi sẽ bị block."

3. **CR Dashboard**: Trang riêng cho Maker xem:
   - Danh sách CR đã tạo
   - Mỗi CR hiển thị: số request gắn vào, trạng thái tổng hợp
   - Filter theo status, date range

---

## Error Cases

| Error | HTTP Code | Khi nào |
|-------|-----------|---------|
| `CrWindowViolation` | 409 Conflict | Execute SQL ngoài maintenance window |
| `ManageEngineUnavailable` | 502 Bad Gateway | Không kết nối được ManageEngine API |
| `InvalidCrUrl` | 400 Bad Request | URL không parse được change_id |
| `CrNotFound` | 404 Not Found | change_id không tồn tại trên ManageEngine |

---

## Audit Log

Các action mới cần ghi vào audit:

| Action | Khi nào |
|--------|---------|
| `CR_LINKED` | SQL request được gắn vào CR |
| `CR_UNLINKED` | SQL request bị gỡ khỏi CR |
| `CR_SYNCED` | Metadata CR được sync từ ManageEngine |
| `CR_WINDOW_BLOCKED` | Execution bị block do ngoài maintenance window |

---

## Câu hỏi chưa quyết định (cần confirm)

1. **Khi ngoài maintenance window**: Block hoàn toàn (Checker phải đợi), hay cho approve nhưng queue lại đợi window đến tự execute?

2. **URL pattern thực tế**: URL ManageEngine của công ty dạng chính xác nào? (để parse chính xác change_id)

3. **Sync frequency**: Khi CR bị cancel trên ManageEngine, SECS tự sync định kỳ hay chỉ sync khi user bấm?

--> Chỉ quản lý thông tin CR, việc tích hợp với hệ thống ManageEngine được phân tích sau

---

## Dependencies mới (Cargo.toml)

```toml
# Đã có sẵn trong project
# reqwest — HTTP client (dùng cho ManageEngine API)
# regex — parse URL

# Không cần thêm dependency mới nếu đã có reqwest
```

---

## Thứ tự triển khai đề xuất

1. **Migration** — tạo bảng `change_requests`, thêm column `cr_id`
2. **Config** — thêm ManageEngine config vào `.env` và `config.rs`
3. **Integration** — `integrations/manageengine.rs` (HTTP client)
4. **Models + DB** — `ChangeRequest` struct, CRUD queries
5. **API** — handlers cho `/change-requests`
6. **Sửa requests flow** — link CR khi tạo request
7. **Sửa approvals flow** — check maintenance window
8. **Frontend** — UI components + pages
9. **Testing** — unit tests + integration tests
