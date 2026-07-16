---
inclusion: auto
description: "Tổng hợp best practices từ các thư viện chính được sử dụng trong dự án, dựa trên nghiên cứu documentation thực tế."
---

# Tài liệu tham khảo triển khai — SECS

Tổng hợp best practices từ các thư viện chính được sử dụng trong dự án, dựa trên nghiên cứu documentation thực tế.

---

## 1. Axum — HTTP Framework

**Library ID**: `/websites/rs_axum_0_7_9_axum` | Version: 0.7.9

### 1.1 Middleware Authentication với JWT

Pattern chuẩn để xác thực JWT trong axum: dùng `middleware::from_fn` hoặc `from_extractor`.

```rust
use axum::{
    Router, http::StatusCode,
    routing::get,
    response::{IntoResponse, Response},
    middleware::{self, Next},
    extract::{Request, Extension},
};

#[derive(Clone)]
struct CurrentUser {
    id: uuid::Uuid,
    role: Role,
}

async fn auth_middleware(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(token) = auth_header {
        if let Some(user) = verify_jwt(token).await {
            req.extensions_mut().insert(user);
            return Ok(next.run(req).await);
        }
    }
    Err(StatusCode::UNAUTHORIZED)
}

// Handler nhận CurrentUser từ extension
async fn handler(Extension(user): Extension<CurrentUser>) {
    // user đã được xác thực
}

let app = Router::new()
    .route("/api/requests", get(handler))
    .route_layer(middleware::from_fn(auth_middleware));
```

### 1.2 Middleware với State (AppState)

Khi middleware cần truy cập shared state (DB pool, JWT secret):

```rust
use axum::extract::State;

#[derive(Clone)]
struct AppState {
    db: sqlx::PgPool,
    jwt_secret: String,
}

async fn auth_with_state(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // access state.jwt_secret để verify token
    next.run(request).await
}

let app = Router::new()
    .route("/", get(handler))
    .route_layer(middleware::from_fn_with_state(state.clone(), auth_with_state))
    .with_state(state);
```

### 1.3 Custom Extractor cho Authorization

Dùng `FromRequestParts` để tạo extractor tái sử dụng, phù hợp cho RBAC:

```rust
use axum::extract::FromRequestParts;
use async_trait::async_trait;

struct RequireChecker; // chỉ cho phép role Checker

#[async_trait]
impl<S> FromRequestParts<S> for RequireChecker
where S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, StatusCode> {
        let user = parts.extensions.get::<CurrentUser>()
            .ok_or(StatusCode::UNAUTHORIZED)?;
        if user.role == Role::Checker || user.role == Role::Admin {
            Ok(Self)
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}
```

### 1.4 Error Handling với Timeout

Bắt buộc xử lý timeout khi thực thi SQL trên database đích:

```rust
use axum::error_handling::HandleErrorLayer;
use tower::{ServiceBuilder, timeout::TimeoutLayer};
use std::time::Duration;

let app = Router::new()
    .route("/execute", post(execute_sql))
    .layer(
        ServiceBuilder::new()
            .layer(HandleErrorLayer::new(|err: BoxError| async {
                (StatusCode::REQUEST_TIMEOUT, format!("SQL execution timeout: {}", err))
            }))
            .layer(TimeoutLayer::new(Duration::from_secs(30)))
    );
```

### 1.5 WebSocket Handler cho Realtime Notifications

```rust
use axum::extract::ws::{WebSocketUpgrade, WebSocket, Message};
use axum::extract::State;

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Response {
    ws.on_upgrade(|socket| handle_ws(socket, state, user))
}

async fn handle_ws(mut socket: WebSocket, state: AppState, user: CurrentUser) {
    // Subscribe vào channel của user
    let mut rx = state.notification_tx.subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                if msg.is_none() { break; } // client disconnect
            }
            notif = rx.recv() => {
                if let Ok(notif) = notif {
                    if socket.send(Message::Text(notif)).await.is_err() {
                        break; // client disconnect
                    }
                }
            }
        }
    }
}
```

> **Pattern**: Dùng `tokio::broadcast` channel cho notification fanout (nhiều Checker cùng nhận khi có request mới).

---

## 2. SQLx — Database Access Layer

