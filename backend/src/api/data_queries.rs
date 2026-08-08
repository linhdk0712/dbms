use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;
use uuid::Uuid;
use validator::Validate;

use crate::api::ip_extractor::extract_client_ip;
use crate::auth::extractors::RequireAuth;
use crate::db;
use crate::error::AppError;
use crate::executor::encryption::decrypt_credentials;
use crate::state::AppState;

/// Max rows trả về cho data query.
const MAX_QUERY_ROWS: i64 = 10_000;

// ── DTOs ───────────────────────────────────────────────────────────────

/// Request body for creating a new data query request.
///
/// Supports both Basic mode (UI builder with table/columns/filters) and
/// Advanced mode (free-form SQL). The `query_mode` field determines which
/// optional fields are relevant.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDataQueryBody {
    #[validate(length(min = 3, max = 500))]
    pub title: String,
    pub database_id: Uuid,
    #[validate(length(max = 200))]
    pub schema_name: String,
    pub query_mode: String,
    #[validate(length(min = 5, max = 50000))]
    pub sql_query: String,
    #[validate(length(min = 1, max = 100))]
    pub ticket_id: String,
    #[serde(default)]
    pub assigned_user_ids: Vec<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    // Basic mode extras (optional)
    pub table_name: Option<String>,
    pub columns: Option<Vec<String>>,
    pub filters: Option<serde_json::Value>,
    pub limit: Option<i64>,
}

/// Request body for updating a data query request in DRAFT status.
///
/// All fields are optional — only provided fields will be updated.
#[derive(Debug, Deserialize)]
pub struct UpdateDataQueryBody {
    pub title: Option<String>,
    pub schema_name: Option<String>,
    pub sql_query: Option<String>,
    pub query_mode: Option<String>,
    pub ticket_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub tables_accessed: Option<Vec<String>>,
    pub columns_accessed: Option<Vec<String>>,
}

/// Query parameters for listing data queries with optional filtering and pagination.
#[derive(Debug, Deserialize)]
pub struct ListQueryParams {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Paginated response for data query listing.
#[derive(Debug, Serialize)]
pub struct DataQueryListResponse {
    pub data: Vec<DataQueryListItem>,
    pub pagination: PaginationMeta,
}

/// Summary item for a data query in list views.
#[derive(Debug, Serialize)]
pub struct DataQueryListItem {
    pub id: Uuid,
    pub title: String,
    pub database_id: Uuid,
    pub database_name: Option<String>,
    pub schema_name: String,
    pub query_mode: String,
    pub ticket_id: String,
    pub status: String,
    pub created_by: Uuid,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Pagination metadata included in paginated responses.
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

/// Full detail response for a single data query request, including
/// approval tracking and assigned users.
#[derive(Debug, Serialize)]
pub struct DataQueryDetailResponse {
    pub id: Uuid,
    pub title: String,
    pub database_id: Uuid,
    pub database_name: Option<String>,
    pub schema_name: String,
    pub sql_query: String,
    pub tables_accessed: Vec<String>,
    pub columns_accessed: Vec<String>,
    pub query_mode: String,
    pub ticket_id: String,
    pub status: String,
    pub created_by: Uuid,
    pub creator_name: Option<String>,
    pub current_approval_level: i32,
    pub total_approval_levels: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub assigned_users: Vec<AssignedUserInfo>,
    pub approval_tracking: Vec<ApprovalTrackingInfo>,
}

/// Information about a user assigned to run a data query.
#[derive(Debug, Serialize)]
pub struct AssignedUserInfo {
    pub user_id: Uuid,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

/// Tracks the status of a single approval level for a data query request.
#[derive(Debug, Serialize)]
pub struct ApprovalTrackingInfo {
    pub level_order: i32,
    pub approver_unit: String,
    pub status: String,
    pub approved_by: Option<Uuid>,
    pub approved_by_name: Option<String>,
    pub comment: Option<String>,
    pub acted_at: Option<DateTime<Utc>>,
}

/// Result of executing an approved data query against the target database.
///
/// Contains the column names, row data, timing info, and whether the result
/// was truncated due to the row limit.
#[derive(Debug, Serialize)]
pub struct QueryExecutionResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub rows_returned: i64,
    pub execution_time_ms: i64,
    pub truncated: bool,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `POST /api/v1/data-queries` — Tạo data query request mới.
///
/// Tạo một request ở trạng thái DRAFT. Maker có thể assign thêm users
/// được phép run query này sau khi được approve.
///
/// # Errors
/// - `ValidationError` nếu body không hợp lệ.
/// - `Database` nếu insert thất bại.
pub async fn create(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Json(body): Json<CreateDataQueryBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    body.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Extract tables/columns from SQL (basic — sẽ parse kỹ hơn sau)
    let tables_accessed: Vec<String> = body.table_name
        .as_ref()
        .map(|t| vec![t.clone()])
        .unwrap_or_default();
    let columns_accessed: Vec<String> = body.columns.clone().unwrap_or_default();

    let request = db::data_queries::create_request(
        &state.db,
        &body.title,
        body.database_id,
        &body.schema_name,
        &body.sql_query,
        &tables_accessed,
        &columns_accessed,
        &body.query_mode,
        &body.ticket_id,
        claims.user_id,
        body.expires_at,
    )
    .await
    .map_err(AppError::Database)?;

    // Insert assigned users
    if !body.assigned_user_ids.is_empty() {
        db::data_queries::insert_assigned_users(&state.db, request.id, &body.assigned_user_ids)
            .await
            .map_err(AppError::Database)?;
    }

    Ok(Json(serde_json::json!({
        "id": request.id,
        "status": request.status.to_string(),
        "message": "Data query created as DRAFT"
    })))
}

/// `GET /api/v1/data-queries` — List data queries (user sees own + assigned).
///
/// Trả về danh sách phân trang các query mà user hiện tại là creator hoặc
/// được assign. Hỗ trợ filter theo status.
///
/// # Errors
/// - `Database` nếu truy vấn thất bại.
pub async fn list(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Query(params): Query<ListQueryParams>,
) -> Result<Json<DataQueryListResponse>, AppError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let total = db::data_queries::count_by_user(
        &state.db,
        claims.user_id,
        params.status.as_deref(),
    )
    .await
    .map_err(AppError::Database)?;

