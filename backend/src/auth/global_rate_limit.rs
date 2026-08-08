//! Global Rate Limiter — giới hạn tổng số request per user trên protected endpoints.
//!
//! Sliding window counter per user_id (từ JWT claims).
//! Mặc định: 200 requests per minute per user.
//! Áp dụng sau auth middleware (cần claims đã được inject).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use uuid::Uuid;

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::auth::jwt::Claims;

/// Số request tối đa per user per window.
const MAX_REQUESTS_PER_USER: u32 = 200;
/// Thời gian cửa sổ sliding window.
const WINDOW_DURATION: Duration = Duration::from_secs(60);
/// Threshold để trigger cleanup entries cũ.
const CLEANUP_THRESHOLD: usize = 5000;

/// Global rate limiter state — chia sẻ giữa requests.
#[derive(Clone)]
pub struct GlobalRateLimiter {
    inner: Arc<Mutex<HashMap<Uuid, Vec<Instant>>>>,
}

impl GlobalRateLimiter {
    /// Tạo một global rate limiter mới với state rỗng.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Kiểm tra và ghi nhận request. Trả `true` nếu cho phép.
    pub async fn check_and_record(&self, user_id: Uuid) -> bool {
        let mut map = self.inner.lock().await;
        let now = Instant::now();

        // Cleanup nếu map quá lớn
        if map.len() > CLEANUP_THRESHOLD {
            map.retain(|_, timestamps| {
                timestamps.retain(|t| now.duration_since(*t) < WINDOW_DURATION);
                !timestamps.is_empty()
            });
        }

        let timestamps = map.entry(user_id).or_default();

        // Loại bỏ timestamps ngoài window
        timestamps.retain(|t| now.duration_since(*t) < WINDOW_DURATION);

        if timestamps.len() >= MAX_REQUESTS_PER_USER as usize {
            return false;
        }

        timestamps.push(now);
        true
    }
}

impl Default for GlobalRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware global rate limit — áp dụng cho tất cả protected routes.
/// Yêu cầu auth middleware đã chạy trước (Claims phải có trong extensions).
///
/// Nếu user chưa authenticated (chưa có Claims) → bỏ qua (để auth middleware handle).
pub async fn global_rate_limit(
    request: Request,
    next: Next,
) -> Response {
    // Lấy user_id từ Claims (đã inject bởi auth middleware)
    let user_id = request
        .extensions()
        .get::<Claims>()
        .map(|c| c.user_id);

    if let Some(user_id) = user_id {
        // Lấy rate limiter từ extensions
        if let Some(limiter) = request.extensions().get::<GlobalRateLimiter>().cloned() {
            if !limiter.check_and_record(user_id).await {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({
                        "error": "Rate limit exceeded. Please slow down.",
                        "retry_after_secs": WINDOW_DURATION.as_secs()
                    })),
                )
                    .into_response();
            }
        }
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allows_under_limit() {
        let limiter = GlobalRateLimiter::new();
        let user = Uuid::new_v4();

        for _ in 0..MAX_REQUESTS_PER_USER {
            assert!(limiter.check_and_record(user).await);
        }
    }

    #[tokio::test]
    async fn test_blocks_over_limit() {
        let limiter = GlobalRateLimiter::new();
        let user = Uuid::new_v4();

        for _ in 0..MAX_REQUESTS_PER_USER {
            limiter.check_and_record(user).await;
        }

        assert!(!limiter.check_and_record(user).await);
    }

    #[tokio::test]
    async fn test_different_users_independent() {
        let limiter = GlobalRateLimiter::new();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();

        for _ in 0..MAX_REQUESTS_PER_USER {
            limiter.check_and_record(user_a).await;
        }

        // User B vẫn OK
        assert!(limiter.check_and_record(user_b).await);
        // User A bị block
        assert!(!limiter.check_and_record(user_a).await);
    }
}
