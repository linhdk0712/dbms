# API Tester (Postman-like) với RBAC — Nghiên cứu Thiết kế

**Tác giả**: Kiro AI  
**Ngày**: 2026-09-19  
**Trạng thái**: Draft — Nghiên cứu ban đầu

---

## 1. Tổng quan tính năng

### 1.1 Vấn đề cần giải quyết

Trong môi trường doanh nghiệp tài chính, nhân viên vận hành thường cần test các API nội bộ hoặc bên ngoài như một phần công việc. Hiện tại họ dùng Postman/curl bên ngoài hệ thống — không có audit trail, không kiểm soát được ai đang gọi API nào, không có approval workflow.

### 1.2 Mục tiêu

Tích hợp một công cụ **API Tester tương tự Postman** trực tiếp vào SECS với:
- **RBAC**: Phân quyền ai được test endpoint nào, với method nào
- **Audit trail**: Ghi lại toàn bộ request/response (có thể bao gồm cả approval workflow cho môi trường PROD)
- **Collection management**: Lưu và chia sẻ tập request theo nhóm
- **Environment variables**: Quản lý biến môi trường (dev/staging/prod) với phân quyền riêng

### 1.3 Phạm vi phiên bản đầu

| Tính năng | Có/Không |
|-----------|----------|
| HTTP Methods (GET/POST/PUT/PATCH/DELETE) | ✅ |
| Custom Headers | ✅ |
| Request Body (JSON, Form, Raw) | ✅ |
| Response viewer (status, headers, body) | ✅ |
| Collection lưu request | ✅ |
| Environment Variables | ✅ |
| RBAC theo endpoint pattern | ✅ |
| Approval workflow (Maker/Checker) | ✅ (cho PROD env) |
| Audit log | ✅ |
| WebSocket testing | ❌ (future) |
| Scripting (pre/post request) | ❌ (future) |
| Certificate auth (mTLS) | ❌ (future) |

---

## 2. Kiến trúc hệ thống

### 2.1 Luồng tổng thể

```
[Frontend — API Tester UI]
        │
        │ POST /api/tester/execute
        │ { method, url, headers, body, env_id }
        ▼
[Backend — Axum Handler]
        │
        ├── 1. Xác thực JWT (RequireAuth)
        ├── 2. Kiểm tra RBAC (user có quyền gọi URL pattern này không?)
        ├── 3. Ghi audit log (BEFORE execution)
        ├── 4. Resolve environment variables
        ├── 5. reqwest Client → gọi API thực
        └── 6. Trả response + ghi audit log (AFTER)
```

### 2.2 Sơ đồ component

```
backend/src/
├── api/
│   ├── api_tester.rs          # Handlers: execute, collections CRUD, environments CRUD
│   └── mod.rs                 # Thêm routes
├── db/
│   ├── api_tester.rs          # Queries: collections, environments, permissions, history
│   └── mod.rs
├── models/
│   ├── api_tester.rs          # ApiRequest, ApiResponse, Collection, Environment, TesterPermission
│   └── mod.rs
└── executor/
    └── http_executor.rs       # reqwest wrapper — thực thi HTTP request

frontend/src/
├── api/
│   └── tester.js              # API client cho /api/tester/*
├── pages/
│   └── tester/
│       ├── ApiTester.svelte   # Trang chính (layout 3 cột: sidebar, request, response)
│       ├── RequestPanel.svelte # Form nhập method/URL/headers/body
│       ├── ResponsePanel.svelte # Hiển thị response
│       ├── CollectionSidebar.svelte # Cây collection
│       └── EnvironmentModal.svelte  # Quản lý environments
└── components/
    └── KeyValueEditor.svelte  # Reusable key-value editor cho headers/params
```

---

## 3. Thiết kế Database

### 3.1 Tables mới

