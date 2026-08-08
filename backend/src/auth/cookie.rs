//! Cookie utilities cho JWT token.
//!
//! Security properties:
//! - HttpOnly: JavaScript không đọc được → chống XSS token theft
//! - Secure: chỉ gửi qua HTTPS → chống man-in-the-middle
//! - SameSite=Lax: chống CSRF cho GET, POST cross-origin bị block
//! - Path=/: cookie áp dụng cho tất cả routes
//! - Max-Age: khớp với JWT expiry (8 giờ)

use axum::http::HeaderValue;

/// Tên cookie chứa JWT token.
pub const AUTH_COOKIE_NAME: &str = "secs_token";

/// Max-Age cho cookie: 8 giờ (khớp JWT expiry).
const COOKIE_MAX_AGE_SECS: u64 = 8 * 60 * 60;

/// Tạo Set-Cookie header value cho JWT token.
///
/// Cookie attributes:
/// - `HttpOnly` — JS không đọc được
/// - `Secure` — chỉ gửi qua HTTPS (skip khi dev nếu cần)
/// - `SameSite=Lax` — chống CSRF nhưng cho phép top-level navigation
/// - `Path=/` — áp dụng cho mọi route
/// - `Max-Age=28800` — 8 giờ
///
/// # Parameters
/// - `token` — JWT token string
/// - `is_secure` — true nếu production (HTTPS), false nếu dev (HTTP localhost)
pub fn build_auth_cookie(token: &str, is_secure: bool) -> HeaderValue {
    let secure_flag = if is_secure { "; Secure" } else { "" };
    let cookie_value = format!(
        "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}{}",
        AUTH_COOKIE_NAME, token, COOKIE_MAX_AGE_SECS, secure_flag
    );
    HeaderValue::from_str(&cookie_value).expect("cookie value must be valid ASCII")
}

/// Tạo Set-Cookie header để xóa cookie (expire ngay lập tức).
///
/// Dùng khi logout — set Max-Age=0 và value rỗng.
pub fn build_clear_cookie(is_secure: bool) -> HeaderValue {
    let secure_flag = if is_secure { "; Secure" } else { "" };
    let cookie_value = format!(
        "{}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}",
        AUTH_COOKIE_NAME, secure_flag
    );
    HeaderValue::from_str(&cookie_value).expect("cookie value must be valid ASCII")
}

/// Extract JWT token từ Cookie header.
///
/// Parse raw cookie string (ví dụ: "secs_token=eyJ...; other_cookie=xyz")
/// và trả về value của `secs_token`.
pub fn extract_token_from_cookie(cookie_header: &str) -> Option<&str> {
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Some(value) = trimmed.strip_prefix(AUTH_COOKIE_NAME) {
            // Phải có dấu = ngay sau tên cookie
            if let Some(token) = value.strip_prefix('=') {
                if !token.is_empty() {
                    return Some(token);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_auth_cookie_secure() {
        let cookie = build_auth_cookie("eyJtoken", true);
        let s = cookie.to_str().unwrap();
        assert!(s.contains("secs_token=eyJtoken"));
        assert!(s.contains("HttpOnly"));
        assert!(s.contains("Secure"));
        assert!(s.contains("SameSite=Lax"));
        assert!(s.contains("Path=/"));
        assert!(s.contains("Max-Age=28800"));
    }

    #[test]
    fn test_build_auth_cookie_insecure() {
        let cookie = build_auth_cookie("eyJtoken", false);
        let s = cookie.to_str().unwrap();
        assert!(s.contains("secs_token=eyJtoken"));
        assert!(s.contains("HttpOnly"));
        assert!(!s.contains("Secure"));
        assert!(s.contains("SameSite=Lax"));
    }

    #[test]
    fn test_build_clear_cookie() {
        let cookie = build_clear_cookie(true);
        let s = cookie.to_str().unwrap();
        assert!(s.contains("secs_token=;"));
        assert!(s.contains("Max-Age=0"));
        assert!(s.contains("HttpOnly"));
    }

    #[test]
    fn test_extract_token_from_cookie_single() {
        let header = "secs_token=eyJ12345";
        assert_eq!(extract_token_from_cookie(header), Some("eyJ12345"));
    }

    #[test]
    fn test_extract_token_from_cookie_multiple() {
        let header = "other=value; secs_token=eyJabc; another=xyz";
        assert_eq!(extract_token_from_cookie(header), Some("eyJabc"));
    }

    #[test]
    fn test_extract_token_from_cookie_missing() {
        let header = "other=value; different=abc";
        assert_eq!(extract_token_from_cookie(header), None);
    }

    #[test]
    fn test_extract_token_from_cookie_empty_value() {
        let header = "secs_token=";
        assert_eq!(extract_token_from_cookie(header), None);
    }

    #[test]
    fn test_extract_token_similar_name_no_match() {
        let header = "secs_token_old=eyJold; secs_token=eyJnew";
        // Should match secs_token, not secs_token_old
        assert_eq!(extract_token_from_cookie(header), Some("eyJnew"));
    }
}
