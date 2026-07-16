use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use std::net::SocketAddr;

/// Maximum length for stored IP address (supports IPv6).
const MAX_IP_LENGTH: usize = 45;

/// Extract client IP with precedence:
/// 1. X-Forwarded-For (first IP in comma-separated list)
/// 2. X-Real-IP
/// 3. Direct connection address
/// 4. Fallback: "unknown"
///
/// The result is truncated at 45 characters to fit the `ip_address` column.
pub fn extract_client_ip(headers: &HeaderMap, conn_info: &ConnectInfo<SocketAddr>) -> String {
    let ip = extract_from_headers(headers)
        .unwrap_or_else(|| conn_info.0.ip().to_string());

    truncate_ip(&ip)
}

/// Try to extract IP from headers in priority order.
fn extract_from_headers(headers: &HeaderMap) -> Option<String> {
    // Priority 1: X-Forwarded-For — take the first IP in the comma-separated list
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(value) = forwarded_for.to_str() {
            let first_ip = value.split(',').next().unwrap_or("").trim();
            if !first_ip.is_empty() {
                return Some(first_ip.to_string());
            }
        }
    }

    // Priority 2: X-Real-IP
    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(value) = real_ip.to_str() {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

/// Truncate IP string to MAX_IP_LENGTH characters.
/// If extraction yields empty or whitespace-only, return "unknown".
fn truncate_ip(ip: &str) -> String {
    let trimmed = ip.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }

    if trimmed.len() > MAX_IP_LENGTH {
        trimmed[..MAX_IP_LENGTH].to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::net::SocketAddr;

    fn make_conn_info(ip: &str, port: u16) -> ConnectInfo<SocketAddr> {
        let addr: SocketAddr = format!("{ip}:{port}").parse().unwrap();
        ConnectInfo(addr)
    }

    #[test]
    fn test_x_forwarded_for_first_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.0.0.1, 10.0.0.2, 10.0.0.3".parse().unwrap());
        let conn = make_conn_info("127.0.0.1", 8080);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result, "10.0.0.1");
    }

    #[test]
    fn test_x_forwarded_for_single_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "192.168.1.100".parse().unwrap());
        let conn = make_conn_info("127.0.0.1", 8080);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result, "192.168.1.100");
    }

    #[test]
    fn test_x_real_ip_when_no_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "172.16.0.50".parse().unwrap());
        let conn = make_conn_info("127.0.0.1", 8080);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result, "172.16.0.50");
    }

    #[test]
    fn test_x_forwarded_for_takes_precedence_over_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "10.1.1.1".parse().unwrap());
        headers.insert("x-real-ip", "10.2.2.2".parse().unwrap());
        let conn = make_conn_info("127.0.0.1", 8080);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result, "10.1.1.1");
    }

    #[test]
    fn test_fallback_to_conn_info() {
        let headers = HeaderMap::new();
        let conn = make_conn_info("192.168.0.1", 9090);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result, "192.168.0.1");
    }

    #[test]
    fn test_truncate_at_45_chars() {
        let mut headers = HeaderMap::new();
        // Create an IP-like string longer than 45 characters
        let long_ip = "a".repeat(60);
        headers.insert("x-forwarded-for", long_ip.parse().unwrap());
        let conn = make_conn_info("127.0.0.1", 8080);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result.len(), 45);
    }

    #[test]
    fn test_whitespace_trimming() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "  10.0.0.1 , 10.0.0.2 ".parse().unwrap());
        let conn = make_conn_info("127.0.0.1", 8080);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result, "10.0.0.1");
    }

    #[test]
    fn test_empty_forwarded_for_falls_through() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "".parse().unwrap());
        headers.insert("x-real-ip", "10.5.5.5".parse().unwrap());
        let conn = make_conn_info("127.0.0.1", 8080);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result, "10.5.5.5");
    }

    #[test]
    fn test_ipv6_address() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "2001:db8::1".parse().unwrap());
        let conn = make_conn_info("127.0.0.1", 8080);

        let result = extract_client_ip(&headers, &conn);
        assert_eq!(result, "2001:db8::1");
    }
}
