mod api;
mod auth;
mod config;
mod db;
mod error;
mod executor;
mod models;
mod state;
mod ws;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;
use tracing_appender::rolling;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::auth::oidc::create_oidc_client;
use crate::api::auth::oidc_session_cleanup;
use crate::config::Config;
use crate::models::notification::Notification;
use crate::state::AppState;

/// Kích thước buffer cho kênh broadcast notification.
const NOTIFICATION_CHANNEL_CAPACITY: usize = 128;

/// Số connection tối đa trong PgPool.
const MAX_DB_CONNECTIONS: u32 = 20;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load .env (nếu có) — bỏ qua lỗi nếu không tồn tại
    let _ = dotenvy::dotenv();

    // 2. Load config — fail fast nếu thiếu required vars
    let config = Config::from_env().context("failed to load configuration")?;

    // 3. Init structured logging (stdout JSON + file daily rotation)
    let _guard = init_tracing(&config.log_dir);
    tracing::info!("configuration loaded successfully");

    // 4. Kết nối database pool
    let db = PgPoolOptions::new()
        .max_connections(MAX_DB_CONNECTIONS)
        .connect(&config.database_url)
        .await
        .context("failed to connect to database")?;
    tracing::info!("database pool connected (max {} connections)", MAX_DB_CONNECTIONS);

    // 5. Chạy migrations — fail fast nếu lỗi
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .context("failed to run database migrations")?;
    tracing::info!("database migrations applied");

    // 6. Tạo kênh broadcast cho notifications
    let (notification_tx, _rx) = broadcast::channel::<Notification>(NOTIFICATION_CHANNEL_CAPACITY);

    // 7. Init OIDC client (graceful — không fail nếu provider down)
    let oidc_client = match create_oidc_client(&config).await {
        Ok(client) => {
            tracing::info!("OIDC client initialized successfully");
            Some(client)
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to initialize OIDC client — SSO login will be unavailable");
            None
        }
    };

    // 8. Build AppState
    let port = config.port;
    let mut state = AppState::new(db, config, notification_tx);
    if let Some(client) = oidc_client {
        state = state.with_oidc_client(client);
    }

    // 8.5. Spawn OIDC session cleanup background task
    {
        let store = state.oidc_sessions.clone();
        let ttl = std::time::Duration::from_secs(state.config.oidc_session_ttl_secs);
        let interval = std::time::Duration::from_secs(state.config.oidc_cleanup_interval_secs);
        tokio::spawn(oidc_session_cleanup(store, ttl, interval));
        tracing::info!(
            ttl_secs = state.config.oidc_session_ttl_secs,
            interval_secs = state.config.oidc_cleanup_interval_secs,
            "OIDC session cleanup task started"
        );
    }

    // 9. Build router với middleware
    let app = api::build_router(state);

    // 10. Bind và serve
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;
    tracing::info!("SECS backend listening on {addr}");

    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .await
        .context("server error")?;

    Ok(())
}

/// Khởi tạo tracing với dual output:
/// - stdout: JSON structured (cho Docker/container logs, log aggregators)
/// - file: daily rotation, human-readable format (cho manual debug)
///
/// Level được điều khiển bởi RUST_LOG env var (default: info).
/// File log được lưu tại `{log_dir}/secs-backend.YYYY-MM-DD.log`.
///
/// Trả về `WorkerGuard` — PHẢI giữ trong scope của main() để đảm bảo
/// tất cả log được flush trước khi process exit.
fn init_tracing(log_dir: &str) -> tracing_appender::non_blocking::WorkerGuard {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // File appender: daily rotation, prefix "secs-backend"
    let file_appender = rolling::daily(log_dir, "secs-backend.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    // Stdout layer: JSON format (machine-readable, cho log aggregators)
    let stdout_layer = tracing_subscriber::fmt::layer().json();

    // File layer: plain text với timestamps, targets, thread IDs (human-readable)
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    guard
}