    let requests = db::data_queries::list_by_user(
        &state.db,
        claims.user_id,
        params.status.as_deref(),
        per_page,
        offset,
    )
    .await
    .map_err(AppError::Database)?;

    let mut items = Vec::with_capacity(requests.len());
    for r in &requests {
        let db_name = db::databases::get_by_id(&state.db, r.database_id)
            .await
            .ok()
            .flatten()
            .map(|d| d.name);

        items.push(DataQueryListItem {
            id: r.id,
            title: r.title.clone(),
            database_id: r.database_id,
            database_name: db_name,
            schema_name: r.schema_name.clone(),
            query_mode: r.query_mode.to_string(),
            ticket_id: r.ticket_id.clone(),
            status: r.status.to_string(),
            created_by: r.created_by,
            expires_at: r.expires_at,
            created_at: r.created_at,
        });
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(DataQueryListResponse {
        data: items,
        pagination: PaginationMeta { page, per_page, total, total_pages },
    }))
}

/// `GET /api/v1/data-queries/:id` — Chi tiết một data query request.
///
/// Trả về thông tin đầy đủ bao gồm SQL, approval tracking, và assigned users.
/// Chỉ creator, assigned users, hoặc Admin mới có quyền xem.
///
/// # Errors
/// - `NotFound` nếu request không tồn tại.
/// - `Forbidden` nếu user không có quyền truy cập.
pub async fn get_detail(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<DataQueryDetailResponse>, AppError> {
    let request = db::data_queries::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Check access: creator OR assigned
    if request.created_by != claims.user_id {
        let assigned = db::data_queries::is_user_assigned(&state.db, id, claims.user_id)
            .await
            .map_err(AppError::Database)?;
        if !assigned {
            // Admin can also view
            if !claims.is_admin() {
                return Err(AppError::Forbidden);
            }
        }
    }

    // Get database name
    let db_name = db::databases::get_by_id(&state.db, request.database_id)
        .await
        .ok()
        .flatten()
        .map(|d| d.name);

    // Get approval tracking
    let tracking = db::data_queries::get_approval_tracking(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    let tracking_info: Vec<ApprovalTrackingInfo> = tracking
        .iter()
        .map(|t| ApprovalTrackingInfo {
            level_order: t.level_order,
            approver_unit: t.approver_unit.to_string(),
            status: t.status.to_string(),
            approved_by: t.approved_by,
            approved_by_name: None, // Could populate from users table
            comment: t.comment.clone(),
            acted_at: t.acted_at,
        })
        .collect();

    Ok(Json(DataQueryDetailResponse {
        id: request.id,
        title: request.title,
        database_id: request.database_id,
        database_name: db_name,
        schema_name: request.schema_name,
        sql_query: request.sql_query,
        tables_accessed: request.tables_accessed,
        columns_accessed: request.columns_accessed,
        query_mode: request.query_mode.to_string(),
        ticket_id: request.ticket_id,
        status: request.status.to_string(),
        created_by: request.created_by,
        creator_name: db::users::get_display_name(&state.db, request.created_by).await.ok(),
        current_approval_level: request.current_approval_level,
        total_approval_levels: request.total_approval_levels,
        expires_at: request.expires_at,
        created_at: request.created_at,
        approved_at: request.approved_at,
        assigned_users: get_assigned_user_infos(&state.db, id).await,
        approval_tracking: tracking_info,
    }))
}

/// `PUT /api/v1/data-queries/:id` — Cập nhật data query đang ở trạng thái DRAFT.
///
/// Chỉ creator mới được update, và request phải đang ở DRAFT.
/// Các field không gửi sẽ giữ nguyên giá trị cũ.
///
/// # Errors
/// - `NotFound` nếu request không tồn tại.
/// - `Forbidden` nếu user không phải creator.
/// - `ImmutableRequest` nếu request đã rời khỏi DRAFT.
pub async fn update(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateDataQueryBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    let request = db::data_queries::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if request.created_by != claims.user_id {
        return Err(AppError::Forbidden);
    }

    if request.status != crate::models::data_query::DataQueryStatus::Draft {
        return Err(AppError::ImmutableRequest);
    }

    let updated = db::data_queries::update_draft(
        &state.db,
        id,
        body.title.as_deref().unwrap_or(&request.title),
        body.schema_name.as_deref().unwrap_or(&request.schema_name),
        body.sql_query.as_deref().unwrap_or(&request.sql_query),
        body.tables_accessed.as_deref().unwrap_or(&request.tables_accessed),
        body.columns_accessed.as_deref().unwrap_or(&request.columns_accessed),
        body.query_mode.as_deref().unwrap_or(&request.query_mode.to_string()),
        body.ticket_id.as_deref().unwrap_or(&request.ticket_id),
        body.expires_at.or(request.expires_at),
    )
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "id": updated.id,
        "status": updated.status.to_string(),
        "message": "Data query updated"
    })))
}