**Library ID**: `/websites/rs_sqlx_sqlx`

### 2.1 Connection Pool khởi tạo

```rust
use sqlx::postgres::{PgPoolOptions, PgConnectOptions};

let options = PgConnectOptions::new()
    .host(&config.db_host)
    .port(config.db_port)
    .username(&config.db_user)
    .password(&config.db_password)
    .database(&config.db_name);

let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(2)
    .after_connect(|conn, _meta| Box::pin(async move {
        conn.execute("SET application_name = 'secs-backend';").await?;
        Ok(())
    }))
    .connect_with(options)
    .await?;
```

### 2.2 Compile-time Checked Queries

Ưu tiên dùng `query!` và `query_as!` cho type safety tại compile time:

```rust
use sqlx::PgPool;

// query! — trả anonymous struct
let row = sqlx::query!(
    "SELECT id, status, sql_content FROM sql_requests WHERE id = $1",
    request_id
)
.fetch_one(&pool)
.await?;

// query_as! — map vào struct cụ thể (dùng cho return từ function)
#[derive(Debug)]
struct SqlRequest {
    id: uuid::Uuid,
    status: String,
    sql_content: String,
    created_by: uuid::Uuid,
}

let request = sqlx::query_as!(
    SqlRequest,
    "SELECT id, status, sql_content, created_by FROM sql_requests WHERE id = $1",
    request_id
)
.fetch_optional(&pool)
.await?;
```

> **Lưu ý**: Bind parameter trong PostgreSQL dùng `$1`, `$2`, ... (khác MySQL dùng `?`)

### 2.3 Transactions — Thực thi SQL nguyên tử

Bắt buộc wrap trong transaction để đảm bảo 4-eyes principle và rollback khi lỗi:

```rust
use sqlx::PgPool;

async fn execute_approved_request(
    pool: &PgPool,
    request_id: uuid::Uuid,
    executor_id: uuid::Uuid,
) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // 1. Ghi audit log TRƯỚC khi thực thi
    sqlx::query!(
        "INSERT INTO audit_log (request_id, action, actor_id) VALUES ($1, 'EXECUTED', $2)",
        request_id, executor_id
    )
    .execute(&mut *tx)
    .await?;

    // 2. Cập nhật trạng thái request
    sqlx::query!(
        "UPDATE sql_requests SET status = 'EXECUTED' WHERE id = $1",
        request_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(1)
    // Nếu bất kỳ bước nào lỗi, tx drop sẽ tự rollback
}
```

### 2.4 Testing với sqlx::test

```rust
#[sqlx::test]
async fn test_create_request(pool: PgPool) -> sqlx::Result<()> {
    let result = create_sql_request(&pool, /* params */).await;
    assert!(result.is_ok());
    Ok(())
}
```

> `#[sqlx::test]` tự động tạo và tear down test database isolation — không ảnh hưởng database thật.

---

## 3. OpenID Connect / OIDC — Xác thực SSO

**Library ID**: `/ramosbugs/openidconnect-rs`

### 3.1 Flow đầy đủ: Authorization Code + PKCE

```rust
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreProviderMetadata, CoreUserInfoClaims
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    IssuerUrl, Nonce, PkceCodeChallenge, RedirectUrl, Scope,
    TokenResponse, OAuth2TokenResponse, AccessTokenHash, reqwest,
};

// Bước 1: Khởi tạo client (chạy 1 lần khi startup)
async fn create_oidc_client(config: &OidcConfig) -> Result<CoreClient, ...> {
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new(config.issuer_url.clone())?,
        &http_client,
    ).await?;

    Ok(CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(config.client_id.clone()),
        Some(ClientSecret::new(config.client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(config.redirect_uri.clone())?))
}

// Bước 2: Tạo authorization URL, lưu csrf + nonce vào session
fn build_auth_url(client: &CoreClient) -> (Url, CsrfToken, Nonce, PkceCodeVerifier) {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    (auth_url, csrf_token, nonce, pkce_verifier)
}

// Bước 3: Xử lý callback, exchange code lấy token
async fn handle_oidc_callback(
    client: &CoreClient,
    http_client: &reqwest::Client,
    code: String,
    csrf_token_from_session: &CsrfToken,
    csrf_token_from_query: &str,
    nonce: &Nonce,
    pkce_verifier: PkceCodeVerifier,
) -> Result<UserInfo, Error> {
    // QUAN TRỌNG: xác minh CSRF trước
    assert_eq!(csrf_token_from_session.secret(), csrf_token_from_query);

    let token_response = client
        .exchange_code(AuthorizationCode::new(code))?
        .set_pkce_verifier(pkce_verifier)
        .request_async(http_client)
        .await?;

    let id_token = token_response.id_token().ok_or(Error::NoIdToken)?;
    let verifier = client.id_token_verifier();
    let claims = id_token.claims(&verifier, nonce)?;

    // Xác minh access token hash chống substitution attack
    if let Some(expected_hash) = claims.access_token_hash() {
        let actual_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            id_token.signing_alg()?,
            id_token.signing_key(&verifier)?,
        )?;
        assert_eq!(&actual_hash, expected_hash);
    }

    Ok(UserInfo {
        subject: claims.subject().to_string(),
        email: claims.email().map(|e| e.to_string()),
    })
}
```

