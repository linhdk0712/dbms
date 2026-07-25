pub mod analysis;
pub mod approvals;
pub mod audit;
pub mod auth;
pub mod change_requests;
pub mod dashboard;
pub mod databases;
pub mod health;
pub mod ip_extractor;
pub mod requests;
pub mod users;

use axum::{
    http::{header, HeaderValue, Method},
    middleware as axum_mw,
    response::Response,
    routing::{get, post, patch, delete},
    Router,
};
use tower_http::{
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::auth::middleware::require_auth;
use crate::auth::rate_limit::{rate_limit_login, RateLimiter};
use crate::state::AppState;
use crate::ws;

/// Giới hạn kích thước request body: 100KB.
const MAX_BODY_SIZE: usize = 100 * 1024;

/// Xây dựng axum Router với đầy đủ middleware stack.
pub fn build_router(state: AppState) -> Router {
    let cors = build_cors(&state.config.frontend_url);
    let rate_limiter = RateLimiter::new();

    // Public routes — không cần auth
    let public_routes = Router::new()
        .route("/health", get(health::health_check))
        .route("/auth/login", get(auth::login).layer(axum_mw::from_fn(rate_limit_login)))
        .route("/auth/callback", get(auth::callback))
        .route("/ws", get(ws::ws_handler))
        .layer(axum::Extension(rate_limiter));

    // Protected routes — cần Bearer token hợp lệ
    let protected_routes = Router::new()
        // Auth
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        // Dashboard
        .route("/api/v1/dashboard", get(dashboard::get_stats))
        // Requests (Maker)
        .route("/api/v1/requests", post(requests::create_request))
        .route("/api/v1/requests", get(requests::list_my_requests))
        .route("/api/v1/requests/:id", get(requests::get_request_detail))
        .route("/api/v1/requests/:id", patch(requests::update_request))
        .route("/api/v1/requests/:id/submit", post(requests::submit_request))
        .route("/api/v1/requests/:id/cancel", post(requests::cancel_request))
        .route("/api/v1/requests/:id/clone", post(requests::clone_request))
        // AI Analysis
        .route("/api/v1/requests/:id/analyze", post(analysis::analyze_request))
        // Audit
        .route("/api/v1/requests/:id/audit", get(audit::get_request_audit))
        // Approvals (Checker)
        .route("/api/v1/approvals/pending", get(approvals::list_pending))
        .route("/api/v1/approvals/:id", get(approvals::get_detail))
        .route("/api/v1/approvals/:id/validate", post(approvals::validate))
        .route("/api/v1/approvals/:id/approve", post(approvals::approve))
        .route("/api/v1/approvals/:id/reject", post(approvals::reject))
        // Databases (Admin)
        .route("/api/v1/databases/my-permissions", get(databases::list_my_databases))
        .route("/api/v1/databases/test-new", post(databases::test_new_connection))
        .route("/api/v1/databases", post(databases::create_database))
        .route("/api/v1/databases", get(databases::list_databases))
        .route("/api/v1/databases/:id", get(databases::get_database))
        .route("/api/v1/databases/:id", patch(databases::update_database))
        .route("/api/v1/databases/:id/test", post(databases::test_connection))
        .route("/api/v1/databases/:id/credentials", patch(databases::update_credentials))
        // Change Requests (CR)
        .route("/api/v1/change-requests", post(change_requests::create_cr))
        .route("/api/v1/change-requests", get(change_requests::list_crs))
        .route("/api/v1/change-requests/:id", get(change_requests::get_cr_detail))
        .route("/api/v1/change-requests/:id/sync", post(change_requests::sync_cr))
        .route("/api/v1/change-requests/:id", delete(change_requests::delete_cr))
        // Users (Admin)
        .route("/api/v1/users", get(users::list_users))
        .route("/api/v1/users/:user_id", get(users::get_user))
        .route("/api/v1/users/:user_id", patch(users::update_user))
        .route("/api/v1/users/:user_id/permissions", get(users::get_user_permissions))
        .route("/api/v1/users/:user_id/permissions", post(users::grant_permission))
        .route("/api/v1/users/:user_id/permissions/:database_id", delete(users::revoke_permission))
        .layer(axum_mw::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        // ── Middleware stack (áp dụng từ dưới lên) ──
        .layer(axum_mw::from_fn(security_headers))
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// CORS policy — chỉ cho phép origin của frontend.
fn build_cors(frontend_url: &str) -> CorsLayer {
    let origin = frontend_url
        .parse::<HeaderValue>()
        .expect("FRONTEND_URL must be a valid origin");

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true)
}

/// Middleware inject HTTP security headers vào mọi response (SECURITY-04).
async fn security_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'"),
    );
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    response
}
