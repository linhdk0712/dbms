---
inclusion: auto
description: "Axum 0.7.x — Reference cho Backend HTTP Layer."
---

# Axum 0.7.x — Reference cho Backend HTTP Layer

Nguồn: https://docs.rs/axum/0.7.9/axum/

## Dependency (Cargo.toml)

```toml
[dependencies]
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "timeout"] }
tokio = { version = "1", features = ["full"] }
```

---

## 1. Router — Định nghĩa Routes

```rust
use axum::{Router, routing::{get, post, put, delete}};

let app = Router::new()
    .route("/", get(root_handler))
    .route("/users", get(list_users).post(create_user))
    .route("/users/:id", get(show_user).put(update_user).delete(delete_user))
    .route("/assets/*path", get(serve_asset));
```

### Nest Router (cho cấu trúc /api/v1/...)

```rust
use axum::{Router, routing::{get, post}};

let request_routes = Router::new()
    .route("/", get(list_requests).post(create_request))
    .route("/:id", get(get_request));

let approval_routes = Router::new()
    .route("/:id/approve", post(approve_request))
    .route("/:id/reject", post(reject_request));

let api_v1 = Router::new()
    .nest("/requests", request_routes)
    .nest("/approvals", approval_routes)
    .nest("/databases", database_routes);

let app = Router::new().nest("/api/v1", api_v1);
// Kết quả: POST /api/v1/requests, POST /api/v1/approvals/:id/approve, ...
```

---

## 2. State — Chia sẻ AppState

```rust
use axum::{Router, routing::get, extract::State};

// State phải implement Clone
#[derive(Clone)]
struct AppState {
    db: sqlx::PgPool,
    jwt_secret: String,
}

let state = AppState {
    db: pool,
    jwt_secret: "secret".into(),
};

let app = Router::new()
    .route("/", get(handler))
    .with_state(state);

// Trong handler:
async fn handler(State(state): State<AppState>) {
    let _pool = &state.db;
}
```

---

## 3. Extractors — Lấy dữ liệu từ Request

```rust
use axum::extract::{Path, Query, Json, State};
use std::collections::HashMap;
use serde::Deserialize;

// Path parameter: /users/:id
async fn get_user(Path(id): Path<i64>) {}

// Query string: /users?page=1&limit=20
#[derive(Deserialize)]
struct Pagination { page: u32, limit: u32 }
async fn list_users(Query(params): Query<Pagination>) {}

// JSON body (phải là extractor cuối cùng)
#[derive(Deserialize)]
struct CreateUser { username: String, email: String }
async fn create_user(Json(payload): Json<CreateUser>) {}

// Kết hợp nhiều extractor
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdatePayload>,
) {}
```

---

## 4. Response — Trả về kết quả

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
    Json,
};
use serde::Serialize;

// Trả về JSON
#[derive(Serialize)]
struct UserResponse { id: i64, username: String }

async fn get_user() -> Json<UserResponse> {
    Json(UserResponse { id: 1, username: "alice".into() })
}

// Trả về với StatusCode
async fn create_user() -> (StatusCode, Json<UserResponse>) {
    (StatusCode::CREATED, Json(UserResponse { id: 2, username: "bob".into() }))
}

// Custom error response
async fn handler() -> Result<Json<UserResponse>, (StatusCode, String)> {
    Err((StatusCode::NOT_FOUND, "User not found".to_string()))
}
```

---

## 5. Middleware với Tower

```rust
use axum::{Router, routing::get, http::StatusCode, BoxError};
use axum::error_handling::HandleErrorLayer;
use tower::{ServiceBuilder, timeout::TimeoutLayer};
use std::time::Duration;

let app = Router::new()
    .route("/", get(handler))
    .layer(
        ServiceBuilder::new()
            // Xử lý lỗi từ TimeoutLayer
            .layer(HandleErrorLayer::new(|_: BoxError| async {
                StatusCode::REQUEST_TIMEOUT
            }))
            // Timeout mỗi request (áp dụng cho SQL execution)
            .layer(TimeoutLayer::new(Duration::from_secs(30)))
    );
```

---

## 6. WebSocket — Realtime Notification

```rust
use axum::{
    extract::{ws::{WebSocketUpgrade, WebSocket, Message}, State},
    response::Response,
    routing::get,
    Router,
};

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                // xử lý message
                let _ = socket.send(Message::Text(text)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

// Route:
let app = Router::new()
    .route("/ws", get(ws_handler))
    .with_state(state);
```

---

## 7. IntoResponse — Custom Error Type

```rust
use axum::{response::{IntoResponse, Response}, http::StatusCode, Json};
use serde_json::json;

pub enum AppError {
    NotFound(String),
    Unauthorized,
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".into()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

// Handler dùng AppError:
async fn get_request(Path(id): Path<i64>) -> Result<Json<RequestResponse>, AppError> {
    // ...
    Err(AppError::NotFound(format!("Request {} not found", id)))
}
```

---

## Quy tắc áp dụng trong dự án SECS

- Handler chỉ extract request + gọi db layer → trả response, không chứa business logic
- Dùng `AppState` để inject `PgPool` và `jwt_secret` vào tất cả handlers
- `Json` extractor phải là extractor cuối cùng trong handler
- Tất cả error trả về phải qua `AppError` implement `IntoResponse`
- Timeout middleware bắt buộc cho mọi route liên quan đến SQL execution