> **Lưu ý bảo mật**: Luôn dùng PKCE dù là confidential client. Luôn verify CSRF token trước khi exchange code.

---

## 4. Svelte + Vite — Frontend

**Library ID**: `/llmstxt/svelte_dev_llms_txt`

### 4.1 Auth Store

Lưu session trong Svelte writable store, tất cả component subscribe:

```javascript
// src/stores/auth.js
import { writable, derived } from 'svelte/store';

export const user = writable(null); // { id, name, email, roles }
export const isAuthenticated = derived(user, $u => $u !== null);
export const isChecker = derived(user, $u => $u?.roles?.includes('CHECKER') ?? false);
export const isMaker = derived(user, $u => $u?.roles?.includes('MAKER') ?? false);
export const isAdmin = derived(user, $u => $u?.roles?.includes('ADMIN') ?? false);
```

### 4.2 API Module Pattern

Không gọi fetch trực tiếp trong component — phải qua `src/api/`:

```javascript
// src/api/index.js — base config
const BASE_URL = import.meta.env.VITE_API_URL || '/api';

export async function apiFetch(path, options = {}) {
    const token = localStorage.getItem('jwt_token');
    const response = await fetch(`${BASE_URL}${path}`, {
        ...options,
        headers: {
            'Content-Type': 'application/json',
            ...(token ? { Authorization: `Bearer ${token}` } : {}),
            ...options.headers,
        },
    });

    if (response.status === 401) {
        // Token hết hạn — redirect về login
        window.location.href = '/login';
        return;
    }

    if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(err.message || `HTTP ${response.status}`);
    }

    return response.json();
}

// src/api/requests.js
import { apiFetch } from './index.js';

export const createRequest = (data) =>
    apiFetch('/requests', { method: 'POST', body: JSON.stringify(data) });

export const getMyRequests = () => apiFetch('/requests/mine');

export const submitRequest = (id) =>
    apiFetch(`/requests/${id}/submit`, { method: 'POST' });
```

### 4.3 Protected Routes — Redirect nếu chưa login

Dùng `$app/navigation` và `$app/state` của SvelteKit:

```javascript
// Trong component hoặc load function
import { redirect } from '@sveltejs/kit';

export function load({ locals }) {
    if (!locals.user) {
        redirect(303, '/login');
    }
    return { user: locals.user };
}
```

### 4.4 WebSocket Client Store

```javascript
// src/stores/notifications.js
import { writable } from 'svelte/store';

export const notifications = writable([]);
export let ws = null;

export function connectWS(token) {
    ws = new WebSocket(`${import.meta.env.VITE_WS_URL}?token=${token}`);

    ws.onmessage = (event) => {
        const notif = JSON.parse(event.data);
        notifications.update(list => [notif, ...list].slice(0, 50));
    };

    ws.onclose = () => {
        // Reconnect sau 3 giây
        setTimeout(() => connectWS(token), 3000);
    };
}
```

---

## 5. Business Logic Patterns

### 5.1 4-Eyes Principle — Enforce trong DB và Application Layer

