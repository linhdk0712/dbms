---
inclusion: manual
---

# thiserror & Error Handling — Reference cho Error Layer

Nguồn: https://github.com/dtolnay/thiserror

## Dependency (Cargo.toml)

```toml
[dependencies]
thiserror = "2"
anyhow = "1"    # cho application-level errors
```

---

## 1. Định nghĩa Custom Error với `thiserror`

```rust
// backend/src/error.rs
use thiserror::Error;
use axum::{response::{IntoResponse, Response}, http::StatusCode, Json};
use serde_json::json;

#[derive(Error, Debug)]
pub enum AppError {
    // Database errors
    #[error("Database error: {0}")]
    Database(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    // Auth errors
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    // Business logic errors
    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Invalid state transition: cannot move from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("SQL execution failed: {0}")]
    ExecutionFailed(String),

    #[error("SQL execution timed out")]
    ExecutionTimeout,

    // Wrapping upstream errors
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("SQLx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// Implement IntoResponse để Axum tự động convert khi handler trả Err(AppError)
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            AppError::InvalidStateTransition { from, to } => (
                StatusCode::CONFLICT,
                format!("Cannot transition from {} to {}", from, to),
            ),
            AppError::ExecutionTimeout => (
                StatusCode::REQUEST_TIMEOUT,
                "SQL execution timed out".to_string(),
            ),
            AppError::ExecutionFailed(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            // 5xx — không expose internal details
            AppError::Database(_) | AppError::Sqlx(_) | AppError::Other(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            AppError::Jwt(_) => (StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
        };

        // Log lỗi internal (không expose ra client)
        if status.is_server_error() {
            tracing::error!(error = %self, "Internal server error");
        }

        (status, Json(json!({ "error": error_message }))).into_response()
    }
}
```

---

## 2. Tất cả `#[error(...)]` Attribute Patterns

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExampleErrors {
    // Đơn giản — string tĩnh
    #[error("something went wrong")]
    Simple,

    // Hiển thị tuple field
    #[error("invalid value: {0}")]
    InvalidValue(String),

    // Hiển thị named field
    #[error("expected {expected}, got {found}")]
    Mismatch { expected: String, found: String },

    // Debug format
    #[error("unexpected type: {0:?}")]
    UnexpectedType(SomeType),

    // Auto-convert từ upstream error (From impl tự động)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // Wrap error và delegate Display + source
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}
```

---

## 3. Error trong các Layer khác nhau

### db/ layer — trả `Result<T, AppError>`

```rust
// backend/src/db/requests.rs
use crate::error::AppError;

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<SqlRequest, AppError> {
    sqlx::query_as!(SqlRequest, "SELECT * FROM sql_requests WHERE id = $1", id)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::NotFound(format!("Request {} not found", id)),
            other => AppError::Sqlx(other),
        })
}
```

### executor/ layer — error riêng cho target DB

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Query execution failed: {0}")]
    QueryFailed(String),

    #[error("Execution timed out after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("Unsupported database type: {0}")]
    UnsupportedDbType(String),
}

// Convert sang AppError khi cần
impl From<ExecutorError> for AppError {
    fn from(e: ExecutorError) -> Self {
        match e {
            ExecutorError::Timeout { .. } => AppError::ExecutionTimeout,
            other => AppError::ExecutionFailed(other.to_string()),
        }
    }
}
```

### Handler — trả `Result<Json<T>, AppError>`

```rust
async fn approve_request(
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(body): Json<ApproveBody>,
) -> Result<Json<RequestResponse>, AppError> {
    // ? operator tự động convert nhờ #[from] trong AppError
    let request = db::requests::find_by_id(&state.db, id).await?;

    check_4_eyes(&claims, request.maker_id)?;

    // ...
    Ok(Json(response))
}
```

---

## 4. `anyhow` cho Application-level Errors

```rust
use anyhow::{Context, Result, bail, ensure};

// Thêm context vào error
fn load_config() -> Result<Config> {
    let content = std::fs::read_to_string(".env")
        .context("Failed to read .env file")?;
    // ...
}

// bail! — trả Err ngay lập tức
fn validate_something(value: &str) -> Result<()> {
    if value.is_empty() {
        bail!("Value cannot be empty");
    }
    Ok(())
}

// ensure! — assert với error message
fn process(n: u32) -> Result<()> {
    ensure!(n > 0, "n must be positive, got {}", n);
    Ok(())
}
```

---

## Quy tắc áp dụng trong dự án SECS

- `AppError` là error type duy nhất ở API layer — tất cả error phải convert về đây
- Lỗi 5xx không expose internal message ra client — chỉ log phía server
- `#[from]` cho `sqlx::Error` và `jsonwebtoken::Error` để dùng `?` operator tự nhiên
- `executor/` có error type riêng (`ExecutorError`) — convert sang `AppError` khi trả về API
- Không dùng `unwrap()` hoặc `expect()` trong production code
- Dùng `tracing::error!` để log internal errors (không dùng `println!`)
