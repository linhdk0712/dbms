use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    Json,
};
use chrono::Utc;
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

/// Request body cho endpoint reject — chứa lý do từ chối.
#[derive(Debug, Deserialize, Validate)]
pub struct RejectBody {
    /// Lý do reject — bắt buộc, min 10 ký tự (BR-004).
    #[validate(length(min = 10, max = 2000, message = "rejection_reason must be 10-2000 characters"))]
    pub rejection_reason: String,
}

/// DTO trả về trong danh sách request chờ duyệt (list pending).
///
/// Không bao gồm `sql_content` để đảm bảo security — Checker phải
/// mở detail mới thấy nội dung SQL.
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

/// DTO trả về chi tiết request cho Checker review.
///
/// Bao gồm `sql_content` và `sql_analysis` (DDL detection) để Checker
/// đánh giá mức độ rủi ro trước khi approve/reject.
#[derive(Debug, Serialize)]
pub struct ApprovalDetailResponse {
    pub id: Uuid,
    pub description: String,
    pub sql_content: String,
    pub status: RequestStatus,
    pub database_id: Uuid,
    pub database_name: Option<String>,
    /// Connection string hiển thị (không chứa credentials), VD: "postgresql://host:5432/dbname"
    pub database_connection_string: Option<String>,
    pub created_by: Uuid,
    pub created_at: String,
    pub submitted_at: Option<String>,
    pub approval_levels_required: i32,
    pub approval_count: i64,
    /// DDL analysis — cảnh báo nếu SQL chứa DDL statements (BR-005)
    pub sql_analysis: SqlAnalysis,
    /// Kết quả validate SQL đã lưu (nếu có) — hiển thị lại khi reload trang
    pub validation_result: Option<serde_json::Value>,
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

    // Lấy database info
    let db_conn = db::databases::get_by_id(&state.db, request.database_id)
        .await
        .ok()
        .flatten();

    let db_name = db_conn.as_ref().map(|d| d.name.clone());
    let db_connection_string = db_conn.as_ref().map(|d| {
        let schema_part = d.schema_name.as_deref().map_or(String::new(), |s| format!("/{}", s));
        format!("{}://{}:{}/{}{}", d.db_type, d.host, d.port, d.database_name, schema_part)
    });

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
        database_connection_string: db_connection_string,
        created_by: request.created_by,
        created_at: request.created_at.to_rfc3339(),
        submitted_at: request.submitted_at.map(|t| t.to_rfc3339()),
        approval_levels_required: request.approval_levels_required,
        approval_count,
        sql_analysis: sql_analyzer::analyze_sql(&request.sql_content),
        validation_result: request.validation_result,
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

        // Check maintenance window (CR integration)
        crate::api::change_requests::check_maintenance_window(&state.db, request.cr_id).await?;

        let _updated = db::requests::approve_request(&state.db, id, claims.user_id)
            .await
            .map_err(AppError::Database)?;

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

        // Execute SQL synchronously and return result in response
        let execution_result = execute_sql_sync(
            &state.db,
            &state.config,
            &state.notification_tx,
            id,
            request.database_id,
            &request.sql_content,
            claims.user_id,
            request.created_by,
            Some(ip.clone()),
        )
        .await;

