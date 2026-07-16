use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::auth::jwt::{verify_token, Claims};
use crate::error::AppError;
use crate::models::notification::{Notification, NotificationType};
use crate::models::user::Role;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// JWT token cho WebSocket authentication (gửi qua query param vì WS không support headers).
    pub token: String,
}

// ── Handler ────────────────────────────────────────────────────────────

/// `GET /ws?token=JWT` — WebSocket upgrade.
///
/// Flow:
/// 1. Verify JWT từ query param
/// 2. Upgrade HTTP → WebSocket
/// 3. Subscribe broadcast channel
/// 4. Forward notifications tới client (filtered by role)
pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    // Verify JWT trước khi upgrade
    let claims = verify_token(&query.token, &state.config.jwt_secret)
        .map_err(|_| AppError::Unauthorized)?;

    // Subscribe vào broadcast channel
    let rx = state.notification_tx.subscribe();
    let pool = state.db.clone();

    Ok(ws.on_upgrade(|socket| handle_ws_connection(socket, rx, claims, pool)))
}

/// Xử lý WebSocket connection sau khi upgrade.
/// - Forward notifications từ broadcast channel → client (filtered by role)
/// - Ping/Pong tự động bởi axum
async fn handle_ws_connection(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<Notification>,
    claims: Claims,
    pool: PgPool,
) {
    loop {
        tokio::select! {
            // Nhận notification từ broadcast channel
            result = rx.recv() => {
                match result {
                    Ok(notification) => {
                        // Apply role-based filtering
                        if !should_forward(&notification, &claims, &pool).await {
                            continue;
                        }

                        let json = match serde_json::to_string(&notification) {
                            Ok(j) => j,
                            Err(_) => continue,
                        };

                        if socket.send(Message::Text(json)).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(lagged = n, "WebSocket client lagged behind, skipping messages");
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }

            // Nhận message từ client (ping/pong/close)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {
                        // Ignore text/binary messages từ client
                    }
                }
            }
        }
    }
}

/// Xác định notification có nên forward cho user này không (role-based filtering).
///
/// Rules:
/// - Admin → always forward
/// - Non-Admin without created_by → discard
/// - Maker → forward if created_by == user_id
/// - Checker → forward if NEW_REQUEST or previously reviewed this request
/// - Maker+Checker → union of Maker and Checker rules
async fn should_forward(
    notification: &Notification,
    claims: &Claims,
    pool: &PgPool,
) -> bool {
    // Admin → always forward
    if claims.has_role(&Role::Admin) {
        return true;
    }

    // Non-Admin: check created_by field
    let created_by = match notification.created_by {
        Some(id) => id,
        None => return false, // discard if missing created_by for non-Admin
    };

    let mut forward = false;

    // Maker rule: forward if created_by == user_id
    if claims.has_role(&Role::Maker) && created_by == claims.user_id {
        forward = true;
    }

    // Checker rule: NEW_REQUEST or previously reviewed this request
    if !forward && claims.has_role(&Role::Checker) {
        if notification.notification_type == NotificationType::NewRequest {
            forward = true;
        } else {
            // Check if checker has previously approved/rejected this request
            let has_reviewed =
                has_previously_reviewed(pool, notification.request_id, claims.user_id).await;
            if has_reviewed {
                forward = true;
            }
        }
    }

    forward
}

/// Check if a user has previously approved or rejected a request
/// by looking up the request_approvals table.
async fn has_previously_reviewed(pool: &PgPool, request_id: Uuid, user_id: Uuid) -> bool {
    let result = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM request_approvals WHERE request_id = $1 AND approver_id = $2)",
    )
    .bind(request_id)
    .bind(user_id)
    .fetch_one(pool)
    .await;

    match result {
        Ok(exists) => exists,
        Err(_) => false, // on DB error, don't forward (fail secure)
    }
}