```rust
// db/requests.rs
async fn approve_request(
    pool: &PgPool,
    request_id: uuid::Uuid,
    checker_id: uuid::Uuid,
) -> Result<(), AppError> {
    // Query check maker != checker — enforce tại DB level
    let row = sqlx::query!(
        r#"
        SELECT created_by FROM sql_requests
        WHERE id = $1 AND status = 'PENDING'
        "#,
        request_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.created_by == checker_id {
        return Err(AppError::FourEyesViolation);
    }

    // Tiến hành approve...
    Ok(())
}
```

### 5.2 State Machine cho RequestStatus

```rust
// models/request.rs
#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[sqlx(type_name = "request_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequestStatus {
    Draft,
    Pending,
    Approved,
    Rejected,
    Executed,
    Failed,
    Cancelled,
}

impl RequestStatus {
    pub fn can_transition_to(&self, next: &RequestStatus) -> bool {
        use RequestStatus::*;
        matches!(
            (self, next),
            (Draft, Pending)
            | (Pending, Approved)
            | (Pending, Rejected)
            | (Pending, Cancelled)
            | (Approved, Executed)
            | (Approved, Failed)
        )
    }
}
```

### 5.3 Audit Log — Ghi log bắt buộc trước mọi thay đổi trạng thái

```rust
// db/audit.rs
pub async fn record_audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request_id: uuid::Uuid,
    action: &str,        // "SUBMITTED" | "APPROVED" | "REJECTED" | "EXECUTED" | "FAILED"
    actor_id: uuid::Uuid,
    detail: Option<&str>,
    rows_affected: Option<i64>,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"INSERT INTO audit_log
           (request_id, action, actor_id, detail, execution_result, timestamp)
           VALUES ($1, $2, $3, $4, $5, NOW())"#,
        request_id, action, actor_id, detail, rows_affected
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
```

> **Quy tắc**: Luôn gọi `record_audit` bên trong cùng transaction với thao tác chính — đảm bảo audit log không bao giờ bị mất nếu DB operation thất bại.

### 5.4 Encrypt Credentials Database Đích

```rust
// Dùng AES-256-GCM để encrypt credentials trước khi lưu PostgreSQL
// Thư viện: aes-gcm crate
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit, OsRng, rand_core::RngCore}};

fn encrypt_credential(plaintext: &str, key_bytes: &[u8; 32]) -> Result<String, Error> {
    let key = Key::<Aes256Gcm>::from_slice(key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())?;

    // Lưu dạng: base64(nonce) + "." + base64(ciphertext)
    Ok(format!(
        "{}.{}",
        base64::encode(nonce_bytes),
        base64::encode(ciphertext)
    ))
}
```

---

## 6. Tổng hợp Checklist Bảo mật & Nghiệp vụ

| Hạng mục | Biện pháp | Section |
|---|---|---|
| JWT | Max 8 giờ, verify signature mỗi request, validate `iss` | §9 |
| OIDC | PKCE + verify CSRF state + verify `at_hash` | §3 |
| Credentials DB đích | AES-256-GCM encrypt, không log, không trả về API | §5.4 |
| SQL execution | Wrap transaction, timeout + `TimeoutLayer`, rollback toàn bộ | §1.4, §2.3 |
| Audit log | Ghi trong cùng transaction, bất biến | §5.3, §13.2 |
| 4-eyes | Enforce tại application layer VÀ `CHECK CONSTRAINT` PostgreSQL | §5.1, §13.2 |
| DDL warning | `analyze_sql()` → flag khi Checker review | §12.2 |
| Input validation | `ValidatedJson` extractor tự động validate + reject 422 | §10 |
| Error handling | `AppError` không expose internal detail ra client | §8 |
| TLS | Tất cả kết nối database đích bắt buộc TLS | §12 |
| Notification | `broadcast::channel` — ignore `SendError` khi không có receiver | §11 |
| State machine | Enforce transition hợp lệ trước mọi thao tác | §5.2 |

---

## 7. Nguồn tham khảo

- Axum 0.7.9: https://docs.rs/axum/0.7.9/axum/
- SQLx (async Rust): https://docs.rs/sqlx/latest/sqlx/
- openidconnect-rs: https://docs.rs/openidconnect/
- jsonwebtoken (Rust): https://docs.rs/jsonwebtoken/
- validator (Rust): https://docs.rs/validator/
- thiserror: https://docs.rs/thiserror/
- Tokio 1.49: https://docs.rs/tokio/1.49.0/tokio/
- Svelte docs: https://svelte.dev/docs
- DaisyUI components: https://daisyui.com/components/
- Context7 documentation power (truy xuất ngày 2026-07-11)

