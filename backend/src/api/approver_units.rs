//! Approver Units API — quản lý đơn vị phê duyệt và gán user.
//!
//! Endpoints:
//! - `GET /api/v1/approver-units` — List tất cả units
//! - `POST /api/v1/approver-units` — Tạo unit mới (Admin)
//! - `PUT /api/v1/approver-units/:id` — Cập nhật unit (Admin)
//! - `DELETE /api/v1/approver-units/:id` — Xóa unit (Admin)
//! - `GET /api/v1/users/:user_id/approver-units` — Units của user
//! - `POST /api/v1/users/:user_id/approver-units` — Gán unit cho user (Admin)
//! - `DELETE /api/v1/users/:user_id/approver-units/:unit_id` — Bỏ gán (Admin)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::extractors::{RequireAdmin, RequireApprover, RequireAuth};
use crate::db;
use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateUnitBody {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUnitBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct AssignUnitBody {
    pub unit_id: Uuid,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/approver-units` — List all approver units.
/// Accessible by all authenticated users (for dropdown population).
pub async fn list_units(
    State(state): State<AppState>,
    RequireAuth(_claims): RequireAuth,
) -> Result<Json<Vec<db::approver_units::ApproverUnit>>, AppError> {
    let units = db::approver_units::list_all(&state.db)
        .await
        .map_err(AppError::Database)?;
    Ok(Json(units))
}

/// `POST /api/v1/approver-units` — Create new approver unit (Approver/Admin only).
pub async fn create_unit(
    State(state): State<AppState>,
    RequireApprover(_claims): RequireApprover,
    Json(body): Json<CreateUnitBody>,
) -> Result<(StatusCode, Json<db::approver_units::ApproverUnit>), AppError> {
    if body.name.trim().is_empty() || body.name.len() > 50 {
        return Err(AppError::ValidationError(
            "Unit name must be 1-50 characters".to_string(),
        ));
    }

    let unit = db::approver_units::create(
        &state.db,
        body.name.trim(),
        body.description.as_deref(),
    )
    .await
    .map_err(|e| {
        if e.to_string().contains("duplicate key") {
            AppError::ValidationError(format!("Unit '{}' already exists", body.name))
        } else {
            AppError::Database(e)
        }
    })?;

    Ok((StatusCode::CREATED, Json(unit)))
}

/// `PUT /api/v1/approver-units/:id` — Update approver unit (Approver/Admin only).
pub async fn update_unit(
    State(state): State<AppState>,
    RequireApprover(_claims): RequireApprover,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateUnitBody>,
) -> Result<Json<db::approver_units::ApproverUnit>, AppError> {
    let unit = db::approver_units::update(
        &state.db,
        id,
        body.name.as_deref(),
        body.description.as_deref(),
        body.is_active,
    )
    .await
    .map_err(AppError::Database)?;

    Ok(Json(unit))
}

/// `DELETE /api/v1/approver-units/:id` — Delete approver unit (Approver/Admin only).
pub async fn delete_unit(
    State(state): State<AppState>,
    RequireApprover(_claims): RequireApprover,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    db::approver_units::delete(&state.db, id)
        .await
        .map_err(AppError::Database)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/users/:user_id/approver-units` — Get user's approver units.
pub async fn get_user_units(
    State(state): State<AppState>,
    RequireAuth(_claims): RequireAuth,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<db::approver_units::ApproverUnit>>, AppError> {
    let units = db::approver_units::get_user_unit_details(&state.db, user_id)
        .await
        .map_err(AppError::Database)?;
    Ok(Json(units))
}

/// `POST /api/v1/users/:user_id/approver-units` — Assign unit to user (Admin only).
pub async fn assign_unit(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Path(user_id): Path<Uuid>,
    Json(body): Json<AssignUnitBody>,
) -> Result<StatusCode, AppError> {
    db::approver_units::assign_user(&state.db, user_id, body.unit_id, claims.user_id)
        .await
        .map_err(AppError::Database)?;
    Ok(StatusCode::CREATED)
}

/// `DELETE /api/v1/users/:user_id/approver-units/:unit_id` — Unassign unit (Admin only).
pub async fn unassign_unit(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path((user_id, unit_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    db::approver_units::unassign_user(&state.db, user_id, unit_id)
        .await
        .map_err(AppError::Database)?;
    Ok(StatusCode::NO_CONTENT)
}
