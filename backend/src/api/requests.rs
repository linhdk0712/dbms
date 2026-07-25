use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::auth::extractors::RequireMaker;
use crate::db;
use crate::error::AppError;
use crate::models::notification::{Notification, NotificationType};
use crate::models::request::RequestStatus;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

/// Body cho endpoint tạo SQL request mới (`POST /api/v1/requests`).
#[derive(Debug, Deserialize, Validate)]
pub struct CreateRequestBody {
    #[validate(length(min = 1, max = 500, message = "description must be 1-500 characters"))]
    pub description: String,
    #[validate(length(min = 1, max = 50000, message = "sql_content must be 1-50000 characters"))]
    pub sql_content: String,
    pub database_id: Uuid,
    /// Nếu true → status PENDING ngay, nếu false → DRAFT
    #[serde(default)]
    pub submit: bool,
    /// URL từ ManageEngine ServiceDesk Plus (optional).
    /// Nếu cung cấp, request sẽ được link với CR tương ứng.
    pub cr_url: Option<String>,
}

/// Body cho endpoint cập nhật request đang DRAFT (`PATCH /api/v1/requests/:id`).
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateRequestBody {
    #[validate(length(min = 1, max = 500, message = "description must be 1-500 characters"))]
    pub description: String,
    #[validate(length(min = 1, max = 50000, message = "sql_content must be 1-50000 characters"))]
    pub sql_content: String,
}