/// `POST /api/v1/data-queries/:id/submit` — Submit data query cho approval flow.
///
/// Chuyển trạng thái DRAFT → PENDING_APPROVAL. Hệ thống tự động resolve
/// approval matrix dựa trên tables_accessed để xác định chuỗi phê duyệt.
///
/// # Errors
/// - `NotFound` nếu request không tồn tại.
/// - `Forbidden` nếu user không phải creator.
/// - `InvalidStateTransition` nếu request không ở trạng thái DRAFT.
pub async fn submit(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let request = db::data_queries::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    if request.created_by != claims.user_id {
        return Err(AppError::Forbidden);
    }

    if request.status != crate::models::data_query::DataQueryStatus::Draft {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "PENDING_APPROVAL".to_string(),
        });
    }

    // Resolve approval levels based on tables accessed
    let tables = if request.tables_accessed.is_empty() {
        vec!["*".to_string()]
    } else {
        request.tables_accessed.clone()
    };

    let approval_levels = db::data_query_rules::resolve_approval_levels(
        &state.db,
        request.database_id,
        &request.schema_name,
        &tables,
    )
    .await
    .map_err(AppError::Database)?;

    let submitted = db::data_queries::submit_request(&state.db, id, &approval_levels)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "id": submitted.id,
        "status": submitted.status.to_string(),
        "total_approval_levels": submitted.total_approval_levels,
        "message": "Data query submitted for approval"
    })))
}