```sql
-- Bảng collection (nhóm request)
CREATE TABLE api_collections (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    description TEXT,
    owner_id    UUID REFERENCES users(id),
    is_shared   BOOLEAN DEFAULT false,  -- shared với team
    created_at  TIMESTAMPTZ DEFAULT now(),
    updated_at  TIMESTAMPTZ DEFAULT now()
);

-- Bảng lưu từng request trong collection
CREATE TABLE api_saved_requests (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id UUID REFERENCES api_collections(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    method        TEXT NOT NULL,        -- GET, POST, PUT, PATCH, DELETE
    url           TEXT NOT NULL,
    headers       JSONB DEFAULT '[]',  -- [{key, value, enabled}]
    query_params  JSONB DEFAULT '[]',  -- [{key, value, enabled}]
    body_type     TEXT,                -- json, form, raw, none
    body_content  TEXT,
    created_by    UUID REFERENCES users(id),
    created_at    TIMESTAMPTZ DEFAULT now()
);

-- Bảng environment (dev/staging/prod)
CREATE TABLE api_environments (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,         -- "Production", "Staging"
    owner_id    UUID REFERENCES users(id),
    is_shared   BOOLEAN DEFAULT false,
    sensitivity TEXT DEFAULT 'low',    -- low, medium, high (ảnh hưởng require approval)
    created_at  TIMESTAMPTZ DEFAULT now()
);

-- Biến trong environment
CREATE TABLE api_env_variables (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    environment_id UUID REFERENCES api_environments(id) ON DELETE CASCADE,
    key            TEXT NOT NULL,
    value          TEXT NOT NULL,      -- giá trị thực (encrypt nếu is_secret=true)
    is_secret      BOOLEAN DEFAULT false,
    UNIQUE(environment_id, key)
);

-- RBAC: quyền thực thi API tester theo user + pattern
CREATE TABLE api_tester_permissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID REFERENCES users(id),
    url_pattern     TEXT NOT NULL,     -- glob pattern vd: "https://api.internal/*"
    allowed_methods TEXT[] NOT NULL,   -- ["GET", "POST"]
    environment_id  UUID REFERENCES api_environments(id), -- NULL = all envs
    require_approval BOOLEAN DEFAULT false, -- true = cần Checker approve trước
    granted_by      UUID REFERENCES users(id),
    granted_at      TIMESTAMPTZ DEFAULT now(),
    expires_at      TIMESTAMPTZ         -- optional expiry
);

-- Lịch sử thực thi (audit log)
CREATE TABLE api_execution_history (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    executor_id       UUID REFERENCES users(id),
    method            TEXT NOT NULL,
    url               TEXT NOT NULL,   -- URL sau khi resolve variables
    request_headers   JSONB,
    request_body      TEXT,
    environment_id    UUID REFERENCES api_environments(id),
    response_status   INT,
    response_headers  JSONB,
    response_body     TEXT,
    duration_ms       INT,
    executed_at       TIMESTAMPTZ DEFAULT now(),
    approval_id       UUID,            -- nếu có approval workflow
    collection_id     UUID REFERENCES api_collections(id),
    saved_request_id  UUID REFERENCES api_saved_requests(id)
);
```

---

## 4. Backend Implementation

### 4.1 HTTP Executor & Phòng vệ SSRF tầng Socket

#### 4.1.1 Rủi ro SSRF & DNS Rebinding
Nếu chỉ kiểm tra URL ở dạng chuỗi (string pattern matching) trước khi gửi:
- **DNS Rebinding**: Tên miền của kẻ tấn công (ví dụ `rebind.evil.com`) có thể trả về IP public khi kiểm tra RBAC, nhưng trả về IP nội bộ (`127.0.0.1` hoặc `169.254.169.254` AWS metadata) khi `reqwest` thực sự kết nối.
- **Open Redirect**: Endpoint được cấp phép trả về HTTP 302 chuyển hướng tới dịch vụ nội bộ chưa được cấp phép.
- **Alternate IP Encodings**: Dạng hex/decimal/octal (ví dụ `0177.0.0.1`, `2130706433`).

#### 4.1.2 Giải pháp: `SafeDnsResolver` tại tầng Socket
Triển khai trait `reqwest::dns::Resolve` để chặn phân giải ra IP cấm **ngay trước khi bắt tay TCP**:

```rust
use reqwest::{
    Client, Method, redirect::Policy,
    dns::{Resolve, Resolving, Addrs},
    header::{HeaderMap, HeaderName, HeaderValue},
};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// Danh sách dải IP nội bộ/nhạy cảm bị cấm theo mặc định
pub fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback()                           // 127.0.0.0/8
            || ipv4.is_unspecified()                    // 0.0.0.0
            || ipv4.is_link_local()                     // 169.254.0.0/16 (gồm Cloud Metadata)
            || ipv4.is_private()                        // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
            || ipv4.is_broadcast()                      // 255.255.255.255
            || ipv4.octets()[0] == 100 && (ipv4.octets()[1] & 0xC0) == 64 // 100.64.0.0/10 CGNAT
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback()                          // ::1
            || ipv6.is_unspecified()                   // ::
            || (ipv6.segments()[0] & 0xfe00) == 0xfc00  // fc00::/7 (Unique Local)
            || (ipv6.segments()[0] & 0xffc0) == 0xfe80  // fe80::/10 (Link-Local)
        }
    }
}

/// Custom DNS Resolver chặn IP nhạy cảm trước khi thiết lập kết nối
pub struct SafeDnsResolver {
    pub allow_internal_ips: bool, // chỉ bật khi RBAC cho phép truy cập explicit
}

impl Resolve for SafeDnsResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> Resolving {
        let allow_internal = self.allow_internal_ips;
        Box::pin(async move {
            let host = name.as_str();
            // Phân giải DNS qua hệ thống
            let addrs = tokio::net::lookup_host((host, 80)).await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

            let mut safe_addrs: Vec<SocketAddr> = Vec::new();
            for addr in addrs {
                if !allow_internal && is_forbidden_ip(addr.ip()) {
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("SSRF Blocked: IP {} không được phép truy cập", addr.ip()),
                    )) as Box<dyn std::error::Error + Send + Sync>);
                }
                safe_addrs.push(addr);
            }

            if safe_addrs.is_empty() {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Không thể phân giải địa chỉ hợp lệ",
                )) as Box<dyn std::error::Error + Send + Sync>);
            }

            let addrs: Addrs = Box::new(safe_addrs.into_iter());
            Ok(addrs)
        })
    }
}
```