/// Query parameters cho endpoint danh sách requests (`GET /api/v1/requests`).
#[derive(Debug, Deserialize)]
pub struct ListRequestsQuery {
    pub status: Option<String>,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

/// Giá trị mặc định cho page number (bắt đầu từ 1).
fn default_page() -> i64 { 1 }

/// Giá trị mặc định cho số lượng items trên mỗi trang.
fn default_per_page() -> i64 { 20 }

/// Response DTO cho request trong danh sách (không bao gồm `sql_content`).
#[derive(Debug, Serialize)]
pub struct RequestResponse {
    pub id: Uuid,
    pub description: String,
    pub status: RequestStatus,
    pub database_id: Uuid,
    pub created_by: Uuid,
    pub approved_by: Option<Uuid>,
    pub rejection_reason: Option<String>,
    pub rows_affected: Option<i64>,
    pub execution_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub submitted_at: Option<String>,
}

/// Response DTO cho chi tiết request (bao gồm `sql_content`).
/// Chỉ trả về cho Maker sở hữu request.
#[derive(Debug, Serialize)]
pub struct RequestDetailResponse {
    pub id: Uuid,
    pub description: String,
    pub sql_content: String,
    pub status: RequestStatus,
    pub database_id: Uuid,
    pub created_by: Uuid,
    pub approved_by: Option<Uuid>,
    pub rejection_reason: Option<String>,
    pub rows_affected: Option<i64>,
    pub execution_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub submitted_at: Option<String>,
}

/// Wrapper response cho danh sách có phân trang.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

/// Metadata phân trang: page hiện tại, tổng số items, tổng số pages.
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `POST /api/v1/requests` — Maker tạo SQL request mới.
///
/// BR-006: Kiểm tra permission trên database trước khi tạo.
/// BR-005: Ghi audit log trong cùng flow.
pub async fn create_request(
    State(state): State<AppState>,
    RequireMaker(claims): RequireMaker,
    Json(body): Json<CreateRequestBody>,
) -> Result<(StatusCode, Json<RequestResponse>), AppError> {
    // Validate input (sử dụng validator crate)
    body.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    // BR-006: Check permission trên database
    let has_perm = db::permissions::check_permission(&state.db, claims.user_id, body.database_id)
        .await
        .map_err(AppError::Database)?;
    if !has_perm {
        return Err(AppError::DatabasePermissionDenied { db_id: body.database_id });
    }

    // Resolve CR nếu có URL
    let cr_id = if let Some(ref cr_url) = body.cr_url {
        let cr_number = crate::integrations::manageengine::parse_cr_url(cr_url)
            .map_err(|e| AppError::ValidationError(e.to_string()))?;

        // Kiểm tra CR đã tồn tại chưa, nếu chưa thì tạo mới
        let existing = db::change_requests::get_by_cr_number(&state.db, &cr_number)
            .await
            .map_err(AppError::Database)?;

        match existing {
            Some(cr) => Some(cr.id),
            None => {
                // Tạo CR mới (fetch metadata nếu ManageEngine enabled)
                let detail = if state.config.manageengine.enabled {
                    let client = crate::integrations::manageengine::ManageEngineClient::new(
                        &state.config.manageengine,
                    );
                    client.get_change(&cr_number).await.ok()
                } else {
                    None
                };

                let cr = db::change_requests::create(
                    &state.db,
                    &cr_number,
                    cr_url,
                    detail.as_ref().and_then(|d| d.title.as_deref()),
                    detail.as_ref().and_then(|d| d.status.as_deref()),
                    detail.as_ref().and_then(|d| d.scheduled_start_time),
                    detail.as_ref().and_then(|d| d.scheduled_end_time),
                    claims.user_id,
                )
                .await
                .map_err(AppError::Database)?;

                Some(cr.id)
            }
        }
    } else {
        None
    };

    // Tạo request
    let request = db::requests::create_request(
        &state.db,
        &body.description,
        &body.sql_content,
        body.database_id,
        claims.user_id,
        body.submit,
        cr_id,
    )
    .await
    .map_err(AppError::Database)?;

    // BR-005: Audit log — CREATED
    let _ = db::audit::insert(
        &state.db,
        request.id,
        "CREATED",
        claims.user_id,
        None,
        None,
        None,
    )
    .await;

    // Nếu submit → thêm audit SUBMITTED và broadcast notification
    if body.submit {
        let _ = db::audit::insert(
            &state.db,
            request.id,
            "SUBMITTED",
            claims.user_id,
            None,
            None,
            None,
        )
        .await;

        // Broadcast cho Checkers
        let _ = state.notification_tx.send(Notification {
            notification_type: NotificationType::NewRequest,
            request_id: request.id,
            created_by: Some(claims.user_id),
            database_name: None,
            actor_id: Some(claims.user_id),
            rows_affected: None,
            detail: None,
        });
    }

    let response = to_request_response(&request);
    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /api/v1/requests` — Maker xem danh sách requests của mình.
/// Hỗ trợ pagination: ?page=1&per_page=20&status=PENDING
pub async fn list_my_requests(
    State(state): State<AppState>,
    RequireMaker(claims): RequireMaker,
    Query(query): Query<ListRequestsQuery>,
) -> Result<Json<PaginatedResponse<RequestResponse>>, AppError> {
    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 100);
    let offset = (page - 1) * per_page;

    let total = db::requests::count_by_user(&state.db, claims.user_id, query.status.as_deref())
        .await
        .map_err(AppError::Database)?;

    let requests = db::requests::list_by_user(
        &state.db,
        claims.user_id,
        query.status.as_deref(),
        per_page,
        offset,
    )
    .await
    .map_err(AppError::Database)?;

    let responses: Vec<RequestResponse> = requests.iter().map(to_request_response).collect();
    let total_pages = (total + per_page - 1) / per_page;

    Ok(Json(PaginatedResponse {
        data: responses,
        pagination: PaginationMeta {
            page,
            per_page,
            total,
            total_pages,
        },
    }))
}

/// `GET /api/v1/requests/:id` — Maker xem chi tiết request (bao gồm sql_content).
/// Chỉ Maker tạo request mới xem được.
pub async fn get_request_detail(
    State(state): State<AppState>,
    RequireMaker(claims): RequireMaker,
    Path(id): Path<Uuid>,
) -> Result<Json<RequestDetailResponse>, AppError> {
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Chỉ Maker tạo request mới xem được detail
    if request.created_by != claims.user_id {
        return Err(AppError::Forbidden);
    }

    Ok(Json(to_detail_response(&request)))
}

/// `POST /api/v1/requests/:id/submit` — Submit request từ DRAFT → PENDING.
pub async fn submit_request(
    State(state): State<AppState>,
    RequireMaker(claims): RequireMaker,
    Path(id): Path<Uuid>,
) -> Result<Json<RequestResponse>, AppError> {
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Chỉ Maker tạo mới submit được
    if request.created_by != claims.user_id {
        return Err(AppError::Forbidden);
    }

    // BR-003: Validate state transition
    if !request.status.can_transition_to(&RequestStatus::Pending) {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "PENDING".to_string(),
        });
    }

    let updated = db::requests::submit_request(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    // Audit log
    let _ = db::audit::insert(&state.db, id, "SUBMITTED", claims.user_id, None, None, None).await;

    // Broadcast notification
    let _ = state.notification_tx.send(Notification {
        notification_type: NotificationType::NewRequest,
        request_id: id,
        created_by: Some(claims.user_id),
        database_name: None,
        actor_id: Some(claims.user_id),
        rows_affected: None,
        detail: None,
    });

    Ok(Json(to_request_response(&updated)))
}

/// `POST /api/v1/requests/:id/cancel` — Maker cancel request PENDING → CANCELLED.
pub async fn cancel_request(
    State(state): State<AppState>,
    RequireMaker(claims): RequireMaker,
    Path(id): Path<Uuid>,
) -> Result<Json<RequestResponse>, AppError> {
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Chỉ Maker tạo request mới cancel được
    if request.created_by != claims.user_id {
        return Err(AppError::Forbidden);
    }

    // BR-003: Validate state transition
    if !request.status.can_transition_to(&RequestStatus::Cancelled) {
        return Err(AppError::InvalidStateTransition {
            from: request.status.to_string(),
            to: "CANCELLED".to_string(),
        });
    }

    let updated = db::requests::cancel_request(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    // Audit log
    let _ = db::audit::insert(&state.db, id, "CANCELLED", claims.user_id, None, None, None).await;

    Ok(Json(to_request_response(&updated)))
}

/// `PATCH /api/v1/requests/:id` — Maker update request đang DRAFT.
///
/// BR-002: Immutable request — chỉ cho update khi status = DRAFT.
/// Sau khi submit (PENDING+), phải cancel rồi tạo mới.
pub async fn update_request(
    State(state): State<AppState>,
    RequireMaker(claims): RequireMaker,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRequestBody>,
) -> Result<Json<RequestDetailResponse>, AppError> {
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Chỉ Maker tạo request mới update được
    if request.created_by != claims.user_id {
        return Err(AppError::Forbidden);
    }

    // BR-002: Immutable after submission
    if request.status != RequestStatus::Draft {
        return Err(AppError::ImmutableRequest);
    }

    // Validate input (sử dụng validator crate)
    body.validate().map_err(|e| AppError::ValidationError(e.to_string()))?;

    let updated = db::requests::update_draft(&state.db, id, &body.description, &body.sql_content)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(to_detail_response(&updated)))
}

/// `POST /api/v1/requests/:id/clone` — Maker clone request (tạo bản DRAFT mới từ request cũ).
///
/// Cho phép tạo lại request bị reject/failed mà không cần nhập lại SQL.
/// Maker chỉ clone được request của chính mình.
pub async fn clone_request(
    State(state): State<AppState>,
    RequireMaker(claims): RequireMaker,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<RequestResponse>), AppError> {
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Chỉ Maker tạo request mới clone được
    if request.created_by != claims.user_id {
        return Err(AppError::Forbidden);
    }

    // Clone tạo bản mới DRAFT
    let cloned = db::requests::clone_request(&state.db, id, claims.user_id)
        .await
        .map_err(AppError::Database)?;

    // Audit log cho request mới
    let _ = db::audit::insert(
        &state.db,
        cloned.id,
        "CREATED",
        claims.user_id,
        Some(&format!("cloned from request {id}")),
        None,
        None,
    )
    .await;

    Ok((StatusCode::CREATED, Json(to_request_response(&cloned))))
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Chuyển đổi `SqlRequest` thành `RequestResponse` (không bao gồm `sql_content`).
/// Dùng cho danh sách requests để tránh expose nội dung SQL.
fn to_request_response(req: &crate::models::request::SqlRequest) -> RequestResponse {
    RequestResponse {
        id: req.id,
        description: req.description.clone(),
        status: req.status.clone(),
        database_id: req.database_id,
        created_by: req.created_by,
        approved_by: req.approved_by,
        rejection_reason: req.rejection_reason.clone(),
        rows_affected: req.rows_affected,
        execution_error: req.execution_error.clone(),
        created_at: req.created_at.to_rfc3339(),
        updated_at: req.updated_at.to_rfc3339(),
        submitted_at: req.submitted_at.map(|t| t.to_rfc3339()),
    }
}

/// Chuyển đổi `SqlRequest` thành `RequestDetailResponse` (bao gồm `sql_content`).
/// Dùng cho endpoint chi tiết, chỉ trả về cho Maker sở hữu request.
fn to_detail_response(req: &crate::models::request::SqlRequest) -> RequestDetailResponse {
    RequestDetailResponse {
        id: req.id,
        description: req.description.clone(),
        sql_content: req.sql_content.clone(),
        status: req.status.clone(),
        database_id: req.database_id,
        created_by: req.created_by,
        approved_by: req.approved_by,
        rejection_reason: req.rejection_reason.clone(),
        rows_affected: req.rows_affected,
        execution_error: req.execution_error.clone(),
        created_at: req.created_at.to_rfc3339(),
        updated_at: req.updated_at.to_rfc3339(),
        submitted_at: req.submitted_at.map(|t| t.to_rfc3339()),
    }
}
