//! Token Blacklist — cho phép revoke JWT tokens trước khi hết hạn.
//!
//! Use cases:
//! - User logout → token bị revoke
//! - Admin deactivate user → tất cả tokens của user bị revoke
//!
//! Implementation: In-memory HashMap với TTL.
//! Mỗi entry tự động bị xóa khi token hết hạn (periodic cleanup).
//!
//! Cho single-instance internal app, in-memory là đủ.
//! Production multi-instance cần Redis.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Token blacklist entry.
#[derive(Debug, Clone)]
struct BlacklistEntry {
    /// Thời điểm entry được thêm vào.
    added_at: Instant,
    /// TTL — entry sẽ bị xóa sau khoảng thời gian này (= token remaining lifetime).
    ttl: Duration,
}

/// Token Blacklist — lưu trữ các token/user_id đã bị revoke.
///
/// Hai cơ chế revoke:
/// 1. **By token JTI (sub + iat)**: revoke token cụ thể (logout)
/// 2. **By user_id**: revoke tất cả tokens của user (deactivate)
#[derive(Clone)]
pub struct TokenBlacklist {
    /// Key = "token:{sub}:{iat}" — revoke token cụ thể
    revoked_tokens: Arc<RwLock<HashMap<String, BlacklistEntry>>>,
    /// Key = user_id — revoke tất cả tokens issued trước thời điểm này
    /// Value = timestamp (Unix seconds) — tokens có iat <= value sẽ bị reject
    revoked_users: Arc<RwLock<HashMap<Uuid, u64>>>,
}

impl TokenBlacklist {
    pub fn new() -> Self {
        Self {
            revoked_tokens: Arc::new(RwLock::new(HashMap::new())),
            revoked_users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Revoke một token cụ thể (dùng khi logout).
    ///
    /// `token_key` = "{sub}:{iat}" — unique identifier cho token.
    /// `remaining_ttl` = thời gian còn lại trước khi token hết hạn.
    pub async fn revoke_token(&self, token_key: &str, remaining_ttl: Duration) {
        let mut map = self.revoked_tokens.write().await;
        map.insert(
            token_key.to_string(),
            BlacklistEntry {
                added_at: Instant::now(),
                ttl: remaining_ttl,
            },
        );
    }

    /// Revoke tất cả tokens của một user (dùng khi deactivate user).
    ///
    /// Tất cả tokens có `iat <= revoked_before` sẽ bị reject.
    pub async fn revoke_user(&self, user_id: Uuid) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before Unix epoch")
            .as_secs();

        let mut map = self.revoked_users.write().await;
        map.insert(user_id, now);
    }

    /// Kiểm tra token có bị revoke không.
    ///
    /// Returns `true` nếu token bị blacklisted (should be rejected).
    pub async fn is_revoked(&self, user_id: Uuid, sub: &str, iat: usize) -> bool {
        // Check 1: token cụ thể bị revoke?
        let token_key = format!("{}:{}", sub, iat);
        {
            let map = self.revoked_tokens.read().await;
            if map.contains_key(&token_key) {
                return true;
            }
        }

        // Check 2: user bị revoke (tất cả tokens trước thời điểm X)?
        {
            let map = self.revoked_users.read().await;
            if let Some(&revoked_before) = map.get(&user_id) {
                if (iat as u64) <= revoked_before {
                    return true;
                }
            }
        }

        false
    }

    /// Cleanup entries đã hết hạn — gọi periodically.
    pub async fn cleanup(&self) {
        let now = Instant::now();

        // Cleanup revoked tokens
        {
            let mut map = self.revoked_tokens.write().await;
            let before = map.len();
            map.retain(|_, entry| now.duration_since(entry.added_at) < entry.ttl);
            let removed = before - map.len();
            if removed > 0 {
                tracing::debug!(removed, remaining = map.len(), "token blacklist cleanup");
            }
        }

        // Cleanup user revocations older than 8 hours (max token lifetime)
        {
            let max_token_lifetime_secs = 8 * 60 * 60;
            let now_unix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before Unix epoch")
                .as_secs();

            let mut map = self.revoked_users.write().await;
            let before = map.len();
            map.retain(|_, &mut revoked_at| {
                now_unix.saturating_sub(revoked_at) < max_token_lifetime_secs
            });
            let removed = before - map.len();
            if removed > 0 {
                tracing::debug!(removed, "user revocation cleanup");
            }
        }
    }
}

impl Default for TokenBlacklist {
    fn default() -> Self {
        Self::new()
    }
}

/// Background task: periodic cleanup of expired blacklist entries.
pub async fn blacklist_cleanup_task(blacklist: TokenBlacklist, interval: Duration) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        blacklist.cleanup().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_revoke_token() {
        let bl = TokenBlacklist::new();
        let user_id = Uuid::new_v4();

        assert!(!bl.is_revoked(user_id, "user-1", 1000).await);

        bl.revoke_token("user-1:1000", Duration::from_secs(3600)).await;

        assert!(bl.is_revoked(user_id, "user-1", 1000).await);
        assert!(!bl.is_revoked(user_id, "user-1", 2000).await);
    }

    #[tokio::test]
    async fn test_revoke_user() {
        let bl = TokenBlacklist::new();
        let user_id = Uuid::new_v4();

        // Token issued at time 1000
        assert!(!bl.is_revoked(user_id, "sub", 1000).await);

        // Revoke user — all tokens issued before "now" are invalid
        bl.revoke_user(user_id).await;

        // Token issued at 1000 (past) → revoked
        assert!(bl.is_revoked(user_id, "sub", 1000).await);

        // Token issued far in the future (hypothetical) → not revoked
        let future_iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize + 100;
        assert!(!bl.is_revoked(user_id, "sub", future_iat).await);
    }

    #[tokio::test]
    async fn test_different_users_independent() {
        let bl = TokenBlacklist::new();
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();

        bl.revoke_user(user_a).await;

        assert!(bl.is_revoked(user_a, "a", 1000).await);
        assert!(!bl.is_revoked(user_b, "b", 1000).await);
    }

    #[tokio::test]
    async fn test_cleanup_removes_expired() {
        let bl = TokenBlacklist::new();

        // Add entry with very short TTL
        bl.revoke_token("short:1", Duration::from_millis(10)).await;
        assert!(bl.is_revoked(Uuid::new_v4(), "short", 1).await);

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(20)).await;

        bl.cleanup().await;

        assert!(!bl.is_revoked(Uuid::new_v4(), "short", 1).await);
    }
}
