use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::extractors::RequireAuth;
use crate::db;
use crate::error::AppError;
use crate::models::data_query::DataQueryStatus;
use crate::models::user::Role;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

/// Request body for approving a data query at the current approval level.
#[derive(Debug, Deserialize)]
pub struct ApproveBody {
    /// Optional comment from the approver explaining the approval decision.
    pub comment: Option<String>,
}

/// Request body for rejecting a data query request.
#[derive(Debug, Deserialize)]
pub struct RejectBody {
    /// Mandatory rejection reason (minimum 10 characters).
    pub comment: String,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/data-query-approvals/pending` — List pending approvals.
///
/// Trả về data queries có level PENDING matching user's approver_unit.
/// approver_unit được xác định từ user role/claims.
pub async fn list_pending(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
) -> Result<Json<serde_json::Value>, AppError> {
    // Determine user's approver_unit from roles
    let approver_unit = determine_approver_unit(&claims.roles)
        .ok_or(AppError::Forbidden)?;

    let requests = db::data_query_approvals::list_pending_by_unit(&state.db, &approver_unit)
        .await
        .map_err(AppError::Database)?;

    let mut items: Vec<serde_json::Value> = Vec::with_capacity(requests.len());
    for r in &requests {
        // Lookup creator name and database name
        let creator_name = db::users::get_display_name(&state.db, r.created_by)
            .await
            .ok();
        let database_name = db::databases::get_by_id(&state.db, r.database_id)
            .await
            .ok()
            .flatten()
            .map(|d| d.name);

        items.push(serde_json::json!({
            "id": r.id,
            "title": r.title,
            "database_id": r.database_id,
            "database_name": database_name,
            "schema_name": r.schema_name,
            "sql_query": r.sql_query,
            "tables_accessed": r.tables_accessed,
            "query_mode": r.query_mode.to_string(),
            "ticket_id": r.ticket_id,
            "status": r.status.to_string(),
            "created_by": r.created_by,
            "creator_name": creator_name,
            "current_approval_level": r.current_approval_level,
            "total_approval_levels": r.total_approval_levels,
            "approver_unit": approver_unit,
            "expires_at": r.expires_at,
            "created_at": r.created_at,
        }));
    }

    Ok(Json(serde_json::json!(items)))
}

/// `POST /api/v1/data-queries/:id/approve` — Approve current level.
pub async fn approve(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
    Json(body): Json<ApproveBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let request = db::data_queries::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Must be PENDING_APPROVAL
    if request.status != DataQueryStatus::PendingApproval {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "APPROVE".to_string(),
        });
    }

    // Verify user has matching approver_unit for current level
    let approver_unit = determine_approver_unit(&claims.roles)
        .ok_or(AppError::Forbidden)?;

    // Check tracking: current level must match approver's unit
    let tracking = db::data_queries::get_approval_tracking(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    let current_tracking = tracking
        .iter()
        .find(|t| t.level_order == request.current_approval_level);

    match current_tracking {
        Some(t) if t.approver_unit.to_string() == approver_unit => {}
        _ => return Err(AppError::Forbidden),
    }

    // Cannot approve own request
    if request.created_by == claims.user_id {
        return Err(AppError::FourEyesViolation);
    }

    let fully_approved = db::data_query_approvals::approve_level(
        &state.db,
        id,
        claims.user_id,
        body.comment.as_deref(),
    )
    .await
    .map_err(AppError::Database)?;

    if fully_approved {
        Ok(Json(serde_json::json!({
            "id": id,
            "status": "APPROVED",
            "message": "Data query fully approved — now executable"
        })))
    } else {
        Ok(Json(serde_json::json!({
            "id": id,
            "status": "PENDING_APPROVAL",
            "message": "Level approved — waiting for next level"
        })))
    }
}

/// `POST /api/v1/data-queries/:id/reject` — Reject request.
pub async fn reject(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
    Json(body): Json<RejectBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.comment.trim().len() < 10 {
        return Err(AppError::ValidationError(
            "Rejection comment must be at least 10 characters".to_string(),
        ));
    }

    let request = db::data_queries::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if request.status != DataQueryStatus::PendingApproval {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "REJECTED".to_string(),
        });
    }

    // Cannot reject own request
    if request.created_by == claims.user_id {
        return Err(AppError::FourEyesViolation);
    }

    db::data_query_approvals::reject_request(
        &state.db,
        id,
        claims.user_id,
        &body.comment,
    )
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "id": id,
        "status": "REJECTED",
        "message": "Data query rejected"
    })))
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Determine approver_unit from user roles.
/// Maps roles to approval units.
fn determine_approver_unit(roles: &[Role]) -> Option<String> {
    // Checker role maps to ITO (team lead approval)
    if roles.contains(&Role::Checker) {
        return Some("ITO".to_string());
    }
    // Admin can also approve as ITO
    if roles.contains(&Role::Admin) {
        return Some("ITO".to_string());
    }
    None
}
