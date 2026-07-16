---
inclusion: auto
description: "Authentication — Reference cho JWT & Validation."
---

# Authentication — Reference cho JWT & Validation

---

## JSONWEBTOKEN

Nguồn: https://github.com/keats/jsonwebtoken

### Dependency (Cargo.toml)

```toml
[dependencies]
jsonwebtoken = "9"
serde = { version = "1", features = ["derive"] }
```

---

### 1. Định nghĩa Claims

```rust
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// Claims struct chuẩn cho SECS.
/// - `sub`: JWT standard field (user_id dưới dạng String)
/// - `user_id`: type-safe Uuid để dùng trực tiếp trong handlers, tránh parse mỗi lần
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    // Standard JWT claims
    pub sub: String,          // subject — user ID (String theo JWT standard)
    pub exp: usize,           // expiration timestamp (Unix), max 8 giờ
    pub iat: usize,           // issued at timestamp (Unix)
    pub iss: String,          // issuer: "secs-backend"

    // Custom claims cho SECS
    pub user_id: Uuid,        // type-safe UUID để dùng trong DB queries
    pub email: String,
    pub roles: Vec<String>,   // ["MAKER", "CHECKER"]
}
```

---

### 2. Encode — Tạo JWT token

```rust
use jsonwebtoken::{encode, Header, EncodingKey, Algorithm};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

pub fn create_jwt_token(
    user_id: Uuid,
    email: &str,
    roles: Vec<String>,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    // JWT tối đa 8 giờ theo security rules
    let expiry = now + (8 * 60 * 60);

    let claims = Claims {
        sub: user_id.to_string(),  // JWT standard: sub là String
        exp: expiry,
        iat: now,
        iss: "secs-backend".to_string(),
        user_id,                   // type-safe Uuid cho handlers
        roles,
        email: email.to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    let key = EncodingKey::from_secret(secret.as_bytes());

    encode(&header, &claims, &key)
}
```

---

### 3. Decode & Validate — Xác thực token

```rust
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

pub fn verify_jwt_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);

    // Validate exp tự động (mặc định là true)
    validation.validate_exp = true;

    let token_data = decode::<Claims>(token, &key, &validation)?;
    Ok(token_data.claims)
}
```

---

### 4. Axum Extractor cho Auth Middleware

```rust
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::{StatusCode, HeaderMap},
};

// Middleware xác thực JWT
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // Lấy token từ Authorization header
    let token = extract_bearer_token(request.headers())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing authorization token".to_string()))?;

    // Verify token
    let claims = verify_jwt_token(&token, &state.config.jwt_secret)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                (StatusCode::UNAUTHORIZED, "Token expired".to_string())
            }
            _ => (StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
        })?;

    // Inject claims vào request extensions để handler dùng
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

// Extractor để lấy claims trong handler
use axum::extract::Extension;

async fn protected_handler(
    Extension(claims): Extension<Claims>,
) -> String {
    format!("Hello, {}!", claims.username)
}
```

---

### 5. Role-Based Access Control (RBAC)

```rust
// Kiểm tra quyền trong handler
pub fn require_role(claims: &Claims, required_role: &str) -> Result<(), AppError> {
    if claims.roles.contains(&required_role.to_string()) {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!("Role {} required", required_role)))
    }
}

// 4-eyes principle: Maker không được tự approve
pub fn check_4_eyes(claims: &Claims, maker_id: Uuid) -> Result<(), AppError> {
    if claims.user_id == maker_id {
        Err(AppError::Forbidden(
            "Maker cannot approve their own request (4-eyes principle)".to_string()
        ))
    } else {
        Ok(())
    }
}

// Dùng trong handler
async fn approve_request(
    Extension(claims): Extension<Claims>,
    Path(request_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ApprovalResponse>, AppError> {
    require_role(&claims, "CHECKER")?;

    let request = db::requests::find_by_id(&state.db, request_id)
        .await?
        .ok_or(AppError::NotFound("Request not found".into()))?;

    check_4_eyes(&claims, request.maker_id)?;

    // ... approve logic
}
```

---

## VALIDATOR

Nguồn: https://github.com/keats/validator

### Dependency (Cargo.toml)

```toml
[dependencies]
validator = { version = "0.18", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
```

---

### 1. Validate Request Body

```rust
use validator::{Validate, ValidationError};
use serde::{Serialize, Deserialize};

// Tạo SQL request mới
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequestBody {
    #[validate(length(min = 1, max = 200, message = "Title is required (max 200 chars)"))]
    pub title: String,

    #[validate(length(min = 1, message = "SQL content cannot be empty"))]
    pub sql_content: String,

    #[validate(custom(function = "validate_uuid_string"))]
    pub database_id: String,

    #[validate(length(max = 1000))]
    pub note: Option<String>,
}

// Approve/Reject request
#[derive(Debug, Deserialize, Validate)]
pub struct RejectBody {
    #[validate(length(min = 10, message = "Rejection reason must be at least 10 characters"))]
    pub reason: String,
}

// Custom validator
fn validate_uuid_string(value: &str) -> Result<(), ValidationError> {
    uuid::Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| ValidationError::new("invalid_uuid"))
}
```

---

### 2. Axum Extractor kết hợp Validation

```rust
use axum::{
    extract::{FromRequest, Request},
    http::StatusCode,
    Json,
    response::{IntoResponse, Response},
};
use validator::Validate;
use serde::de::DeserializeOwned;

// Custom extractor: vừa parse JSON vừa validate
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| {
                (StatusCode::BAD_REQUEST, e.to_string()).into_response()
            })?;

        value.validate().map_err(|errors| {
            (StatusCode::UNPROCESSABLE_ENTITY, Json(errors)).into_response()
        })?;

        Ok(ValidatedJson(value))
    }
}

// Sử dụng trong handler
async fn create_request(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<CreateRequestBody>,
) -> Result<Json<RequestResponse>, AppError> {
    // body đã được validate — an toàn để dùng
    todo!()
}
```

---

### 3. Các validation rules hay dùng

```rust
use validator::Validate;

#[derive(Validate)]
struct Examples {
    // Email format
    #[validate(email)]
    email: String,

    // Length
    #[validate(length(min = 8, max = 128))]
    password: String,

    // Range số
    #[validate(range(min = 1, max = 100))]
    page_size: u32,

    // Regex
    #[validate(regex(path = "USERNAME_REGEX"))]
    username: String,

    // Required Option
    #[validate(required)]
    accept_terms: Option<bool>,

    // Nested struct validation
    #[validate(nested)]
    address: Address,

    // Custom error message
    #[validate(length(min = 1, message = "Field cannot be empty"))]
    title: String,
}
```

---

## Quy tắc áp dụng trong dự án SECS

### JWT
- Token expire tối đa 8 giờ — bắt buộc theo security rules
- Dùng `HS256` cho internal token; nếu SSO trả về RS256 thì verify với public key
- Không log token value, không trả token trong error response
- Claims phải chứa `user_id`, `roles` để RBAC hoạt động đúng

### Validator
- Tất cả request body phải dùng `#[derive(Validate)]`
- Dùng `ValidatedJson<T>` extractor thay `Json<T>` trong handler
- Rejection reason khi reject phải có minimum length (tránh lý do trống)
- SQL content không validate nội dung (không filter SQL) — chỉ check không rỗng

### RBAC
- Luôn kiểm tra role trước khi thực hiện hành động trong handler
- 4-eyes principle: luôn `check_4_eyes()` trước khi approve/reject
- Phân quyền theo DatabaseConnection — cần verify user có quyền trên DB đó
