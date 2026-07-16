use openidconnect::core::CoreClient;
use sqlx::PgPool;
use tokio::sync::broadcast;

use crate::api::auth::OidcSessionStore;
use crate::config::Config;
use crate::models::notification::Notification;

/// State toàn cục được chia sẻ giữa các handler.
/// Clone rẻ — `PgPool`, `broadcast::Sender`, `Arc<CoreClient>` đều là Arc bên trong.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
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
        Self {
            db,
            config,
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