#### 4.1.3 HTTP Executor hoàn chỉnh

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub timeout_secs: u64,
    pub accept_invalid_certs: bool,  // chỉ cho dev/staging env
    pub allow_internal_ips: bool,    // chỉ bật nếu user có permission explicit vào internal network
}

#[derive(Debug, Serialize)]
pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub duration_ms: u64,
    pub size_bytes: usize,
}

pub async fn execute_http_request(req: HttpRequest) -> Result<HttpResponse, AppError> {
    // 1. Kiểm tra scheme chỉ cho phép http / https
    let parsed_url = url::Url::parse(&req.url)
        .map_err(|_| AppError::BadRequest("URL không hợp lệ".into()))?;
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err(AppError::BadRequest("Chỉ hỗ trợ giao thức HTTP và HTTPS".into()));
    }

    // 2. Cấu hình DNS resolver an toàn ở tầng socket
    let resolver = Arc::new(SafeDnsResolver {
        allow_internal_ips: req.allow_internal_ips,
    });

    // 3. Cấu hình Redirect Policy (Tối đa 5 hops, kiểm tra scheme mỗi hop)
    let redirect_policy = Policy::custom(|attempt| {
        if attempt.previous().len() >= 5 {
            attempt.error("Quá nhiều lượt chuyển hướng (tối đa 5)");
        } else if attempt.url().scheme() != "http" && attempt.url().scheme() != "https" {
            attempt.error("Không được chuyển hướng sang giao thức không an toàn");
        } else {
            attempt.follow()
        }
    });

    let client = Client::builder()
        .dns_resolver(resolver)
        .redirect(redirect_policy)
        .timeout(Duration::from_secs(req.timeout_secs))
        .tls_danger_accept_invalid_certs(req.accept_invalid_certs)
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let method = Method::from_bytes(req.method.as_bytes())
        .map_err(|_| AppError::BadRequest("Invalid HTTP method".into()))?;

    let mut request_builder = client.request(method, parsed_url);

    // Build headers
    let mut headers = HeaderMap::new();
    for (k, v) in &req.headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(v),
        ) {
            headers.insert(name, value);
        }
    }
    request_builder = request_builder.headers(headers);

    if let Some(body) = req.body {
        request_builder = request_builder.body(body);
    }

    let start = Instant::now();
    let response = request_builder.send().await
        .map_err(|e| AppError::ExecutionFailed(format!("Gửi request thất bại: {}", e)))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let status = response.status();
    let resp_headers: Vec<(String, String)> = response.headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Giới hạn đọc tối đa 1MB response để tránh tràn RAM / DoS
    let body = response.text().await
        .unwrap_or_else(|_| String::from("[binary response]"));

    let size_bytes = body.len();

    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers: resp_headers,
        body,
        duration_ms,
        size_bytes,
    })
}

### 4.2 RBAC Checker

