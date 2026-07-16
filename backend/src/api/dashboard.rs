use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::extractors::RequireAuth;
use crate::error::AppError;
use crate::models::user::Role;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_requests: i64,
    pub pending_requests: i64,
    pub approved_requests: i64,
    pub executed_requests: i64,
    pub failed_requests: i64,
    pub rejected_requests: i64,
    pub cancelled_requests: i64,
    // Checker-specific fields
    pub pending_for_review: Option<i64>,
    pub total_reviewed: Option<i64>,
    pub total_approved_by_me: Option<i64>,
    pub total_rejected_by_me: Option<i64>,
}

impl DashboardStats {
    /// Returns an empty stats response (all zeros, no role-specific data).
    /// Used when user has no assigned roles.
    pub fn empty() -> Self {
        Self {
            total_requests: 0,
            pending_requests: 0,
            approved_requests: 0,
            executed_requests: 0,
            failed_requests: 0,
            rejected_requests: 0,
            cancelled_requests: 0,
            pending_for_review: None,
            total_reviewed: None,
            total_approved_by_me: None,
            total_rejected_by_me: None,
        }
    }
}

// ── Role Priority ──────────────────────────────────────────────────────

/// Determine the effective (highest-privilege) role from a list of roles.
/// Priority: Admin > Checker > Maker.
/// Returns None if roles is empty.
fn determine_effective_role(roles: &[Role]) -> Option<&Role> {
    if roles.contains(&Role::Admin) {
        Some(&Role::Admin)
    } else if roles.contains(&Role::Checker) {
        Some(&Role::Checker)
    } else if roles.contains(&Role::Maker) {
        Some(&Role::Maker)
    } else {
        None
    }
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/dashboard` — Role-aware dashboard statistics.
/// - Admin: global system-wide stats
/// - Checker: pending for review (excluding own) + approval history
/// - Maker: own requests only
/// - No role: empty stats (all zeros)
pub async fn get_stats(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
) -> Result<Json<DashboardStats>, AppError> {
    match determine_effective_role(&claims.roles) {
        Some(Role::Admin) => get_admin_stats(&state.db).await,
        Some(Role::Checker) => get_checker_stats(&state.db, claims.user_id).await,
        Some(Role::Maker) => get_maker_stats(&state.db, claims.user_id).await,
        None => Ok(Json(DashboardStats::empty())),
    }
}

// ── Role-specific stats functions ──────────────────────────────────────

/// Admin stats: global counts across all requests.
async fn get_admin_stats(pool: &PgPool) -> Result<Json<DashboardStats>, AppError> {
    let stats: (i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"SELECT
               COUNT(*) as total,
               COUNT(*) FILTER (WHERE status = 'PENDING') as pending,
               COUNT(*) FILTER (WHERE status = 'APPROVED') as approved,
               COUNT(*) FILTER (WHERE status = 'EXECUTED') as executed,
               COUNT(*) FILTER (WHERE status = 'FAILED') as failed,
               COUNT(*) FILTER (WHERE status = 'REJECTED') as rejected,
               COUNT(*) FILTER (WHERE status = 'CANCELLED') as cancelled
           FROM sql_requests"#,
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(DashboardStats {
        total_requests: stats.0,
        pending_requests: stats.1,
        approved_requests: stats.2,
        executed_requests: stats.3,
        failed_requests: stats.4,
        rejected_requests: stats.5,
        cancelled_requests: stats.6,
        pending_for_review: None,
        total_reviewed: None,
        total_approved_by_me: None,
        total_rejected_by_me: None,
    }))
}

/// Checker stats:
/// - pending_for_review: pending requests excluding own
/// - total_reviewed: total approvals + rejections by this checker
/// - total_approved_by_me: approvals from request_approvals
/// - total_rejected_by_me: rejections from audit_log (action = 'REJECTED')
/// Main stats fields are set to 0.
async fn get_checker_stats(pool: &PgPool, user_id: Uuid) -> Result<Json<DashboardStats>, AppError> {
    // Count pending requests excluding checker's own
    let (pending_for_review,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*)
           FROM sql_requests
           WHERE status = 'PENDING' AND created_by != $1"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    // Count total approved by this checker (from request_approvals)
    let (total_approved,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*)
           FROM request_approvals
           WHERE approver_id = $1"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    // Count total rejected by this checker (from audit_log)
    let (total_rejected,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*)
           FROM audit_log
           WHERE action = 'REJECTED' AND actor_id = $1"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    let total_reviewed = total_approved + total_rejected;

    Ok(Json(DashboardStats {
        total_requests: 0,
        pending_requests: 0,
        approved_requests: 0,
        executed_requests: 0,
        failed_requests: 0,
        rejected_requests: 0,
        cancelled_requests: 0,
        pending_for_review: Some(pending_for_review),
        total_reviewed: Some(total_reviewed),
        total_approved_by_me: Some(total_approved),
        total_rejected_by_me: Some(total_rejected),
    }))
}

/// Maker stats: same counters as admin but filtered to own requests only.
/// Checker-specific fields remain None.
async fn get_maker_stats(pool: &PgPool, user_id: Uuid) -> Result<Json<DashboardStats>, AppError> {
    let stats: (i64, i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"SELECT
               COUNT(*) as total,
               COUNT(*) FILTER (WHERE status = 'PENDING') as pending,
               COUNT(*) FILTER (WHERE status = 'APPROVED') as approved,
               COUNT(*) FILTER (WHERE status = 'EXECUTED') as executed,
               COUNT(*) FILTER (WHERE status = 'FAILED') as failed,
               COUNT(*) FILTER (WHERE status = 'REJECTED') as rejected,
               COUNT(*) FILTER (WHERE status = 'CANCELLED') as cancelled
           FROM sql_requests
           WHERE created_by = $1"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Json(DashboardStats {
        total_requests: stats.0,
        pending_requests: stats.1,
        approved_requests: stats.2,
        executed_requests: stats.3,
        failed_requests: stats.4,
        rejected_requests: stats.5,
        cancelled_requests: stats.6,
        pending_for_review: None,
        total_reviewed: None,
        total_approved_by_me: None,
        total_rejected_by_me: None,
    }))
}
