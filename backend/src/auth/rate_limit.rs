//! Rate Limiter — bảo vệ endpoint login khỏi brute-force.
//!
//! Dùng sliding window counter per IP.
//! Không cần dependency ngoài — dùng DashMap-like pattern với tokio::sync::Mutex.
//!
//! Design:
//! - Max 10 requests per minute per IP trên /auth/login
//! - Trả 429 Too Many Requests nếu vượt limit
//! - Tự động cleanup entries cũ

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Cấu hình rate limit.
const MAX_REQUESTS: u32 = 10;
const WINDOW_DURATION: Duration = Duration::from_secs(60);
const CLEANUP_THRESHOLD: usize = 1000;

/// Lưu trữ rate limit state — chia sẻ giữa các requests.
#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Kiểm tra và ghi nhận request. Trả `true` nếu được phép, `false` nếu bị limit.
    pub async fn check_and_record(&self, ip: IpAddr) -> bool {
        let mut map = self.inner.lock().await;
        let now = Instant::now();

        // Cleanup entries cũ nếu map quá lớn
        if map.len() > CLEANUP_THRESHOLD {
            map.retain(|_, timestamps| {
                timestamps.retain(|t| now.duration_since(*t) < WINDOW_DURATION);
                !timestamps.is_empty()
            });
        }

        let timestamps = map.entry(ip).or_default();

        // Loại bỏ timestamps ngoài window
        timestamps.retain(|t| now.duration_since(*t) < WINDOW_DURATION);

        if timestamps.len() >= MAX_REQUESTS as usize {
            return false; // Rate limited
        }

        timestamps.push(now);
        true
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware rate limit cho login endpoint.
/// Áp dụng trên route `/auth/login` only.
pub async fn rate_limit_login(
    request: Request,
    next: Next,
) -> Response {
    // Extract IP từ request extensions (ConnectInfo) hoặc headers
    let ip = extract_client_ip(&request);

    // Lấy rate limiter từ request extensions
    let limiter = request
        .extensions()
        .get::<RateLimiter>()
        .cloned();

    if let Some(limiter) = limiter {
        if !limiter.check_and_record(ip).await {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "Too many login attempts. Please try again later.",
                    "retry_after_secs": WINDOW_DURATION.as_secs()
                })),
            )
                .into_response();
        }
    }

    next.run(request).await
}

/// Extract client IP — ưu tiên X-Forwarded-For (khi sau reverse proxy), fallback ConnectInfo.
fn extract_client_ip(request: &Request) -> IpAddr {
    // Thử X-Forwarded-For header trước (khi đứng sau proxy)
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            if let Some(first_ip) = val.split(',').next() {
                if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                    return ip;
                }
            }
        }
    }

    // Fallback: X-Real-IP
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(val) = real_ip.to_str() {
            if let Ok(ip) = val.trim().parse::<IpAddr>() {
                return ip;
            }
        }
    }

    // Fallback: ConnectInfo (direct connection)
    if let Some(connect_info) = request.extensions().get::<ConnectInfo<std::net::SocketAddr>>() {
        return connect_info.0.ip();
    }

    // Final fallback — dùng loopback
    IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allows_under_limit() {
        let limiter = RateLimiter::new();
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        for _ in 0..MAX_REQUESTS {
            assert!(limiter.check_and_record(ip).await);
        }
    }

    #[tokio::test]
    async fn test_blocks_over_limit() {
        let limiter = RateLimiter::new();
        let ip: IpAddr = "192.168.1.2".parse().unwrap();

        for _ in 0..MAX_REQUESTS {
            limiter.check_and_record(ip).await;
        }

        // Request thứ 11 phải bị block
        assert!(!limiter.check_and_record(ip).await);
    }

    #[tokio::test]
    async fn test_different_ips_independent() {
        let limiter = RateLimiter::new();
        let ip1: IpAddr = "10.0.0.1".parse().unwrap();
        let ip2: IpAddr = "10.0.0.2".parse().unwrap();

        for _ in 0..MAX_REQUESTS {
            limiter.check_and_record(ip1).await;
        }

        // IP khác vẫn được phép
        assert!(limiter.check_and_record(ip2).await);
    }
}
