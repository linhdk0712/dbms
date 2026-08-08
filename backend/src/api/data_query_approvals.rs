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
/// Trả về data queries có level PENDING matching user's approver_units.
/// approver_units được lấy từ bảng user_approver_units.
pub async fn list_pending(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
) -> Result<Json<serde_json::Value>, AppError> {
    // Get user's approver units from DB
    let user_units = db::approver_units::get_user_units(&state.db, claims.user_id)
        .await
        .map_err(AppError::Database)?;

    if user_units.is_empty() {
        return Ok(Json(serde_json::json!([])));
    }

    // Query pending requests for ALL units the user belongs to
    let mut all_items: Vec<serde_json::Value> = Vec::new();
    for unit in &user_units {
        let requests = db::data_query_approvals::list_pending_by_unit(&state.db, unit)
            .await
            .map_err(AppError::Database)?;

        for r in &requests {
            let creator_name = db::users::get_display_name(&state.db, r.created_by)
                .await
                .ok();
            let database_name = db::databases::get_by_id(&state.db, r.database_id)
                .await
                .ok()
                .flatten()
                .map(|d| d.name);

            all_items.push(serde_json::json!({
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
                "approver_unit": unit,
                "expires_at": r.expires_at,
                "created_at": r.created_at,
            }));
        }
    }

    Ok(Json(serde_json::json!(all_items)))
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

    // Get user's approver units from DB
    let user_units = db::approver_units::get_user_units(&state.db, claims.user_id)
        .await
        .map_err(AppError::Database)?;

    if user_units.is_empty() {
        return Err(AppError::Forbidden);
    }

    // Check tracking: current level must match one of user's units
    let tracking = db::data_queries::get_approval_tracking(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    let current_tracking = tracking
        .iter()
        .find(|t| t.level_order == request.current_approval_level);

    match current_tracking {
        Some(t) if user_units.contains(&t.approver_unit.to_string()) => {}
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

// (determine_approver_unit removed — now uses DB-based user_approver_units)
