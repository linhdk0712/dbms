use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use std::net::SocketAddr;
use uuid::Uuid;
use validator::Validate;

use crate::api::ip_extractor::extract_client_ip;
use crate::auth::extractors::{RequireAdmin, RequireMaker};
use crate::db;
use crate::error::AppError;
use crate::executor::encryption::{decrypt_credentials, encrypt_credentials};
use crate::executor;
use crate::models::database::{DatabaseConnectionResponse, MakerDatabaseResponse, UpdateCredentialsBody};
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateDatabaseBody {
    pub name: String,
    pub db_type: String,
    pub host: String,
    pub port: i32,
    pub database_name: String,
    pub schema_name: Option<String>,
    pub username: String,
    pub password: String,
    pub timeout_secs: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDatabaseBody {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<i32>,
    pub database_name: Option<String>,
    pub schema_name: Option<String>,
    pub timeout_secs: Option<i32>,
    pub is_active: Option<bool>,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `POST /api/v1/databases` — Admin tạo database connection mới.
/// Credentials được encrypt trước khi lưu (BR-009).
pub async fn create_database(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Json(body): Json<CreateDatabaseBody>,
) -> Result<(StatusCode, Json<DatabaseConnectionResponse>), AppError> {
    // Validate db_type
    let valid_types = ["postgresql", "mysql", "oracle", "mongodb"];
    if !valid_types.contains(&body.db_type.as_str()) {
        return Err(AppError::ValidationError(format!(
            "db_type must be one of: {}",
            valid_types.join(", ")
        )));
    }

    // BR-009: Encrypt credentials
    let key_bytes = state.config.encryption_key_bytes()
        .map_err(|e| AppError::Encryption(e.to_string()))?;
    let encrypted = encrypt_credentials(&body.username, &body.password, &key_bytes)
        .map_err(|e| AppError::Encryption(e.to_string()))?;

    let timeout = body.timeout_secs.unwrap_or(30).min(300).max(1);

    let db_conn = db::databases::create(
        &state.db,
        &body.name,
        &body.db_type,
        &body.host,
        body.port,
        &body.database_name,
        body.schema_name.as_deref(),
        &encrypted,
        timeout,
    )
    .await
    .map_err(AppError::Database)?;

    // Response KHÔNG chứa credentials
    Ok((StatusCode::CREATED, Json(DatabaseConnectionResponse::from(db_conn))))
}

/// `GET /api/v1/databases` — Admin xem tất cả database connections.
pub async fn list_databases(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
) -> Result<Json<Vec<DatabaseConnectionResponse>>, AppError> {
    let connections = db::databases::list_all(&state.db)
        .await
        .map_err(AppError::Database)?;

    let responses: Vec<DatabaseConnectionResponse> =
        connections.into_iter().map(DatabaseConnectionResponse::from).collect();

    Ok(Json(responses))
}

/// `GET /api/v1/databases/:id` — Admin xem chi tiết database connection.
pub async fn get_database(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<Json<DatabaseConnectionResponse>, AppError> {
    let conn = db::databases::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    Ok(Json(DatabaseConnectionResponse::from(conn)))
}

/// `PATCH /api/v1/databases/:id` — Admin cập nhật database connection.
pub async fn update_database(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateDatabaseBody>,
) -> Result<Json<DatabaseConnectionResponse>, AppError> {
    let conn = db::databases::update(
        &state.db,
        id,
        body.name.as_deref(),
        body.host.as_deref(),
        body.port,
        body.database_name.as_deref(),
        body.schema_name.as_deref(),
        body.timeout_secs,
        body.is_active,
    )
    .await
    .map_err(AppError::Database)?;

    Ok(Json(DatabaseConnectionResponse::from(conn)))
}

/// `POST /api/v1/databases/:id/test` — Test kết nối đến database đích.
pub async fn test_connection(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let conn = db::databases::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let key_bytes = state.config.encryption_key_bytes()
        .map_err(|e| AppError::Encryption(e.to_string()))?;
    let credentials = decrypt_credentials(&conn.encrypted_credentials, &key_bytes)
        .map_err(|e| AppError::Encryption(e.to_string()))?;

    // Hiện tại support PostgreSQL và MySQL executor
    match executor::test_db_connection(
        &conn.db_type.to_string(),
        &credentials,
        &conn.host,
        conn.port,
        &conn.database_name,
    )
    .await
    {
        Ok(()) => Ok(Json(serde_json::json!({
            "status": "ok",
            "message": "connection successful"
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "status": "error",
            "message": e.to_string()
        }))),
    }
}

/// `PATCH /api/v1/databases/:id/credentials` — Admin rotate credentials cho database connection.
/// Encrypt new credentials bằng AES-256-GCM và ghi audit log "CREDENTIAL_ROTATED".
/// Không log credential values (Req 2.6).
pub async fn update_credentials(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    headers: HeaderMap,
    conn_info: ConnectInfo<SocketAddr>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateCredentialsBody>,
) -> Result<StatusCode, AppError> {
    // Validate body
    body.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Check database exists
    db::databases::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Encrypt new credentials
    let key_bytes = state.config.encryption_key_bytes()
        .map_err(|e| AppError::Encryption(e.to_string()))?;
    let encrypted = encrypt_credentials(&body.username, &body.password, &key_bytes)
        .map_err(|e| AppError::Encryption(e.to_string()))?;

    // Update credentials in DB
    db::databases::update_credentials(&state.db, id, &encrypted)
        .await
        .map_err(AppError::Database)?;

    // Write audit log — không log credential values
    let ip = extract_client_ip(&headers, &conn_info);
    let detail = format!("database_id: {id}");
    db::audit::insert_admin_action(
        &state.db,
        "CREDENTIAL_ROTATED",
        claims.user_id,
        &detail,
        &ip,
    )
    .await
    .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/databases/my-permissions` — Maker xem databases mình có quyền.
pub async fn list_my_databases(
    State(state): State<AppState>,
    RequireMaker(claims): RequireMaker,
) -> Result<Json<Vec<MakerDatabaseResponse>>, AppError> {
    let databases = db::databases::list_by_user_permission(&state.db, claims.user_id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(databases))
}
