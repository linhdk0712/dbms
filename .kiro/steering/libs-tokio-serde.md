---
inclusion: manual
---

# Tokio & Serde — Reference cho Async Runtime và Serialization

---

## TOKIO

Nguồn: https://docs.rs/tokio/latest/tokio/

### Dependency (Cargo.toml)

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
```

---

### 1. Entry Point — `#[tokio::main]`

```rust
// main.rs
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Khởi tạo tracing/logging
    tracing_subscriber::init();

    // Load config
    let config = Config::from_env()?;

    // Kết nối database
    let pool = PgPool::connect(&config.database_url).await?;

    // Chạy migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Khởi tạo state
    let state = AppState { db: pool, config };

    // Build router
    let app = create_router(state);

    // Bind và serve
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

### 2. `tokio::spawn` — Chạy Task Bất Đồng Bộ

```rust
use tokio::task::JoinHandle;

// Spawn task không cần đợi kết quả
tokio::spawn(async move {
    notify_websocket_clients(notification).await;
});

// Spawn và đợi kết quả
let handle: JoinHandle<String> = tokio::spawn(async {
    expensive_operation().await
});
let result = handle.await?; // propagate panic nếu có

// Spawn blocking (CPU-intensive hoặc sync I/O)
let result = tokio::task::spawn_blocking(|| {
    heavy_crypto_operation()
}).await?;
```

---

### 3. Channels — Giao tiếp giữa Tasks

#### `broadcast` — Gửi notification tới nhiều WebSocket clients

```rust
use tokio::sync::broadcast;

// Tạo channel với buffer 100 messages
let (tx, _rx) = broadcast::channel::<NotificationEvent>(100);

// Clone tx để share sang nhiều task
let tx_clone = tx.clone();
tokio::spawn(async move {
    tx_clone.send(NotificationEvent::NewRequest { id: request_id }).unwrap();
});

// Subscribe trong WebSocket handler
let mut rx = tx.subscribe();
tokio::spawn(async move {
    loop {
        match rx.recv().await {
            Ok(event) => send_to_client(event).await,
            Err(broadcast::error::RecvError::Lagged(n)) => {
                // Client bị lag, bỏ qua n messages
                tracing::warn!("WebSocket client lagged {} messages", n);
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
});
```

#### `mpsc` — Gửi task vào queue xử lý

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel::<ExecutionJob>(32);

// Worker nhận và xử lý jobs
tokio::spawn(async move {
    while let Some(job) = rx.recv().await {
        execute_sql_job(job).await;
    }
});

// Gửi job sau khi approve
tx.send(ExecutionJob { request_id, sql, db_conn }).await?;
```

#### `Mutex` và `RwLock` — Shared mutable state

```rust
use tokio::sync::{Mutex, RwLock};
use std::sync::Arc;

// RwLock: nhiều reader, một writer — dùng cho cache
let cache: Arc<RwLock<HashMap<Uuid, DbConnection>>> = Arc::new(RwLock::new(HashMap::new()));

// Read
let guard = cache.read().await;
let conn = guard.get(&db_id);

// Write
let mut guard = cache.write().await;
guard.insert(db_id, connection);
```

---

### 4. `tokio::time` — Timeout cho SQL Execution

```rust
use tokio::time::{timeout, Duration};

// Timeout cho SQL execution (bắt buộc theo business rule)
let execution_timeout = Duration::from_secs(config.sql_execution_timeout_secs);

match timeout(execution_timeout, execute_sql(&conn, &sql)).await {
    Ok(Ok(result)) => Ok(result),
    Ok(Err(e)) => Err(AppError::ExecutionFailed(e.to_string())),
    Err(_elapsed) => {
        // Ghi audit log FAILED
        audit_log_timeout(request_id).await;
        Err(AppError::ExecutionTimeout)
    }
}
```

---

## SERDE

Nguồn: https://serde.rs/ | https://docs.rs/serde_json/

### Dependency (Cargo.toml)

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

### 1. Derive cơ bản

```rust
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SqlRequest {
    pub id: Uuid,
    pub title: String,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
}

// Serialize → JSON string
let json = serde_json::to_string(&request)?;

// Deserialize ← JSON string
let request: SqlRequest = serde_json::from_str(&json)?;
```

---

### 2. Attributes hay dùng

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]    // snake_case → camelCase cho JSON API
pub struct CreateRequestBody {
    pub sql_content: String,           // → "sqlContent" trong JSON
    pub database_id: String,           // → "databaseId" trong JSON
    pub execution_note: Option<String>,// → "executionNote"
}

#[derive(Serialize, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,

    // Không serialize trường này ra API response
    #[serde(skip_serializing)]
    pub password_hash: String,

    // Bỏ qua nếu None khi serialize
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<DateTime<Utc>>,

    // Đổi tên field trong JSON
    #[serde(rename = "role")]
    pub user_role: String,
}
```

---

### 3. Enum Serialization

```rust
use serde::{Serialize, Deserialize};

// Enum serialize thành string
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, sqlx::Type)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[sqlx(type_name = "request_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequestStatus {
    Draft,      // → "DRAFT"
    Pending,    // → "PENDING"
    Approved,   // → "APPROVED"
    Rejected,   // → "REJECTED"
    Executed,   // → "EXECUTED"
    Failed,     // → "FAILED"
}

// Enum với tagged content
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum NotificationEvent {
    NewRequest { request_id: Uuid, title: String },
    RequestApproved { request_id: Uuid },
    RequestRejected { request_id: Uuid, reason: String },
}
// → {"type": "NewRequest", "data": {"request_id": "...", "title": "..."}}
```

---

### 4. `serde_json::Value` — Dynamic JSON

```rust
use serde_json::{json, Value};

// Tạo JSON object động
let response = json!({
    "error": "Not found",
    "request_id": request_id.to_string(),
    "timestamp": Utc::now().to_rfc3339(),
});

// Parse JSON không biết schema trước
let value: Value = serde_json::from_str(&raw_json)?;
if let Some(status) = value.get("status").and_then(|v| v.as_str()) {
    println!("Status: {}", status);
}
```

---

## Quy tắc áp dụng trong dự án SECS

### Tokio
- Luôn dùng `timeout()` wrapper cho SQL execution — không để query chạy vô hạn
- Dùng `broadcast::channel` cho WebSocket notification (1 sender → nhiều clients)
- Dùng `spawn_blocking` cho các thao tác đồng bộ nặng (encrypt/decrypt credentials)
- Không `unwrap()` trong production code — dùng `?` hoặc `match`

### Serde
- API request bodies dùng `#[serde(rename_all = "camelCase")]` để phù hợp với frontend JS
- Response types luôn `#[serde(skip_serializing_if = "Option::is_none")]` để giữ JSON gọn
- Không bao giờ serialize `password_hash`, credentials, hoặc dữ liệu nhạy cảm
- `RequestStatus`, `DbType` enum phải đồng bộ cả `serde` và `sqlx::Type` attributes