---

## 8. Error Handling — thiserror + anyhow

**Library ID**: `/dtolnay/thiserror`

### 8.1 AppError enum với thiserror — tích hợp axum IntoResponse

Pattern chuẩn: dùng `thiserror` cho domain errors, impl `IntoResponse` để axum tự convert:

```rust
// src/error.rs
use axum::{response::{IntoResponse, Response}, http::StatusCode, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    // 400 — lỗi nghiệp vụ có thể trả về client
    #[error("4-eyes principle violated: maker cannot approve own request")]
    FourEyesViolation,

    #[error("invalid state transition: cannot move from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("immutable request: SQL content cannot be modified after submission")]
    ImmutableRequest,

    #[error("permission denied: user does not have access to database {db_id}")]
    DatabasePermissionDenied { db_id: uuid::Uuid },

    // 404
    #[error("resource not found")]
    NotFound,

    // 401 / 403
    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    // 500 — lỗi internal, wrap các error khác
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    // catch-all cho anyhow
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::FourEyesViolation
            | AppError::InvalidStateTransition { .. }
            | AppError::ImmutableRequest
            | AppError::DatabasePermissionDenied { .. } => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            _ => {
                // Log internal error, không expose chi tiết ra client
                tracing::error!("internal error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error".to_string())
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

// Shorthand cho handler return type
pub type Result<T> = std::result::Result<T, AppError>;
```

> **Quy tắc**: Lỗi internal (DB, JWT) → log + trả 500 mà không expose detail. Lỗi nghiệp vụ → trả 400/404 kèm message rõ ràng.

---

## 9. JWT — jsonwebtoken

**Library ID**: `/keats/jsonwebtoken`

### 9.1 Claims struct và encode/decode

```rust
// src/auth/jwt.rs
use jsonwebtoken::{encode, decode, Header, EncodingKey, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,         // user_id (UUID)
    pub email: String,       // email từ OIDC
    pub roles: Vec<String>,  // ["MAKER", "CHECKER"] — roles của user
    pub iss: String,         // issuer: "secs-backend"
    pub iat: usize,          // issued at (Unix timestamp)
    pub exp: usize,          // expiration (Unix timestamp, max 8 giờ)
}

pub fn generate_token(
    user_id: &str,
    email: &str,
    roles: Vec<String>,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        roles,
        iss: "secs-backend".to_string(),
        iat: now,
        exp: now + 8 * 3600, // 8 giờ tối đa — per security rules
    };

    encode(
        &Header::default(), // HS256
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&["secs-backend"]);
    // validate_exp = true by default — tự động reject token hết hạn

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(data.claims)
}
```

### 9.2 Tích hợp vào Axum middleware

```rust
// Trong auth middleware — trích từ Authorization header
async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = verify_token(token, &state.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
```

---

## 10. Input Validation — validator

**Library ID**: `/keats/validator`

### 10.1 Validate request body với derive macro

```rust
// src/models/request.rs
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateRequestBody {
    #[validate(length(min = 1, max = 10000, message = "SQL content must be 1–10000 characters"))]
    pub sql_content: String,

    #[validate(length(min = 1, message = "description is required"))]
    pub description: String,

    // UUID của DatabaseConnection
    pub database_id: uuid::Uuid,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RejectRequestBody {
    #[validate(length(min = 10, max = 2000, message = "rejection reason must be 10–2000 characters"))]
    pub reason: String,
}
```

### 10.2 Custom ValidatedJson extractor — tích hợp validator + axum

Tạo extractor bọc `Json` + tự động gọi `validate()`:

```rust
// src/api/extractors.rs
use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use validator::Validate;
use serde_json::json;

pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S>,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": e.to_string() }))).into_response()
            })?;

        value.validate().map_err(|errors| {
            (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({
                "error": "validation failed",
                "details": errors
            }))).into_response()
        })?;

        Ok(Self(value))
    }
}
```

**Cách dùng trong handler:**

