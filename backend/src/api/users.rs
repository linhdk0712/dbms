use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::api::ip_extractor::extract_client_ip;
use crate::auth::extractors::RequireAdmin;
use crate::db;
use crate::error::AppError;
use crate::models::user::UserDbPermission;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpdateUserBody {
    pub is_active: Option<bool>,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct UserDetailResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub is_active: bool,
    pub roles: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GrantPermissionBody {
    pub database_id: Uuid,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/users` — Admin xem danh sách users.
pub async fn list_users(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
) -> Result<Json<Vec<UserDetailResponse>>, AppError> {
    let users = db::users::list_all(&state.db)
        .await
        .map_err(AppError::Database)?;

    let mut responses = Vec::with_capacity(users.len());
    for u in &users {
        let roles = db::users::get_user_roles(&state.db, u.id)
            .await
            .map_err(AppError::Database)?;
        responses.push(UserDetailResponse {
            id: u.id,
            email: u.email.clone(),
            display_name: u.display_name.clone(),
            is_active: u.is_active,
            roles: roles.iter().map(|r| r.to_string()).collect(),
            created_at: u.created_at.to_rfc3339(),
        });
    }

    Ok(Json(responses))
}

/// `GET /api/v1/users/:id` — Admin xem chi tiết user.
pub async fn get_user(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserDetailResponse>, AppError> {
    let user = db::users::get_by_id(&state.db, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let roles = db::users::get_user_roles(&state.db, user.id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(UserDetailResponse {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        is_active: user.is_active,
        roles: roles.iter().map(|r| r.to_string()).collect(),
        created_at: user.created_at.to_rfc3339(),
    }))
}

/// `PATCH /api/v1/users/:id` — Admin cập nhật user: activate/deactivate, gán roles.
pub async fn update_user(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    headers: HeaderMap,
    conn_info: ConnectInfo<SocketAddr>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UpdateUserBody>,
) -> Result<Json<UserDetailResponse>, AppError> {
    // Không cho phép admin tự deactivate hoặc tự xoá role ADMIN của mình
    if user_id == claims.user_id {
        if let Some(false) = body.is_active {
            return Err(AppError::ValidationError("Cannot deactivate yourself".into()));
        }
        if let Some(ref roles) = body.roles {
            if !roles.iter().any(|r| r == "ADMIN") {
                return Err(AppError::ValidationError("Cannot remove your own ADMIN role".into()));
            }
        }
    }

    // ── Capture old state BEFORE updates ──────────────────────────────
    let old_active = db::users::is_user_active(&state.db, user_id)
        .await
        .map_err(AppError::Database)?;

    let old_roles = db::users::get_user_roles(&state.db, user_id)
        .await
        .map_err(AppError::Database)?;

    // Update is_active nếu có
    if let Some(is_active) = body.is_active {
        db::users::update_active_status(&state.db, user_id, is_active)
            .await
            .map_err(AppError::Database)?;
    }

    // Update roles nếu có
    if let Some(ref roles) = body.roles {
        // Validate role names
        let valid_roles = ["ADMIN", "MAKER", "CHECKER"];
        for role in roles {
            if !valid_roles.contains(&role.as_str()) {
                return Err(AppError::ValidationError(format!("Invalid role: {role}")));
            }
        }

        db::users::replace_roles(&state.db, user_id, roles, claims.user_id)
            .await
            .map_err(AppError::Database)?;
    }

    // ── Audit logging — compare old vs new ────────────────────────────
    let ip = extract_client_ip(&headers, &conn_info);

    // Check role change (only if roles were submitted)
    if body.roles.is_some() {
        let new_roles = db::users::get_user_roles(&state.db, user_id)
            .await
            .map_err(AppError::Database)?;

        // Sort both for comparison (avoid false positives from order differences)
        let mut old_sorted: Vec<String> = old_roles.iter().map(|r| r.to_string()).collect();
        old_sorted.sort();
        let mut new_sorted: Vec<String> = new_roles.iter().map(|r| r.to_string()).collect();
        new_sorted.sort();

        if old_sorted != new_sorted {
            let detail = format!(
                "old_roles: [{}], new_roles: [{}]",
                old_sorted.join(", "),
                new_sorted.join(", ")
            );
            db::audit::insert_admin_action(
                &state.db,
                "ROLE_CHANGED",
                claims.user_id,
                &detail,
                &ip,
            )
            .await
            .map_err(AppError::Database)?;
        }
    }

    // Check status change (only if is_active was submitted)
    if let Some(new_active) = body.is_active {
        if old_active != new_active {
            let old_status = if old_active { "active" } else { "inactive" };
            let new_status = if new_active { "active" } else { "inactive" };
            let detail = format!("old_status: {old_status}, new_status: {new_status}");
            db::audit::insert_admin_action(
                &state.db,
                "USER_STATUS_CHANGED",
                claims.user_id,
                &detail,
                &ip,
            )
            .await
            .map_err(AppError::Database)?;
        }
    }

    // Trả về user sau khi update
    let user = db::users::get_by_id(&state.db, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let updated_roles = db::users::get_user_roles(&state.db, user.id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(UserDetailResponse {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        is_active: user.is_active,
        roles: updated_roles.iter().map(|r| r.to_string()).collect(),
        created_at: user.created_at.to_rfc3339(),
    }))
}

/// `GET /api/v1/users/:id/permissions` — Admin xem quyền của user.
pub async fn get_user_permissions(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<UserDbPermission>>, AppError> {
    let permissions = db::permissions::list_user_permissions(&state.db, user_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(permissions))
}

/// `POST /api/v1/users/:user_id/permissions` — Admin cấp quyền cho user trên database.
pub async fn grant_permission(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Path(user_id): Path<Uuid>,
    Json(body): Json<GrantPermissionBody>,
) -> Result<(StatusCode, Json<UserDbPermission>), AppError> {
    // Kiểm tra user tồn tại và active
    let is_active = db::users::is_user_active(&state.db, user_id)
        .await
        .map_err(AppError::Database)?;
    if !is_active {
        return Err(AppError::NotFound);
    }

    // Kiểm tra database tồn tại
    let db_conn = db::databases::get_by_id(&state.db, body.database_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;
    if !db_conn.is_active {
        return Err(AppError::NotFound);
    }

    // Cấp quyền
    let permission = db::permissions::grant_permission(
        &state.db,
        user_id,
        body.database_id,
        claims.user_id,
    )
    .await
    .map_err(|e| {
        // UNIQUE constraint violation → 409 Conflict
        if e.to_string().contains("duplicate key") || e.to_string().contains("unique") {
            AppError::ValidationError("permission already exists".into())
        } else {
            AppError::Database(e)
        }
    })?;

    Ok((StatusCode::CREATED, Json(permission)))
}

/// `DELETE /api/v1/users/:user_id/permissions/:database_id` — Admin thu hồi quyền.
pub async fn revoke_permission(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path((user_id, database_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let revoked = db::permissions::revoke_permission(&state.db, user_id, database_id)
        .await
        .map_err(AppError::Database)?;

    if !revoked {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}