```rust
// db/api_tester.rs
pub async fn check_tester_permission(
    pool: &PgPool,
    user_id: Uuid,
    method: &str,
    url: &str,
    environment_id: Option<Uuid>,
) -> Result<Option<ApiTesterPermission>, AppError> {
    // Lấy tất cả permissions của user, check URL pattern matching
    // Dùng glob matching: "https://api.internal/*" khớp với "https://api.internal/users/123"
    let permissions = sqlx::query_as!(
        ApiTesterPermission,
        r#"SELECT * FROM api_tester_permissions
           WHERE user_id = $1
             AND $2 = ANY(allowed_methods)
             AND (expires_at IS NULL OR expires_at > now())
             AND (environment_id IS NULL OR environment_id = $3)"#,
        user_id,
        method.to_uppercase(),
        environment_id
    )
    .fetch_all(pool)
    .await?;

    // Pattern matching (glob)
    for perm in permissions {
        if glob_match(&perm.url_pattern, url) {
            return Ok(Some(perm));
        }
    }
    Ok(None)
}

fn glob_match(pattern: &str, url: &str) -> bool {
    // Dùng crate `glob` hoặc implement đơn giản với wildcard
    let pattern_regex = pattern
        .replace('.', r"\.")
        .replace('*', ".*")
        .replace('?', ".");
    regex::Regex::new(&format!("^{}$", pattern_regex))
        .map(|r| r.is_match(url))
        .unwrap_or(false)
}
```

### 4.3 Chuẩn hóa Luồng Resolve Biến & API Handler tại Backend

#### 4.3.1 Kiến trúc Chuẩn hóa Resolve Biến
Nhằm loại bỏ hoàn toàn nguy cơ lộ secret ra frontend và đồng nhất dữ liệu kiểm toán, luồng biến môi trường được quy chuẩn như sau:
1. **Frontend gửi Raw Template**: Mọi trường URL, Header, Query Param, Request Body đều giữ nguyên cú pháp `{{variable_name}}`.
2. **Backend Dual-Resolution**:
   - `ResolvedPayload`: Chứa dữ liệu thật (đã giải mã secret) → Chỉ chuyển cho `reqwest` thực thi HTTP.
   - `SanitizedPayload`: Các biến có `is_secret = true` hoặc header nhạy cảm (`Authorization`, `X-Api-Key`) sẽ bị thay thế bằng `[REDACTED]` → Dùng để ghi vào `api_execution_history` và màn hình Approval của Checker.
3. **Biến Động Hệ Thống (Built-in Dynamic Variables)**: Hỗ trợ `{{$timestamp}}`, `{{$guid}}`, `{{$randomInt}}` sinh trực tiếp tại backend để đảm bảo tính duy nhất và nhất quán giữa thực thi và audit log.

```rust
// executor/variable_resolver.rs
use std::collections::HashMap;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use regex::Regex;

#[derive(Clone)]
pub struct EnvVarEntry {
    pub value: String,
    pub is_secret: bool,
}

pub struct ResolvedBundle {
    pub resolved_url: String,
    pub sanitized_url: String,
    pub resolved_headers: Vec<(String, String)>,
    pub sanitized_headers: Vec<(String, String)>,
    pub resolved_body: Option<String>,
    pub sanitized_body: Option<String>,
}

pub async fn resolve_env_vars(
    pool: &PgPool,
    payload: &ExecuteApiRequest,
    environment_id: Option<Uuid>,
) -> Result<ResolvedBundle, AppError> {
    let mut var_map: HashMap<String, EnvVarEntry> = HashMap::new();

    // 1. Tải biến từ DB nếu có environment_id
    if let Some(env_id) = environment_id {
        let rows = sqlx::query!(
            "SELECT key, value, is_secret FROM api_env_variables WHERE environment_id = $1",
            env_id
        )
        .fetch_all(pool)
        .await?;

        for row in rows {
            var_map.insert(row.key, EnvVarEntry {
                value: row.value, // Đã decrypt nếu trước đó lưu encrypted
                is_secret: row.is_secret.unwrap_or(false),
            });
        }
    }

    // 2. Định nghĩa hàm thay thế template
    let re = Regex::new(r"\{\{([a-zA-Z0-9_$.]+)\}\}").unwrap();

    // Sinh giá trị động cho các biến hệ thống
    let current_ts = Utc::now().timestamp().to_string();
    let current_guid = Uuid::new_v4().to_string();

    let replace_fn = |text: &str, mask_secret: bool| -> String {
        re.replace_all(text, |caps: &regex::Captures| {
            let key = &caps[1];
            match key {
                "$timestamp" => current_ts.clone(),
                "$guid" => current_guid.clone(),
                _ => {
                    if let Some(entry) = var_map.get(key) {
                        if entry.is_secret && mask_secret {
                            "[REDACTED]".to_string()
                        } else {
                            entry.value.clone()
                        }
                    } else {
                        caps[0].to_string() // Giữ nguyên nếu không tìm thấy
                    }
                }
            }
        }).to_string()
    };

    // 3. Resolve URL & Query Params
    let mut full_url = payload.url.clone();
    if !payload.query_params.is_empty() {
        let qs: Vec<String> = payload.query_params.iter()
            .filter(|p| p.enabled)
            .map(|p| format!("{}={}", urlencoding::encode(&p.key), urlencoding::encode(&p.value)))
            .collect();
        if !qs.is_empty() {
            let separator = if full_url.contains('?') { '&' } else { '?' };
            full_url = format!("{}{}{}", full_url, separator, qs.join("&"));
        }
    }

    let resolved_url = replace_fn(&full_url, false);
    let sanitized_url = replace_fn(&full_url, true);

    // 4. Resolve Headers
    let mut resolved_headers = Vec::new();
    let mut sanitized_headers = Vec::new();

    for h in &payload.headers {
        if !h.enabled { continue; }
        let key = replace_fn(&h.key, false);
        let val_real = replace_fn(&h.value, false);
        let val_sanitized = if h.key.to_lowercase() == "authorization" || h.key.to_lowercase() == "x-api-key" {
            // Mask các header nhạy cảm mặc định trong audit log
            "[REDACTED]".to_string()
        } else {
            replace_fn(&h.value, true)
        };

        resolved_headers.push((key.clone(), val_real));
        sanitized_headers.push((key, val_sanitized));
    }

    // 5. Resolve Body
    let resolved_body = payload.body_content.as_ref().map(|b| replace_fn(b, false));
    let sanitized_body = payload.body_content.as_ref().map(|b| replace_fn(b, true));

    Ok(ResolvedBundle {
        resolved_url,
        sanitized_url,
        resolved_headers,
        sanitized_headers,
        resolved_body,
        sanitized_body,
    })
}
```

