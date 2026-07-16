use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

use crate::auth::jwt::verify_token;
use crate::error::AppError;
use crate::state::AppState;

/// Auth middleware: extract Bearer token từ Authorization header,
/// verify JWT, inject `Claims` vào request extensions.
///
/// Áp dụng cho tất cả protected routes.
/// Không áp dụng cho: /health, /auth/login, /auth/callback
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer_token(&request)?;

    let claims = verify_token(&token, &state.config.jwt_secret)?;

    // Inject claims vào request extensions để handler/extractor lấy ra
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Extract Bearer token từ Authorization header.
/// Format: "Bearer <token>"
fn extract_bearer_token(request: &Request) -> Result<String, AppError> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    if token.is_empty() {
        return Err(AppError::Unauthorized);
    }

    Ok(token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn make_request_with_header(header_value: Option<&str>) -> Request {
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

    #[test]
    fn test_extract_bearer_valid() {
        let request = make_request_with_header(Some("Bearer abc123"));
        let token = extract_bearer_token(&request).unwrap();
        assert_eq!(token, "abc123");
    }

    #[test]
    fn test_extract_bearer_missing_header() {
        let request = make_request_with_header(None);
        let err = extract_bearer_token(&request).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[test]
    fn test_extract_bearer_wrong_scheme() {
        let request = make_request_with_header(Some("Basic abc123"));
        let err = extract_bearer_token(&request).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[test]
    fn test_extract_bearer_empty_token() {
        let request = make_request_with_header(Some("Bearer "));
        let err = extract_bearer_token(&request).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }
}
