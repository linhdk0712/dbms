use openidconnect::core::CoreClient;
use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::ai::{AiCache, AiConfig, AiRateLimiter};
use crate::api::auth::OidcSessionStore;
use crate::config::Config;
use crate::models::notification::Notification;

/// State toàn cục được chia sẻ giữa các handler.
/// Clone rẻ — `PgPool`, `broadcast::Sender`, `Arc<CoreClient>` đều là Arc bên trong.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    /// Cấu hình AI analysis module.
    pub ai_config: AiConfig,
    /// Rate limiter per user cho AI analysis.
    pub ai_rate_limiter: AiRateLimiter,
    /// Cache kết quả AI analysis per request.
    pub ai_cache: AiCache,
    /// Kênh broadcast cho realtime notification qua WebSocket.
    pub notification_tx: broadcast::Sender<Notification>,
    /// OIDC client — None nếu OIDC provider không available lúc startup.
    pub oidc_client: Option<std::sync::Arc<CoreClient>>,
    /// In-memory session store cho OIDC PKCE flow.
    pub oidc_sessions: OidcSessionStore,
}

impl AppState {
    pub fn new(
        db: PgPool,
        config: Config,
        notification_tx: broadcast::Sender<Notification>,
    ) -> Self {
        let ai_config = AiConfig::from_env();
        let ai_rate_limiter = AiRateLimiter::new(ai_config.rate_limit_per_user);
        let ai_cache = AiCache::new(600); // 10 minutes TTL
        Self {
            db,
            config,
            ai_config,
            ai_rate_limiter,
            ai_cache,
            notification_tx,
            oidc_client: None,
            oidc_sessions: crate::api::auth::new_session_store(),
        }
    }

    /// Set OIDC client sau khi khởi tạo thành công.
    pub fn with_oidc_client(mut self, client: CoreClient) -> Self {
        self.oidc_client = Some(std::sync::Arc::new(client));
        self
    }
}