#### 4.3.2 Handler Thực thi (`api_tester.rs`)

```rust
// api/api_tester.rs
#[derive(Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
pub struct ExecuteApiRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub query_params: Vec<KeyValue>,
    pub body_type: Option<String>,
    pub body_content: Option<String>,
    pub environment_id: Option<Uuid>,
    pub collection_id: Option<Uuid>,
    pub saved_request_id: Option<Uuid>,
}

pub async fn execute_api(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Json(payload): Json<ExecuteApiRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Chuẩn hóa resolve biến tại Backend (sinh cặp resolved thực thi & sanitized audit)
    let bundle = resolve_env_vars(&state.db, &payload, payload.environment_id).await?;

    // 2. Kiểm tra RBAC trên URL đã resolve
    let permission = db::api_tester::check_tester_permission(
        &state.db,
        claims.user_id,
        &payload.method,
        &bundle.resolved_url,
        payload.environment_id,
    ).await?;

    let permission = permission.ok_or(AppError::Forbidden)?;

    // 3. Nếu require_approval → tạo approval request với thông tin ĐÃ ĐƯỢC SANITIZED
    if permission.require_approval {
        let approval_id = db::api_tester::create_approval_request(
            &state.db, claims.user_id, &payload.method, &bundle, payload.environment_id
        ).await?;
        return Ok(Json(serde_json::json!({
            "status": "pending_approval",
            "approval_id": approval_id,
            "message": "Yêu cầu đã được chuyển tới Checker phê duyệt trước khi thực thi"
        })));
    }

    // 4. Ghi audit log TRƯỚC khi execute (dùng sanitized data, không lộ secret)
    let history_id = db::api_tester::insert_execution_history_pending(
        &state.db,
        claims.user_id,
        &payload.method,
        &bundle.sanitized_url,
        &bundle.sanitized_headers,
        &bundle.sanitized_body,
        payload.environment_id,
    ).await?;

    // 5. Build HttpRequest và gọi qua HTTP Executor (dùng resolved data thực tế)
    let http_req = HttpRequest {
        method: payload.method.clone(),
        url: bundle.resolved_url,
        headers: bundle.resolved_headers,
        body: bundle.resolved_body,
        timeout_secs: 30,
        accept_invalid_certs: false, // Phụ thuộc vào sensitivity của environment
        allow_internal_ips: permission.allow_internal_ips.unwrap_or(false),
    };

    let response = execute_http_request(http_req).await;

    // 6. Cập nhật audit log với kết quả thực thi
    match &response {
        Ok(resp) => {
            db::api_tester::update_execution_history_success(
                &state.db, history_id, resp.status, &resp.headers, &resp.body, resp.duration_ms
            ).await?;
        }
        Err(e) => {
            db::api_tester::update_execution_history_error(
                &state.db, history_id, &e.to_string()
            ).await?;
        }
    }

    Ok(Json(response?))
}

---

## 5. RBAC Design

### 5.1 Model phân quyền

| Tầng | Mô tả |
|------|-------|
| **Role** | Admin, Maker, Checker (kế thừa từ SECS hiện tại) |
| **Permission** | Cấp theo `user_id + url_pattern + methods + environment` |
| **Approval** | Một số endpoint/environment yêu cầu Maker/Checker workflow |

### 5.2 Use cases phân quyền

**Scenario 1 — Developer test API staging:**
- User: `alice` (role: Maker)
- Permission: `url_pattern = "https://staging.api.company.com/*"`, `methods = [GET, POST]`, `environment = staging`, `require_approval = false`
- Kết quả: Alice có thể test tự do, ghi audit log nhưng không cần approve

**Scenario 2 — Vận hành test API production:**
- User: `bob` (role: Maker)
- Permission: `url_pattern = "https://prod.api.company.com/users/*"`, `methods = [GET]`, `environment = production`, `require_approval = false`
- Permission: `url_pattern = "https://prod.api.company.com/users/*"`, `methods = [POST, PUT, DELETE]`, `environment = production`, `require_approval = true`
- Kết quả: Bob xem tự do, nhưng mutating calls phải có Checker approve trước khi execute

**Scenario 3 — Admin:**
- Admin thấy tất cả execution history
- Admin cấp/thu hồi permissions

### 5.3 Admin UI để quản lý permissions

```
Trang: Admin > API Tester Permissions
┌─────────────────────────────────────────────────────────┐
│ + Cấp quyền mới                                         │
├──────────┬─────────────────────────┬────────┬───────────┤
│ User     │ URL Pattern             │ Methods│ Env       │
├──────────┼─────────────────────────┼────────┼───────────┤
│ alice    │ https://staging.api.*/  │ GET,POST│ staging  │
│ bob      │ https://prod.api.*/users│ GET    │ prod      │
│ bob      │ https://prod.api.*/users│ POST   │ prod ⚠️   │
└──────────┴─────────────────────────┴────────┴───────────┘
                                             ⚠️ = require_approval