        match execution_result {
            Ok(result) => {
                Ok(Json(serde_json::json!({
                    "id": id,
                    "status": "EXECUTED",
                    "approval_level": new_level,
                    "approvals_required": request.approval_levels_required,
                    "message": "Request approved and executed successfully.",
                    "execution_result": {
                        "success": true,
                        "rows_affected": result.rows_affected
                    }
                })))
            }
            Err(error_msg) => {
                Ok(Json(serde_json::json!({
                    "id": id,
                    "status": "FAILED",
                    "approval_level": new_level,
                    "approvals_required": request.approval_levels_required,
                    "message": "Request approved but execution failed.",
                    "execution_result": {
                        "success": false,
                        "error": error_msg
                    }
                })))
            }
        }
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

// ── Synchronous SQL Execution (returns result to caller) ───────────────

/// `POST /api/v1/approvals/:id/retry` — Re-execute request đã FAILED.
///
/// Cho phép Checker hoặc Admin retry execution khi request bị lỗi do
/// connection timeout, network issue, hoặc lỗi tạm thời khác.
/// Request phải đang ở trạng thái FAILED.
///
/// Flow:
/// 1. Validate status = FAILED
/// 2. Reset status → APPROVED (xóa execution_error cũ)
/// 3. Ghi audit log RETRY
/// 4. Re-execute SQL
/// 5. Trả kết quả (EXECUTED hoặc FAILED lại)
pub async fn retry(
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

    // Chỉ retry request đang FAILED
    if request.status != RequestStatus::Failed {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "RETRY".to_string(),
        });
    }

    // Extract client IP for audit trail
    let ip = extract_client_ip(&headers, &ConnectInfo(addr));

    // Reset status → APPROVED
    db::requests::reset_to_approved(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    // Audit log: RETRY
    let _ = db::audit::insert(
        &state.db,
        id,
        "RETRY",
        claims.user_id,
        Some("Re-executing after failure"),
        None,
        Some(&ip),
    )
    .await;

    // Re-execute SQL
    let execution_result = execute_sql_sync(
        &state.db,
        &state.config,
        &state.notification_tx,
        id,
        request.database_id,
        &request.sql_content,
        claims.user_id,
        request.created_by,
        Some(ip.clone()),
    )
    .await;

    match execution_result {
        Ok(result) => {
            Ok(Json(serde_json::json!({
                "id": id,
                "status": "EXECUTED",
                "message": "Retry successful — SQL executed.",
                "execution_result": {
                    "success": true,
                    "rows_affected": result.rows_affected
                }
            })))
        }
        Err(error_msg) => {
            Ok(Json(serde_json::json!({
                "id": id,
                "status": "FAILED",
                "message": "Retry failed — execution error persists.",
                "execution_result": {
                    "success": false,
                    "error": error_msg
                }
            })))
        }
    }
}

/// `POST /api/v1/approvals/:id/validate` — Validate SQL trước khi approve.
///
/// Chạy EXPLAIN trên database đích để kiểm tra:
/// - Syntax SQL hợp lệ
/// - Bảng/cột tồn tại (schema validation)
/// - Execution plan và performance warnings
///
/// Không thực thi SQL — chỉ validate.
/// Checker PHẢI validate thành công trước khi được phép Approve & Execute.
pub async fn validate(
    State(state): State<AppState>,
    RequireChecker(_claims): RequireChecker,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Chỉ validate request đang PENDING
    if request.status != RequestStatus::Pending {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "VALIDATE".to_string(),
        });
    }

    // Lấy database connection info
    let db_conn = db::databases::get_by_id(&state.db, request.database_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Decrypt credentials
    let key_bytes = state
        .config
        .encryption_key_bytes()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("encryption key error: {e}")))?;

    let credentials = decrypt_credentials(&db_conn.encrypted_credentials, &key_bytes)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("credential decryption failed: {e}")))?;

    // Timeout cho validation (dùng timeout ngắn hơn execution)
    let validate_timeout = Duration::from_secs(30);

    // Run EXPLAIN validation
    let result = executor::validate_sql(
        &db_conn.db_type.to_string(),
        &request.sql_content,
        &credentials,
        &db_conn.host,
        db_conn.port,
        &db_conn.database_name,
        db_conn.schema_name.as_deref(),
        validate_timeout,
    )
    .await;

    // Lưu kết quả validate vào DB để hiển thị khi reload trang
    let validation_json = serde_json::json!({
        "valid": result.valid,
        "execution_plan": result.execution_plan,
        "warnings": result.warnings,
        "error": result.error,
        "validated_at": Utc::now().to_rfc3339()
    });

    let _ = db::requests::save_validation_result(&state.db, id, &validation_json).await;

    Ok(Json(validation_json))
}