```rust
async fn create_request(
    Extension(user): Extension<Claims>,
    ValidatedJson(body): ValidatedJson<CreateRequestBody>,
) -> Result<impl IntoResponse> {
    // body đã được validate, an toàn dùng
}
```

---

## 11. Tokio Broadcast — Notification Fanout

**Library ID**: `/websites/rs_tokio_1_49_0`

### 11.1 Broadcast channel cho realtime notifications

`tokio::sync::broadcast` là lựa chọn đúng khi cần gửi cùng một event đến **nhiều subscriber** (nhiều tab, nhiều Checker đang online):

```rust
// src/ws/mod.rs
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Notification {
    pub event: NotificationEvent,
    pub request_id: uuid::Uuid,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationEvent {
    RequestSubmitted,   // Maker gửi request → Checker nhận
    RequestApproved,    // Checker approve → Maker nhận
    RequestRejected,    // Checker reject → Maker nhận
    RequestExecuted,    // System thực thi xong → Maker + Checker nhận
    RequestFailed,      // Thực thi lỗi → Maker + Checker nhận
}

// Trong AppState — giữ sender, WS handler tự subscribe
pub struct AppState {
    pub db: sqlx::PgPool,
    pub jwt_secret: String,
    pub notification_tx: broadcast::Sender<Notification>,
}

impl AppState {
    pub fn new(db: sqlx::PgPool, jwt_secret: String) -> Self {
        let (notification_tx, _) = broadcast::channel(128);
        Self { db, jwt_secret, notification_tx }
    }
}
```

### 11.2 Gửi notification sau khi thay đổi trạng thái

```rust
// Trong db/requests.rs — sau khi approve thành công
pub async fn approve_request(
    pool: &PgPool,
    notification_tx: &broadcast::Sender<Notification>,
    request_id: uuid::Uuid,
    checker_id: uuid::Uuid,
) -> Result<()> {
    // ... transaction logic ...

    // Broadcast sau khi commit
    let _ = notification_tx.send(Notification {
        event: NotificationEvent::RequestApproved,
        request_id,
        message: "Your request has been approved".to_string(),
    });
    // send() trả Err nếu không có receiver — dùng let _ = ... để ignore (bình thường)

    Ok(())
}
```

> **Lưu ý**: `broadcast::channel` giữ message trong buffer với capacity đã cấu hình. Nếu receiver không đọc kịp và buffer đầy, receiver sẽ nhận `RecvError::Lagged` — cần xử lý gracefully trong WS handler.

---

## 12. DatabaseExecutor Trait — Multi-Database Abstraction

### 12.1 Trait design cho executor module

Mục tiêu: tất cả code gọi executor đều không phụ thuộc vào database type cụ thể.

```rust
// src/executor/mod.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionResult {
    pub rows_affected: u64,
    pub execution_time_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("execution timeout after {timeout_secs}s")]
    Timeout { timeout_secs: u64 },

    #[error("sql execution error: {0}")]
    SqlError(String),

    #[error("transaction rollback: {0}")]
    TransactionRollback(String),
}

#[async_trait]
pub trait DatabaseExecutor: Send + Sync {
    async fn execute(
        &self,
        sql: &str,
        timeout_secs: u64,
    ) -> Result<ExecutionResult, ExecutorError>;

    async fn test_connection(&self) -> Result<(), ExecutorError>;
}

// Factory function — tạo đúng executor theo db_type
pub fn create_executor(
    db_type: &str,
    connection_string: &str,
) -> Result<Box<dyn DatabaseExecutor>, ExecutorError> {
    match db_type {
        "postgresql" => Ok(Box::new(
            postgres::PostgresExecutor::new(connection_string)?
        )),
        "mysql" => Ok(Box::new(
            mysql::MySqlExecutor::new(connection_string)?
        )),
        "oracle" => Ok(Box::new(
            oracle::OracleExecutor::new(connection_string)?
        )),
        "mongodb" => Ok(Box::new(
            mongodb::MongoExecutor::new(connection_string)?
        )),
        _ => Err(ExecutorError::ConnectionFailed(
            format!("unsupported database type: {}", db_type)
        )),
    }
}
```

### 12.2 DDL Detection — Flag cảnh báo trước khi thực thi

