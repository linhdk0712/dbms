use anyhow::{Context, Result};

/// Cấu hình ứng dụng — load từ environment variables.
/// Fail fast (panic) nếu thiếu bất kỳ required config nào.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    /// 32-byte encryption key dưới dạng hex string (64 chars)
    pub encryption_key: String,
    pub oidc_issuer_url: String,
    pub oidc_client_id: String,
    pub oidc_client_secret: String,
    pub oidc_redirect_uri: String,
    /// Origin của frontend — dùng cho CORS policy
    pub frontend_url: String,
    /// Timeout mặc định cho SQL execution (giây), default: 30
    pub sql_execution_timeout_secs: u64,
    /// OIDC session TTL in seconds (default: 300, range: 60-900)
    pub oidc_session_ttl_secs: u64,
    /// OIDC cleanup interval in seconds (default: 60, range: 10-300)
    pub oidc_cleanup_interval_secs: u64,
    pub port: u16,
    /// Thư mục chứa log files (default: "./logs")
    pub log_dir: String,
}

impl Config {
    /// Load config từ environment. Gọi `dotenvy::dotenv()` trước nếu dùng .env file.
    /// Panics ngay nếu thiếu bất kỳ required variable nào.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: required_env("DATABASE_URL")?,
            jwt_secret: required_env("JWT_SECRET")
                .and_then(|s| {
                    anyhow::ensure!(
                        s.len() >= 32,
                        "JWT_SECRET must be at least 32 characters"
                    );
                    Ok(s)
                })?,
            encryption_key: required_env("ENCRYPTION_KEY")
                .and_then(|s| {
                    anyhow::ensure!(
                        s.len() == 64,
                        "ENCRYPTION_KEY must be exactly 64 hex characters (32 bytes)"
                    );
                    Ok(s)
                })?,
            oidc_issuer_url: required_env("OIDC_ISSUER_URL")?,
            oidc_client_id: required_env("OIDC_CLIENT_ID")?,
            oidc_client_secret: required_env("OIDC_CLIENT_SECRET")?,
            oidc_redirect_uri: required_env("OIDC_REDIRECT_URI")?,
            frontend_url: required_env("FRONTEND_URL")?,
            sql_execution_timeout_secs: std::env::var("SQL_EXECUTION_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse::<u64>()
                .context("SQL_EXECUTION_TIMEOUT_SECS must be a valid integer")?
                .min(300), // hard cap 300s
            oidc_session_ttl_secs: std::env::var("OIDC_SESSION_TTL_SECS")
                .unwrap_or_else(|_| "300".to_string())
                .parse::<u64>()
                .context("OIDC_SESSION_TTL_SECS must be a valid integer")?
                .max(60)
                .min(900),
            oidc_cleanup_interval_secs: std::env::var("OIDC_CLEANUP_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse::<u64>()
                .context("OIDC_CLEANUP_INTERVAL_SECS must be a valid integer")?
                .max(10)
                .min(300),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse::<u16>()
                .context("PORT must be a valid port number")?,
            log_dir: std::env::var("LOG_DIR").unwrap_or_else(|_| "./logs".to_string()),
        })
    }

    /// Parse encryption key từ hex string thành 32-byte array.
    pub fn encryption_key_bytes(&self) -> Result<[u8; 32]> {
        let bytes = hex::decode(&self.encryption_key)
            .context("ENCRYPTION_KEY is not valid hex")?;
        bytes.try_into().map_err(|_| anyhow::anyhow!("ENCRYPTION_KEY must be exactly 32 bytes"))
    }
}

fn required_env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("Missing required environment variable: {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_env_missing() {
        // Đảm bảo biến không tồn tại
        std::env::remove_var("__SECS_TEST_MISSING_VAR__");
        let result = required_env("__SECS_TEST_MISSING_VAR__");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("__SECS_TEST_MISSING_VAR__"));
    }
}