```

---

## 6. Frontend Design

### 6.1 Layout trang API Tester

```
┌────────────────────────────────────────────────────────────────┐
│ SECS > API Tester                              [+ New Request] │
├──────────────┬─────────────────────────────────────────────────┤
│              │                                                   │
│  Collections │  GET   https://api.example.com/users  [Send]    │
│              │ ────────────────────────────────────────────── │
│  📁 My APIs  │  Headers  Params  Body  Auth                    │
│   └─ GET /   │  ┌────────────────────────────────────────────┐ │
│   └─ POST /  │  │ Authorization  Bearer {{token}}            │ │
│              │  │ Content-Type   application/json            │ │
│  📁 Shared   │  └────────────────────────────────────────────┘ │
│   └─ ...     │                                                   │
│              ├─────────────────────────────────────────────────┤
│  Environments│  Response                200 OK  145ms  2.3KB   │
│  🌍 Staging  │  ────────────────────────────────────────────── │
│  🌍 Prod ⚠️  │  Body  Headers  History                         │
│              │  ┌────────────────────────────────────────────┐ │
│              │  │ {                                          │ │
│              │  │   "users": [...]                           │ │
│              │  │ }                                          │ │
│              │  └────────────────────────────────────────────┘ │
└──────────────┴─────────────────────────────────────────────────┘
```

### 6.2 Key Svelte Components

#### RequestPanel.svelte
```javascript
// Dùng CodeMirror 6 cho body editor với JSON syntax highlighting
import { json } from "@codemirror/lang-json"
import { autocompletion, completeFromList } from "@codemirror/autocomplete"

// Autocomplete cho headers phổ biến
const commonHeaders = completeFromList([
  { label: "Authorization", detail: "Bearer token" },
  { label: "Content-Type", detail: "application/json" },
  { label: "Accept", detail: "application/json" },
  { label: "X-Request-ID", detail: "Unique request ID" },
  { label: "Cache-Control", detail: "no-cache" },
])
```

#### KeyValueEditor.svelte — Reusable component
```svelte
<!-- Dùng cho cả Headers, Params, Form body -->
<script>
  export let items = []; // [{key, value, enabled}]
  export let placeholder = { key: "Header name", value: "Value" };
  
  function addRow() {
    items = [...items, { key: '', value: '', enabled: true }];
  }
  
  function removeRow(i) {
    items = items.filter((_, idx) => idx !== i);
  }
</script>

