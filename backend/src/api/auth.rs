use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::auth::extractors::RequireAuth;
use crate::auth::jwt::generate_token;
use crate::auth::oidc::{build_auth_url, handle_oidc_callback, OidcSession};
use crate::db;
use crate::error::AppError;
use crate::state::AppState;

/// OIDC session with creation timestamp for TTL enforcement.
pub struct TimestampedSession {
    pub session: OidcSession,
    pub created_at: Instant,
}

/// In-memory session store cho OIDC flow.
/// Key = CSRF state string, Value = TimestampedSession (session + created_at).
///
/// Production nên dùng Redis — nhưng cho single-instance app nội bộ, in-memory đủ dùng.
pub type OidcSessionStore = Arc<RwLock<HashMap<String, TimestampedSession>>>;

/// Tạo session store mới.
pub fn new_session_store() -> OidcSessionStore {
    Arc::new(RwLock::new(HashMap::new()))
}

// ── Request/Response DTOs ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub roles: Vec<String>,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /auth/login` — Redirect user đến OIDC provider để đăng nhập.
///
/// Flow:
/// 1. Tạo authorization URL với PKCE challenge
/// 2. Lưu session (csrf, nonce, pkce_verifier) vào store
/// 3. Redirect browser đến OIDC provider
pub async fn login(State(state): State<AppState>) -> Result<Response, AppError> {
    let oidc_client = state
        .oidc_client
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("OIDC client not initialized")))?;

    let (auth_url, session) = build_auth_url(oidc_client);

    // Lưu session — key = csrf state secret
    let csrf_key = session.csrf_token.secret().to_string();
    state
        .oidc_sessions
        .write()
        .await
        .insert(csrf_key, TimestampedSession {
            session,
            created_at: Instant::now(),
        });

    Ok(Redirect::temporary(auth_url.as_str()).into_response())
}

/// `GET /auth/callback` — OIDC provider redirect về đây sau khi user đăng nhập.
///
/// Flow:
/// 1. Lấy session từ store bằng state parameter
/// 2. Exchange code lấy ID token
/// 3. Upsert user vào DB
/// 4. Generate JWT
/// 5. Set httpOnly cookie + redirect về frontend
pub async fn callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, AppError> {
    let oidc_client = state
        .oidc_client
        .as_ref()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("OIDC client not initialized")))?;

    // 1. Lấy và xóa session từ store
    let timestamped = state
        .oidc_sessions
        .write()
        .await
        .remove(&query.state)
        .ok_or(AppError::Unauthorized)?;
    let session = timestamped.session;

    // 2. Exchange code, verify claims
    let user_info = handle_oidc_callback(oidc_client, query.code, &query.state, session)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "OIDC callback failed");
            AppError::Unauthorized
        })?;

    // 3. Upsert user vào DB
    let user = db::users::upsert_from_oidc(
        &state.db,
        &user_info.subject,
        &user_info.email,
        &user_info.display_name,
    )
    .await
    .map_err(AppError::Database)?;

    // Kiểm tra user active
    if !user.is_active {
        return Err(AppError::Forbidden);
    }

    // 4. Lấy roles
    let roles = db::users::get_user_roles(&state.db, user.id)
        .await
        .map_err(AppError::Database)?;

    // 5. Generate JWT
    let token = generate_token(
        user.id,
        &user.email,
        &user.display_name,
        roles,
        &state.config.jwt_secret,
    )?;

    // 6. Set httpOnly cookie + redirect về frontend
    let is_secure = state.config.frontend_url.starts_with("https");
    let cookie = crate::auth::cookie::build_auth_cookie(&token, is_secure);

    let redirect_url = format!("{}/#/auth/callback", state.config.frontend_url);
    let mut response = Redirect::temporary(&redirect_url).into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie,
    );

    Ok(response)
}

/// `GET /auth/me` — Trả về thông tin user hiện tại từ JWT claims.
///
/// Yêu cầu: Bearer token hợp lệ.
pub async fn me(RequireAuth(claims): RequireAuth) -> Json<UserInfo> {
    Json(UserInfo {
        id: claims.user_id.to_string(),
        email: claims.email,
        display_name: claims.display_name,
        roles: claims.roles.iter().map(|r| r.to_string()).collect(),
    })
}

/// `POST /auth/logout` — Revoke token hiện tại và xóa httpOnly cookie.
///
/// Token được thêm vào blacklist với TTL = remaining lifetime.
/// Cookie được clear bằng Set-Cookie Max-Age=0.
pub async fn logout(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
) -> Response {
    // Tính remaining lifetime của token
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs() as usize;

    let remaining_secs = claims.exp.saturating_sub(now);
    if remaining_secs > 0 {
        let token_key = format!("{}:{}", claims.sub, claims.iat);
        state
            .token_blacklist
            .revoke_token(&token_key, std::time::Duration::from_secs(remaining_secs as u64))
            .await;
    }

    // Clear cookie
    let is_secure = state.config.frontend_url.starts_with("https");
    let clear_cookie = crate::auth::cookie::build_clear_cookie(is_secure);

    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        clear_cookie,
    );

    response
}


/// Background task that periodically removes expired OIDC sessions.
pub async fn oidc_session_cleanup(
    store: OidcSessionStore,
    ttl: std::time::Duration,
    interval: std::time::Duration,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let now = Instant::now();
        let mut sessions = store.write().await;
        let before = sessions.len();
        sessions.retain(|_, v| now.duration_since(v.created_at) < ttl);
        let removed = before - sessions.len();
        if removed > 0 {
            tracing::debug!(removed, remaining = sessions.len(), "OIDC session cleanup");
        }
    }
}