/// `POST /api/v1/data-queries/:id/execute` — Thực thi query đã được approve.
///
/// Chỉ creator hoặc assigned users mới được run. Query phải ở trạng thái
/// APPROVED và chưa hết hạn. Kết quả được giới hạn tối đa [`MAX_QUERY_ROWS`] rows.
/// Mỗi lần execute đều được ghi audit log (thành công hoặc thất bại).
///
/// # Errors
/// - `NotFound` nếu request không tồn tại.
/// - `InvalidStateTransition` nếu request không ở APPROVED.
/// - `ValidationError` nếu query đã hết hạn.
/// - `Forbidden` nếu user không có quyền run.
/// - `Internal` nếu thực thi SQL thất bại.
pub async fn execute(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<Uuid>,
) -> Result<Json<QueryExecutionResult>, AppError> {
    let request = db::data_queries::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Check status = APPROVED
    if request.status != crate::models::data_query::DataQueryStatus::Approved {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "EXECUTE".to_string(),
        });
    }

    // Check expiration
    if let Some(expires) = request.expires_at {
        if expires < Utc::now() {
            return Err(AppError::ValidationError("Query has expired".to_string()));
        }
    }

    // Check user is creator OR assigned
    if request.created_by != claims.user_id {
        let assigned = db::data_queries::is_user_assigned(&state.db, id, claims.user_id)
            .await
            .map_err(AppError::Database)?;
        if !assigned {
            return Err(AppError::Forbidden);
        }
    }

    let ip = extract_client_ip(&headers, &ConnectInfo(addr));
    let start = std::time::Instant::now();

    // Get database connection
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

    // Execute query (read-only)
    let timeout = Duration::from_secs(
        (db_conn.timeout_secs as u64).min(state.config.sql_execution_timeout_secs),
    );

    let result = crate::executor::execute_query_readonly(
        &db_conn.db_type.to_string(),
        &request.sql_query,
        &credentials,
        &db_conn.host,
        db_conn.port,
        &db_conn.database_name,
        db_conn.schema_name.as_deref(),
        timeout,
        MAX_QUERY_ROWS,
    )
    .await;

    let elapsed_ms = start.elapsed().as_millis() as i64;

    match result {
        Ok(query_result) => {
            // Insert execution audit
            let _ = db::data_queries::insert_execution(
                &state.db,
                id,
                claims.user_id,
                query_result.rows_returned,
                elapsed_ms,
                Some(&ip),
                "SUCCESS",
                None,
            )
            .await;

            Ok(Json(QueryExecutionResult {
                columns: query_result.columns,
                rows: query_result.rows,
                rows_returned: query_result.rows_returned,
                execution_time_ms: elapsed_ms,
                truncated: query_result.truncated,
            }))
        }
        Err(e) => {
            let error_msg = e.to_string();
            let _ = db::data_queries::insert_execution(
                &state.db,
                id,
                claims.user_id,
                0,
                elapsed_ms,
                Some(&ip),
                "FAILED",
                Some(&error_msg),
            )
            .await;

            Err(AppError::Internal(anyhow::anyhow!("Query execution failed: {}", error_msg)))
        }
    }
}

/// `GET /api/v1/data-queries/:id/executions` — Lịch sử thực thi của một query.
///
/// Trả về danh sách các lần execute (ai run, khi nào, bao nhiêu rows, thời gian).
/// Chỉ creator hoặc assigned users mới có quyền xem.
///
/// # Errors
/// - `NotFound` nếu request không tồn tại.
/// - `Forbidden` nếu user không có quyền truy cập.
pub async fn list_executions(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<crate::models::data_query::DataQueryExecution>>, AppError> {
    let request = db::data_queries::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Check access
    if request.created_by != claims.user_id {
        let assigned = db::data_queries::is_user_assigned(&state.db, id, claims.user_id)
            .await
            .map_err(AppError::Database)?;
        if !assigned {
            return Err(AppError::Forbidden);
        }
    }

    let executions = db::data_queries::list_executions(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(executions))
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Lấy danh sách assigned users kèm display_name/email cho một query request.
async fn get_assigned_user_infos(pool: &sqlx::PgPool, query_request_id: Uuid) -> Vec<AssignedUserInfo> {
    #[derive(sqlx::FromRow)]
    struct AssignedRow {
        user_id: Uuid,
        display_name: Option<String>,
        email: Option<String>,
    }

    let rows = sqlx::query_as::<_, AssignedRow>(
        r#"SELECT dqau.user_id, u.display_name, u.email
           FROM data_query_assigned_users dqau
           JOIN users u ON u.id = dqau.user_id
           WHERE dqau.query_request_id = $1"#,
    )
    .bind(query_request_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| AssignedUserInfo {
            user_id: r.user_id,
            display_name: r.display_name,
            email: r.email,
        })
        .collect()
}