<div class="space-y-1">
  {#each items as item, i}
    <div class="flex gap-2 items-center">
      <input type="checkbox" bind:checked={item.enabled} class="checkbox checkbox-sm" />
      <input bind:value={item.key}   placeholder={placeholder.key}   class="input input-sm input-bordered flex-1" />
      <input bind:value={item.value} placeholder={placeholder.value} class="input input-sm input-bordered flex-1" />
      <button on:click={() => removeRow(i)} class="btn btn-ghost btn-xs">✕</button>
    </div>
  {/each}
  <button on:click={addRow} class="btn btn-ghost btn-xs">+ Add</button>
</div>
```

#### ResponsePanel.svelte
- Tab **Body**: CodeMirror read-only với JSON pretty-print, syntax highlighting
- Tab **Headers**: Key-value table hiển thị response headers
- Tab **History**: Danh sách các lần execute gần nhất (lấy từ `api_execution_history`)
- Status badge màu (xanh 2xx, vàng 3xx, đỏ 4xx/5xx)
- Thời gian (ms) và kích thước response

### 6.3 Quản lý Biến Môi trường & Quy tắc Frontend

#### 6.3.1 Nguyên tắc Không Resolve Secret tại Client
- **Raw Template Sending**: Khi người dùng nhấn "Send", Frontend gửi nguyên mẫu URL, Headers, Params và Body chứa cú pháp `{{variable_name}}`. Frontend **không tự ý thay thế các biến bí mật** trước khi gửi API.
- **Biến Secret**: Trong màn hình quản trị hoặc thanh chọn Environment, các biến có `is_secret = true` chỉ hiển thị dạng `••••••••`. API backend `/api/tester/environments/:id/variables` chỉ trả về `is_secret: true` và chuỗi masked (tuyệt đối không trả về plaintext).
- **Client-Side Autocomplete & Non-Secret Preview**: Frontend chỉ dùng danh sách tên biến để hỗ trợ autocomplete (qua CodeMirror gợi ý `{{...}}`) và chỉ cho phép preview trước giá trị của các biến thông thường (non-secret).

```javascript
// frontend/src/api/tester.js
// Frontend chỉ hỗ trợ preview các biến non-secret cho mục đích trải nghiệm người dùng (UX)
export function previewNonSecretTemplate(text, envVars) {
  return text.replace(/\{\{([a-zA-Z0-9_$.]+)\}\}/g, (match, key) => {
    const found = envVars.find(v => v.key === key);
    if (!found) return match;
    if (found.is_secret) return "{{••••••••}}"; // Luôn che giấu secret ở client preview
    return found.value;
  });
}
```

---

## 7. Security Considerations

| Rủi ro | Biện pháp |
|--------|-----------|
| **SSRF (Server-Side Request Forgery) & DNS Rebinding** | Triển khai `SafeDnsResolver` ở tầng socket (hook vào `reqwest::dns::Resolve`); chặn phân giải ra IP thuộc RFC 1918 (10.x, 172.16-31.x, 192.168.x), Loopback (127.x, ::1), Link-local / Cloud metadata (169.254.169.254, fe80::/10), CGNAT (100.64.x); Whitelist scheme chỉ cho phép `http/https`; Giới hạn tối đa 5 lần redirect và tái thẩm định IP an toàn qua DNS resolver ở từng hop. |
| **Secret leakage trong audit log & approvals** | Backend phân tách 2 luồng: `ResolvedPayload` (thực thi) và `SanitizedPayload` (lưu vết); Mọi biến `is_secret = true` và các headers `Authorization`, `X-Api-Key` tự động mask thành `[REDACTED]` khi lưu vào `api_execution_history` hoặc hiển thị cho Checker duyệt; Response body truncate ở 1MB. |
| **Bypass TLS validation** | `accept_invalid_certs` chỉ bật với dev/staging env, ghi rõ cờ này trong audit log. |
| **Unauthorized escalation** | Backend enforce RBAC trên URL đã resolve, frontend chỉ hiện nút Send nếu có quyền (UX) nhưng backend là nguồn sự thật duy nhất. |
| **Large response DoS** | Giới hạn response size (1MB), timeout 30s. |
| **Header injection & CRLF** | Dùng kiểu dữ liệu `HeaderName` và `HeaderValue` của `reqwest`/`http` crate để tự động sanitize ký tự xuống dòng `\r\n`. |

---

## 8. API Endpoints mới

| Method | Path | Mô tả | Auth |
|--------|------|--------|------|
| `POST` | `/api/tester/execute` | Thực thi HTTP request | RequireAuth |
| `GET` | `/api/tester/collections` | Danh sách collections của user | RequireAuth |
| `POST` | `/api/tester/collections` | Tạo collection mới | RequireAuth |
| `PUT` | `/api/tester/collections/:id` | Cập nhật collection | RequireAuth |
| `DELETE` | `/api/tester/collections/:id` | Xóa collection | RequireAuth |
| `GET` | `/api/tester/collections/:id/requests` | Danh sách requests trong collection | RequireAuth |
| `POST` | `/api/tester/collections/:id/requests` | Lưu request vào collection | RequireAuth |
| `GET` | `/api/tester/environments` | Danh sách environments | RequireAuth |
| `POST` | `/api/tester/environments` | Tạo environment mới | RequireAuth |
| `GET` | `/api/tester/environments/:id/variables` | Lấy variables (secret ẩn) | RequireAuth |
| `GET` | `/api/tester/history` | Lịch sử execute của user | RequireAuth |
| `GET` | `/api/tester/history/all` | Lịch sử tất cả users | RequireAdmin |
| `GET` | `/api/tester/permissions` | Danh sách quyền | RequireAdmin |
| `POST` | `/api/tester/permissions` | Cấp quyền | RequireAdmin |
| `DELETE` | `/api/tester/permissions/:id` | Thu hồi quyền | RequireAdmin |

---

## 9. Tích hợp với Workflow hiện có của SECS

### 9.1 Khi `require_approval = true`

Thay vì execute ngay, hệ thống tạo một bản ghi tương tự `SqlRequest` nhưng cho API call:

```
Maker gửi API call
    → Backend tạo ApiCallRequest (trạng thái PENDING)
    → Checker nhận notification (WebSocket)
    → Checker review: URL, method, headers, body
    → Checker approve → Backend execute
    → Kết quả ghi vào history, notify Maker
```

Tái sử dụng hoàn toàn:
- Bảng `approvals` và approval workflow hiện tại
- WebSocket notification system
- Audit log infrastructure

### 9.2 Dashboard integration

- **Maker dashboard**: thêm widget "Recent API Tests"
- **Checker dashboard**: thêm "Pending API Call Approvals"
- **Admin dashboard**: thêm stats về số API calls theo environment

---

## 10. Thư viện & Dependencies

### Backend (Rust)

| Crate | Version | Mục đích |
|-------|---------|----------|
| `reqwest` | `0.12` | HTTP client để execute requests (kèm trait `reqwest::dns::Resolve`) |
| `glob` | `0.3` | Pattern matching cho URL permissions |
| `url` | `2.5` | Phân tích và kiểm tra URL scheme (`http`/`https`) |
| `urlencoding` | `2.1` | Encode query params khi resolve biến môi trường |
| `tokio` | `1.x` | Bật feature `net` phục vụ `tokio::net::lookup_host` trong DNS resolver |
| Các crate hiện có | — | sqlx, axum, serde, chrono, uuid, regex, v.v. |

Thêm vào `Cargo.toml`:
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
glob = "0.3"
url = "2.5"
urlencoding = "2.1"
tokio = { version = "1", features = ["full"] }
```

### Frontend (Svelte)

| Package | Mục đích |
|---------|----------|
| `@codemirror/lang-json` | JSON syntax highlighting cho body editor |
| `@codemirror/autocomplete` | Autocomplete header names |
| Đã có CodeMirror 6 | Từ SqlEditor.svelte hiện tại — **tái sử dụng** |

---

## 11. Roadmap triển khai

### Phase 1 — MVP (2-3 tuần)
- [ ] Database migrations (api_collections, api_saved_requests, api_environments, api_env_variables, api_tester_permissions, api_execution_history)
- [ ] Backend: `executor/http_executor.rs` dùng reqwest
- [ ] Backend: RBAC check + SSRF protection
- [ ] Backend: API endpoints cơ bản (execute + collections + environments)
- [ ] Frontend: ApiTester.svelte layout cơ bản
- [ ] Frontend: RequestPanel + ResponsePanel
- [ ] Frontend: CollectionSidebar

### Phase 2 — RBAC & Approval (1-2 tuần)
- [ ] Admin UI: quản lý permissions
- [ ] Approval workflow cho `require_approval = true`
- [ ] History view đầy đủ
- [ ] Environment variables với secret masking

### Phase 3 — Polish (1 tuần)
- [ ] Import/Export Postman Collection v2.1 format
- [ ] Keyboard shortcuts (Ctrl+Enter để send)
- [ ] Response search/filter
- [ ] Bulk permission management

---

## 12. Nguồn tham khảo

- [reqwest docs — HTTP client](https://github.com/seanmonstar/reqwest) — Xây dựng HTTP executor
- [Axum middleware — from_extractor](https://docs.rs/axum/0.7.9/axum/middleware/fn.from_extractor.html) — RBAC pattern
- [Axum middleware — from_fn](https://docs.rs/axum/0.7.9/axum/middleware/fn.from_fn.html) — Auth middleware
- [CodeMirror Autocomplete — completeFromList](https://github.com/codemirror/autocomplete) — Header autocomplete
- [reqwest — TLS configuration](https://github.com/seanmonstar/reqwest) — TLS options cho dev/prod env