Business rule #5: DDL phải được flag đặc biệt khi Checker review.

```rust
// src/executor/mod.rs
use regex::Regex;
use std::sync::OnceLock;

static DDL_PATTERN: OnceLock<Regex> = OnceLock::new();

fn ddl_regex() -> &'static Regex {
    DDL_PATTERN.get_or_init(|| {
        Regex::new(
            r"(?i)\b(DROP|TRUNCATE|ALTER|CREATE|RENAME|COMMENT ON)\b"
        ).unwrap()
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SqlAnalysis {
    pub contains_ddl: bool,
    pub ddl_keywords: Vec<String>,
    pub statement_count: usize,
    pub is_dml_only: bool,   // true nếu chỉ có INSERT/UPDATE/DELETE
}

pub fn analyze_sql(sql: &str) -> SqlAnalysis {
    let regex = ddl_regex();
    let ddl_keywords: Vec<String> = regex
        .find_iter(sql)
        .map(|m| m.as_str().to_uppercase())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Đếm số statements (tách bởi dấu ;)
    let statement_count = sql
        .split(';')
        .filter(|s| !s.trim().is_empty())
        .count();

    let dml_only_regex = Regex::new(
        r"(?i)^\s*(INSERT|UPDATE|DELETE|SELECT|WITH)\b"
    ).unwrap();

    let is_dml_only = sql
        .split(';')
        .filter(|s| !s.trim().is_empty())
        .all(|stmt| dml_only_regex.is_match(stmt.trim()));

    SqlAnalysis {
        contains_ddl: !ddl_keywords.is_empty(),
        ddl_keywords,
        statement_count,
        is_dml_only,
    }
}
```

**Dùng trong API tạo request:**

```rust
// Khi Maker tạo request — lưu kết quả analysis vào DB
let analysis = analyze_sql(&body.sql_content);

// Khi Checker review — trả thêm field warning
if analysis.contains_ddl {
    // Hiển thị badge "⚠️ DDL DETECTED" trong UI
    // Checker phải confirm thêm bước trước khi approve
}
```

---

## 13. Svelte UI Patterns — DaisyUI Components

### 13.1 StatusBadge component

```svelte
<!-- src/components/StatusBadge.svelte -->
<script>
    /** @type {'DRAFT'|'PENDING'|'APPROVED'|'REJECTED'|'EXECUTED'|'FAILED'|'CANCELLED'} */
    export let status;

    const config = {
        DRAFT:     { label: 'Draft',     class: 'badge-ghost' },
        PENDING:   { label: 'Pending',   class: 'badge-warning' },
        APPROVED:  { label: 'Approved',  class: 'badge-info' },
        REJECTED:  { label: 'Rejected',  class: 'badge-error' },
        EXECUTED:  { label: 'Executed',  class: 'badge-success' },
        FAILED:    { label: 'Failed',    class: 'badge-error' },
        CANCELLED: { label: 'Cancelled', class: 'badge-neutral' },
    };

    $: cfg = config[status] ?? { label: status, class: 'badge-ghost' };
</script>

<span class="badge {cfg.class}">{cfg.label}</span>
```

### 13.2 DDL Warning Banner

```svelte
<!-- Dùng trong ReviewDetail.svelte khi checker review -->
<script>
    export let analysis; // SqlAnalysis từ API
</script>

{#if analysis?.contains_ddl}
    <div role="alert" class="alert alert-warning">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 shrink-0 stroke-current" fill="none" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <div>
            <h3 class="font-bold">⚠️ DDL Statement Detected</h3>
            <p class="text-sm">
                This SQL contains DDL operations:
                {#each analysis.ddl_keywords as kw}
                    <span class="badge badge-warning badge-sm mr-1">{kw}</span>
                {/each}
            </p>
            <p class="text-sm mt-1">Please review carefully before approving.</p>
        </div>
    </div>
{/if}
```

### 13.3 Optimistic UI + Error Rollback pattern

Khi Maker submit request — cập nhật UI trước khi API trả về:

```svelte
<script>
    import { createRequest, submitRequest } from '../api/requests.js';

    let submitting = false;
    let error = null;

    async function handleSubmit() {
        submitting = true;
        error = null;
        try {
            const req = await createRequest(formData);
            await submitRequest(req.id);
            // Navigate về danh sách
            goto('/maker/requests');
        } catch (e) {
            error = e.message;
        } finally {
            submitting = false;
        }
    }
</script>

<button
    class="btn btn-primary"
    disabled={submitting}
    on:click={handleSubmit}
>
    {#if submitting}
        <span class="loading loading-spinner loading-sm"></span>
        Submitting...
    {:else}
        Submit for Review
    {/if}
</button>

{#if error}
    <div role="alert" class="alert alert-error mt-2">
        <span>{error}</span>
    </div>
{/if}
```

---

## 14. Database Schema — PostgreSQL Migration Pattern

### 14.1 Thứ tự migration files

```
migrations/
├── 001_create_users.sql
├── 002_create_database_connections.sql
├── 003_create_sql_requests.sql
├── 004_create_audit_log.sql
├── 005_create_user_db_permissions.sql
└── 006_create_indexes.sql
```

### 14.2 Core tables design

```sql
-- migrations/001_create_users.sql
CREATE TYPE user_role AS ENUM ('ADMIN', 'MAKER', 'CHECKER');

CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    oidc_sub    TEXT UNIQUE NOT NULL,   -- subject claim từ OIDC
    email       TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    roles       user_role[] NOT NULL DEFAULT '{}',
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- migrations/002_create_database_connections.sql
CREATE TYPE db_type AS ENUM ('postgresql', 'mysql', 'oracle', 'mongodb');

CREATE TABLE database_connections (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    db_type         db_type NOT NULL,
    host            TEXT NOT NULL,
    port            INTEGER NOT NULL,
    database_name   TEXT NOT NULL,
    schema_name     TEXT,
    -- credentials được encrypt bằng AES-256-GCM trước khi lưu
    encrypted_credentials TEXT NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- migrations/003_create_sql_requests.sql
CREATE TYPE request_status AS ENUM (
    'DRAFT', 'PENDING', 'APPROVED', 'REJECTED',
    'EXECUTED', 'FAILED', 'CANCELLED'
);

CREATE TABLE sql_requests (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sql_content     TEXT NOT NULL,
    description     TEXT NOT NULL,
    contains_ddl    BOOLEAN NOT NULL DEFAULT FALSE,
    ddl_keywords    TEXT[],
    status          request_status NOT NULL DEFAULT 'DRAFT',
    database_id     UUID NOT NULL REFERENCES database_connections(id),
    created_by      UUID NOT NULL REFERENCES users(id),
    approved_by     UUID REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Enforce 4-eyes: approved_by != created_by (DB level constraint)
    CONSTRAINT four_eyes_check CHECK (approved_by IS NULL OR approved_by != created_by)
);

-- migrations/004_create_audit_log.sql
CREATE TYPE audit_action AS ENUM (
    'CREATED', 'SUBMITTED', 'APPROVED', 'REJECTED',
    'EXECUTED', 'FAILED', 'CANCELLED'
);

CREATE TABLE audit_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id      UUID NOT NULL REFERENCES sql_requests(id),
    action          audit_action NOT NULL,
    actor_id        UUID NOT NULL REFERENCES users(id),
    detail          TEXT,                   -- lý do reject, error message
    execution_result BIGINT,               -- rows affected
    timestamp       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- migrations/005_create_user_db_permissions.sql
CREATE TABLE user_db_permissions (
    user_id     UUID NOT NULL REFERENCES users(id),
    database_id UUID NOT NULL REFERENCES database_connections(id),
    granted_by  UUID NOT NULL REFERENCES users(id),
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, database_id)
);

-- migrations/006_create_indexes.sql
CREATE INDEX idx_sql_requests_created_by ON sql_requests(created_by);
CREATE INDEX idx_sql_requests_status ON sql_requests(status);
CREATE INDEX idx_sql_requests_database_id ON sql_requests(database_id);
CREATE INDEX idx_audit_log_request_id ON audit_log(request_id);
CREATE INDEX idx_audit_log_actor_id ON audit_log(actor_id);
```

> **Điểm quan trọng**: `CONSTRAINT four_eyes_check` enforce 4-eyes ngay tại DB level — dù application có bug vẫn không thể vi phạm.
