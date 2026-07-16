use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::extractors::RequireAuth;
use crate::db;
use crate::error::AppError;
use crate::models::approval::AuditLog;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: Uuid,
    pub request_id: Option<Uuid>,
    pub action: String,
    pub actor_id: Uuid,
    pub detail: Option<String>,
    pub execution_result: Option<i64>,
    pub timestamp: String,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/requests/:id/audit` — Xem audit trail của một request.
///
/// Accessible cho:
/// - Maker: chỉ request mình tạo
/// - Checker: request mình đã review
/// - Admin: tất cả
pub async fn get_request_audit(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<AuditLogResponse>>, AppError> {
    // Verify request tồn tại
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Access control: Maker chỉ xem audit request của mình,
    // Checker xem nếu mình là người approve/reject, Admin xem tất cả
    let is_owner = request.created_by == claims.user_id;
    let is_approver = request.approved_by == Some(claims.user_id);
    let is_admin = claims.is_admin();

    if !is_owner && !is_approver && !is_admin {
        return Err(AppError::Forbidden);
    }

    let logs = db::audit::get_by_request_id(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    let responses: Vec<AuditLogResponse> = logs
        .into_iter()
        .map(|log| to_audit_response(log))
        .collect();

    Ok(Json(responses))
}

// ── Helpers ────────────────────────────────────────────────────────────

fn to_audit_response(log: AuditLog) -> AuditLogResponse {
    AuditLogResponse {
        id: log.id,
        request_id: log.request_id,
        action: log.action.to_string(),
        actor_id: log.actor_id,
        detail: log.detail,
        execution_result: log.execution_result,
        timestamp: log.timestamp.to_rfc3339(),
    }
}
