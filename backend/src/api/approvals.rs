use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use uuid::Uuid;
use validator::Validate;

use crate::api::ip_extractor::extract_client_ip;
use crate::auth::extractors::RequireChecker;
use crate::db;
use crate::error::AppError;
use crate::executor::encryption::decrypt_credentials;
use crate::executor::sql_analyzer::{self, SqlAnalysis};
use crate::executor;
use crate::models::notification::{Notification, NotificationType};
use crate::models::request::RequestStatus;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct RejectBody {
    /// Lý do reject — bắt buộc, min 10 ký tự (BR-004).
    #[validate(length(min = 10, max = 2000, message = "rejection_reason must be 10-2000 characters"))]
    pub rejection_reason: String,
}

#[derive(Debug, Serialize)]
pub struct ApprovalListItem {
    pub id: Uuid,
    pub description: String,
    pub database_id: Uuid,
    pub database_name: Option<String>,
    pub created_by: Uuid,
    pub created_at: String,
    pub submitted_at: Option<String>,
    pub approval_levels_required: i32,
    pub approval_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ApprovalDetailResponse {
    pub id: Uuid,
    pub description: String,
    pub sql_content: String,
    pub status: RequestStatus,
    pub database_id: Uuid,
    pub database_name: Option<String>,
    pub created_by: Uuid,
    pub created_at: String,
    pub submitted_at: Option<String>,
    pub approval_levels_required: i32,
    pub approval_count: i64,
    /// DDL analysis — cảnh báo nếu SQL chứa DDL statements (BR-005)
    pub sql_analysis: SqlAnalysis,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/approvals/pending` — Checker xem danh sách request chờ duyệt.
/// Không trả về sql_content trong list (security).
/// Exclude requests created by the Checker themselves (Req 13.1, 13.2).
pub async fn list_pending(
    State(state): State<AppState>,
    RequireChecker(claims): RequireChecker,
) -> Result<Json<Vec<ApprovalListItem>>, AppError> {
    let requests = db::requests::list_pending(&state.db, claims.user_id)
        .await
        .map_err(AppError::Database)?;

    let mut items: Vec<ApprovalListItem> = Vec::with_capacity(requests.len());
    for r in &requests {
        // Lấy database name
        let db_name = db::databases::get_by_id(&state.db, r.database_id)
            .await
            .ok()
            .flatten()
            .map(|d| d.name);

        // Đếm số approvals hiện có
        let approval_count = db::requests::count_approvals(&state.db, r.id)
            .await
            .unwrap_or(0);

        items.push(ApprovalListItem {
            id: r.id,
            description: r.description.clone(),
            database_id: r.database_id,
            database_name: db_name,
            created_by: r.created_by,
            created_at: r.created_at.to_rfc3339(),
            submitted_at: r.submitted_at.map(|t| t.to_rfc3339()),
            approval_levels_required: r.approval_levels_required,
            approval_count,
        });
    }

    Ok(Json(items))
}

/// `GET /api/v1/approvals/:id` — Checker xem chi tiết request (bao gồm SQL).
pub async fn get_detail(
    State(state): State<AppState>,
    RequireChecker(_claims): RequireChecker,
    Path(id): Path<Uuid>,
) -> Result<Json<ApprovalDetailResponse>, AppError> {
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Lấy database name
    let db_name = db::databases::get_by_id(&state.db, request.database_id)
        .await
        .ok()
        .flatten()
        .map(|d| d.name);

    // Đếm số approvals hiện có
    let approval_count = db::requests::count_approvals(&state.db, id)
        .await
        .unwrap_or(0);

    Ok(Json(ApprovalDetailResponse {
        id: request.id,
        description: request.description,
        sql_content: request.sql_content.clone(),
        status: request.status,
        database_id: request.database_id,
        database_name: db_name,
        created_by: request.created_by,
        created_at: request.created_at.to_rfc3339(),
        submitted_at: request.submitted_at.map(|t| t.to_rfc3339()),
        approval_levels_required: request.approval_levels_required,
        approval_count,
        sql_analysis: sql_analyzer::analyze_sql(&request.sql_content),
    }))
}

/// `POST /api/v1/approvals/:id/approve` — Checker approve request.
///
/// BR-001: 4-eyes principle — checker != maker
/// BR-003: State transition PENDING → APPROVED (chỉ khi đủ approval levels)
/// BR-005: Audit log
/// BR-007: SQL execution trong transaction
/// BR-008: Timeout enforcement
/// Req 10: Multi-level approval — insert approval, check count vs required
pub async fn approve(
    State(state): State<AppState>,
    RequireChecker(claims): RequireChecker,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // BR-003: Validate status is PENDING
    if request.status != RequestStatus::Pending {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "APPROVED".to_string(),
        });
    }

    // BR-001: 4-eyes principle — approver cannot be the maker (Req 10.6)
    if request.created_by == claims.user_id {
        return Err(AppError::FourEyesViolation);
    }

    // Req 10.7: Check duplicate approval (409 Conflict)
    let already_approved = db::requests::has_already_approved(&state.db, id, claims.user_id)
        .await
        .map_err(AppError::Database)?;
    if already_approved {
        return Err(AppError::DuplicateApproval);
    }

    // Extract client IP for audit trail
    let ip = extract_client_ip(&headers, &ConnectInfo(addr));

    // Req 10.3: Determine current level and insert approval
    let current_count = db::requests::count_approvals(&state.db, id)
        .await
        .map_err(AppError::Database)?;
    let new_level = (current_count + 1) as i32;

    let _approval = db::requests::insert_approval(&state.db, id, claims.user_id, new_level)
        .await
        .map_err(AppError::Database)?;

    // Audit log for this approval action
    let _ = db::audit::insert(
        &state.db,
        id,
        "APPROVED",
        claims.user_id,
        Some(&format!("approval level {}/{}", new_level, request.approval_levels_required)),
        None,
        Some(&ip),
    )
    .await;

    // Req 10.5: Check if all required approvals are met
    let total_approvals = db::requests::count_approvals(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    if total_approvals >= request.approval_levels_required as i64 {
        // All required approvals met → transition to APPROVED and trigger execution
        let _updated = db::requests::approve_request(&state.db, id, claims.user_id)
            .await
            .map_err(AppError::Database)?;

        // Spawn async task để execute SQL
        let db_pool = state.db.clone();
        let notification_tx = state.notification_tx.clone();
        let config = state.config.clone();
        let sql_content = request.sql_content.clone();
        let database_id = request.database_id;
        let request_id = id;
        let actor_id = claims.user_id;
        let request_created_by = request.created_by;
        let ip_address = Some(ip.clone());

        tokio::spawn(async move {
            execute_sql_async(
                &db_pool,
                &config,
                &notification_tx,
                request_id,
                database_id,
                &sql_content,
                actor_id,
                request_created_by,
                ip_address,
            )
            .await;
        });

        // Broadcast APPROVED notification
        let _ = state.notification_tx.send(Notification {
            notification_type: NotificationType::RequestApproved,
            request_id: id,
            created_by: Some(request.created_by),
            database_name: None,
            actor_id: Some(claims.user_id),
            rows_affected: None,
            detail: None,
        });

        Ok(Json(serde_json::json!({
            "id": id,
            "status": "APPROVED",
            "approval_level": new_level,
            "approvals_required": request.approval_levels_required,
            "message": "Request fully approved. SQL execution in progress."
        })))
    } else {
        // Req 10.4: Not enough approvals yet — keep PENDING, broadcast partial approval
        let _ = state.notification_tx.send(Notification {
            notification_type: NotificationType::PartialApproval,
            request_id: id,
            created_by: Some(request.created_by),
            database_name: None,
            actor_id: Some(claims.user_id),
            rows_affected: None,
            detail: Some(format!("{}/{} approvals", total_approvals, request.approval_levels_required)),
        });

        Ok(Json(serde_json::json!({
            "id": id,
            "status": "PENDING",
            "approval_level": new_level,
            "approvals_required": request.approval_levels_required,
            "approvals_received": total_approvals,
            "message": format!("Approval recorded ({}/{}). Waiting for more approvals.", total_approvals, request.approval_levels_required)
        })))
    }
}

/// `POST /api/v1/approvals/:id/reject` — Checker reject request.
///
/// BR-001: 4-eyes principle
/// BR-004: Rejection reason min 10 chars
pub async fn reject(
    State(state): State<AppState>,
    RequireChecker(claims): RequireChecker,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<Uuid>,
    Json(body): Json<RejectBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    // BR-004: Validate rejection reason (validator crate)
    body.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // BR-003: Must be PENDING
    if request.status != RequestStatus::Pending {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "REJECTED".to_string(),
        });
    }

    // BR-001: 4-eyes principle
    if request.created_by == claims.user_id {
        return Err(AppError::FourEyesViolation);
    }

    // Extract client IP for audit trail
    let ip = extract_client_ip(&headers, &ConnectInfo(addr));

    // Reject trong DB
    db::requests::reject_request(&state.db, id, &body.rejection_reason)
        .await
        .map_err(AppError::Database)?;

    // Audit log REJECTED
    let _ = db::audit::insert(
        &state.db,
        id,
        "REJECTED",
        claims.user_id,
        Some(&body.rejection_reason),
        None,
        Some(&ip),
    )
    .await;

    // Broadcast REJECTED
    let _ = state.notification_tx.send(Notification {
        notification_type: NotificationType::RequestRejected,
        request_id: id,
        created_by: Some(request.created_by),
        database_name: None,
        actor_id: Some(claims.user_id),
        rows_affected: None,
        detail: Some(body.rejection_reason.clone()),
    });

    Ok(Json(serde_json::json!({
        "id": id,
        "status": "REJECTED",
        "rejection_reason": body.rejection_reason
    })))
}

// ── Async SQL Execution ────────────────────────────────────────────────

/// Thực thi SQL bất đồng bộ sau khi approve.
/// Chạy trong tokio::spawn — không block response.
async fn execute_sql_async(
    pool: &sqlx::PgPool,
    config: &crate::config::Config,
    notification_tx: &tokio::sync::broadcast::Sender<Notification>,
    request_id: Uuid,
    database_id: Uuid,
    sql: &str,
    actor_id: Uuid,
    request_created_by: Uuid,
    ip_address: Option<String>,
) {
    // 1. Lấy database connection info
    let db_conn = match db::databases::get_by_id(pool, database_id).await {
        Ok(Some(conn)) => conn,
        Ok(None) => {
            tracing::error!(%request_id, "database connection not found");
            let _ = db::requests::mark_failed(pool, request_id, "database connection not found").await;
            let _ = db::audit::insert(pool, request_id, "FAILED", actor_id, Some("database connection not found"), None, ip_address.as_deref()).await;
            return;
        }
        Err(e) => {
            tracing::error!(%request_id, error = %e, "failed to fetch database connection");
            let _ = db::requests::mark_failed(pool, request_id, &e.to_string()).await;
            return;
        }
    };

    // 2. Decrypt credentials
    let key_bytes = match config.encryption_key_bytes() {
        Ok(k) => k,
        Err(e) => {
            tracing::error!(%request_id, error = %e, "failed to parse encryption key");
            let _ = db::requests::mark_failed(pool, request_id, "internal: encryption key error").await;
            let _ = db::audit::insert(pool, request_id, "FAILED", actor_id, Some("encryption key error"), None, ip_address.as_deref()).await;
            return;
        }
    };

    let credentials = match decrypt_credentials(&db_conn.encrypted_credentials, &key_bytes) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(%request_id, error = %e, "failed to decrypt credentials");
            let _ = db::requests::mark_failed(pool, request_id, "failed to decrypt database credentials").await;
            let _ = db::audit::insert(pool, request_id, "FAILED", actor_id, Some("credential decryption failed"), None, ip_address.as_deref()).await;
            return;
        }
    };

    // 3. INSERT EXECUTING audit BEFORE SQL execution (Req 11.1, 11.2, 11.3)
    let _ = db::audit::insert(
        pool,
        request_id,
        "EXECUTING",
        actor_id,
        None,
        None,
        ip_address.as_deref(),
    )
    .await;

    // 4. Execute SQL — apply global cap: effective_timeout = min(db_timeout, global_cap)
    let db_timeout_secs = db_conn.timeout_secs as u64;
    let global_cap = config.sql_execution_timeout_secs;
    let effective_timeout = Duration::from_secs(db_timeout_secs.min(global_cap));

    match executor::execute_sql(
        &db_conn.db_type.to_string(),
        sql,
        &credentials,
        &db_conn.host,
        db_conn.port,
        &db_conn.database_name,
        db_conn.schema_name.as_deref(),
        effective_timeout,
    )
    .await
    {
        Ok(result) => {
            // EXECUTED thành công
            let _ = db::requests::mark_executed(pool, request_id, result.rows_affected).await;
            let _ = db::audit::insert(
                pool,
                request_id,
                "EXECUTED",
                actor_id,
                None,
                Some(result.rows_affected),
                ip_address.as_deref(),
            )
            .await;

            let _ = notification_tx.send(Notification {
                notification_type: NotificationType::RequestExecuted,
                request_id,
                created_by: Some(request_created_by),
                database_name: Some(db_conn.name.clone()),
                actor_id: Some(actor_id),
                rows_affected: Some(result.rows_affected),
                detail: None,
            });

            tracing::info!(%request_id, rows_affected = result.rows_affected, "SQL executed successfully");
        }
        Err(e) => {
            // FAILED
            let error_msg = e.to_string();
            let _ = db::requests::mark_failed(pool, request_id, &error_msg).await;
            let _ = db::audit::insert(
                pool,
                request_id,
                "FAILED",
                actor_id,
                Some(&error_msg),
                None,
                ip_address.as_deref(),
            )
            .await;

            let _ = notification_tx.send(Notification {
                notification_type: NotificationType::RequestFailed,
                request_id,
                created_by: Some(request_created_by),
                database_name: Some(db_conn.name.clone()),
                actor_id: Some(actor_id),
                rows_affected: None,
                detail: Some(error_msg.clone()),
            });

            tracing::error!(%request_id, error = %error_msg, "SQL execution failed");
        }
    }
}
