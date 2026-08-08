use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

use crate::auth::cookie::extract_token_from_cookie;
use crate::auth::jwt::verify_token;
use crate::error::AppError;
use crate::state::AppState;

/// Auth middleware: extract JWT từ Cookie (httpOnly) HOẶC Authorization Bearer header.
/// Verify JWT, check blacklist, inject `Claims` vào request extensions.
///
/// Priority:
/// 1. Authorization: Bearer <token> (cho API clients, backward compat)
/// 2. Cookie: secs_token=<token> (cho browser — httpOnly, XSS-safe)
///
/// Áp dụng cho tất cả protected routes.
/// Không áp dụng cho: /health, /auth/login, /auth/callback
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_token(&request)?;

    let claims = verify_token(&token, &state.config.jwt_secret)?;

    // Check token blacklist — reject revoked tokens
    if state
        .token_blacklist
        .is_revoked(claims.user_id, &claims.sub, claims.iat)
        .await
    {
        return Err(AppError::Unauthorized);
    }

    // Inject claims vào request extensions để handler/extractor lấy ra
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Extract JWT token — thử Bearer header trước, fallback sang Cookie.
fn extract_token(request: &Request) -> Result<String, AppError> {
    // Priority 1: Authorization: Bearer <token>
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(val) = auth_header.to_str() {
            if let Some(token) = val.strip_prefix("Bearer ") {
                if !token.is_empty() {
                    return Ok(token.to_string());
                }
            }
        }
    }

    // Priority 2: Cookie: secs_token=<token>
    if let Some(cookie_header) = request.headers().get(header::COOKIE) {
        if let Ok(val) = cookie_header.to_str() {
            if let Some(token) = extract_token_from_cookie(val) {
                return Ok(token.to_string());
            }
        }
    }

    Err(AppError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn make_request_with_auth_header(header_value: Option<&str>) -> Request {
        let mut request = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        if let Some(val) = header_value {
            request
                .headers_mut()
                .insert(header::AUTHORIZATION, HeaderValue::from_str(val).unwrap());
        }

        request
    }

    fn make_request_with_cookie(cookie_value: &str) -> Request {
        let mut request = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        request
            .headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(cookie_value).unwrap());

        request
    }

    #[test]
    fn test_extract_token_from_bearer() {
        let request = make_request_with_auth_header(Some("Bearer abc123"));
        let token = extract_token(&request).unwrap();
        assert_eq!(token, "abc123");
    }

    #[test]
    fn test_extract_token_missing_both() {
        let request = make_request_with_auth_header(None);
        let err = extract_token(&request).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[test]
    fn test_extract_token_wrong_scheme() {
        let request = make_request_with_auth_header(Some("Basic abc123"));
        // No Bearer, no Cookie → Unauthorized
        let err = extract_token(&request).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[test]
    fn test_extract_token_empty_bearer() {
        let request = make_request_with_auth_header(Some("Bearer "));
        // Empty Bearer token, no Cookie → Unauthorized
        let err = extract_token(&request).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[test]
    fn test_extract_token_from_cookie() {
        let request = make_request_with_cookie("secs_token=eyJtoken123; other=val");
        let token = extract_token(&request).unwrap();
        assert_eq!(token, "eyJtoken123");
    }

    #[test]
    fn test_bearer_takes_precedence_over_cookie() {
        let mut request = Request::builder()
            .uri("/test")
            .body(axum::body::Body::empty())
            .unwrap();

        request.headers_mut().insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer from_header"),
        );
        request.headers_mut().insert(
            header::COOKIE,
            HeaderValue::from_static("secs_token=from_cookie"),
        );

        let token = extract_token(&request).unwrap();
        assert_eq!(token, "from_header");
    }

    #[test]
    fn test_cookie_fallback_when_no_bearer() {
        let request = make_request_with_cookie("secs_token=fallback_token");
        let token = extract_token(&request).unwrap();
        assert_eq!(token, "fallback_token");
    }
}