/// Thực thi SQL đồng bộ sau khi request được approve đủ levels.
///
/// Block cho đến khi execution hoàn thành, trả kết quả trực tiếp trong
/// HTTP response — giúp caller (approve handler) biết ngay trạng thái.
///
/// # Flow
/// 1. Lấy database connection info từ PostgreSQL nội bộ
/// 2. Decrypt credentials (AES-256-GCM)
/// 3. Ghi audit log `EXECUTING` trước khi chạy SQL (Req 11.1–11.3)
/// 4. Thực thi SQL trên database đích với timeout = min(db_timeout, global_cap)
/// 5. Cập nhật trạng thái request (`EXECUTED`/`FAILED`) và ghi audit log
/// 6. Broadcast notification qua WebSocket channel
///
/// # Returns
/// - `Ok(ExecutionResult)` — thực thi thành công, kèm `rows_affected`
/// - `Err(String)` — thực thi thất bại, kèm error message mô tả lỗi
async fn execute_sql_sync(
    pool: &sqlx::PgPool,
    config: &crate::config::Config,
    notification_tx: &tokio::sync::broadcast::Sender<Notification>,
    request_id: Uuid,
    database_id: Uuid,
    sql: &str,
    actor_id: Uuid,
    request_created_by: Uuid,
    ip_address: Option<String>,
) -> Result<executor::ExecutionResult, String> {
    // 1. Lấy database connection info
    let db_conn = match db::databases::get_by_id(pool, database_id).await {
        Ok(Some(conn)) => conn,
        Ok(None) => {
            let msg = "database connection not found".to_string();
            let _ = db::requests::mark_failed(pool, request_id, &msg).await;
            let _ = db::audit::insert(pool, request_id, "FAILED", actor_id, Some(&msg), None, ip_address.as_deref()).await;
            return Err(msg);
        }
        Err(e) => {
            let msg = format!("failed to fetch database connection: {}", e);
            let _ = db::requests::mark_failed(pool, request_id, &msg).await;
            return Err(msg);
        }
    };

    // 2. Decrypt credentials
    let key_bytes = match config.encryption_key_bytes() {
        Ok(k) => k,
        Err(e) => {
            let msg = format!("internal: encryption key error: {}", e);
            let _ = db::requests::mark_failed(pool, request_id, "internal: encryption key error").await;
            let _ = db::audit::insert(pool, request_id, "FAILED", actor_id, Some("encryption key error"), None, ip_address.as_deref()).await;
            return Err(msg);
        }
    };

    let credentials = match decrypt_credentials(&db_conn.encrypted_credentials, &key_bytes) {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("failed to decrypt database credentials: {}", e);
            let _ = db::requests::mark_failed(pool, request_id, "failed to decrypt database credentials").await;
            let _ = db::audit::insert(pool, request_id, "FAILED", actor_id, Some("credential decryption failed"), None, ip_address.as_deref()).await;
            return Err(msg);
        }
    };

    // 3. INSERT EXECUTING audit BEFORE SQL execution
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

    // 4. Execute SQL
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
            Ok(result)
        }
        Err(e) => {
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
            Err(error_msg)
        }
    }
}

// ── Async SQL Execution (legacy — kept for reference) ──────────────────

/// Thực thi SQL bất đồng bộ sau khi approve (legacy).
///
/// Được giữ lại để tham khảo — trước đây chạy trong `tokio::spawn` để không
/// block HTTP response. Đã thay bằng [`execute_sql_sync`] để trả kết quả
/// execution trực tiếp trong approve response.
///
/// # Note
/// Hàm này không trả giá trị — kết quả được cập nhật vào DB và broadcast
/// qua notification channel.
#[allow(dead_code)]
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
