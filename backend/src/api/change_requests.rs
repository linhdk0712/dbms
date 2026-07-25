use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::auth::extractors::{RequireAdmin, RequireAuth};
use crate::db;
use crate::error::AppError;
use crate::integrations::manageengine::{self, ManageEngineClient, ManageEngineError};
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCrBody {
    /// Full URL từ ManageEngine ServiceDesk Plus
    #[validate(length(min = 10, max = 2000, message = "cr_url must be 10-2000 characters"))]
    pub cr_url: String,
}

/// Query parameters cho endpoint list Change Requests.
#[derive(Debug, Deserialize)]
pub struct ListCrQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

/// Giá trị mặc định cho page number (bắt đầu từ 1).
fn default_page() -> i64 { 1 }

/// Giá trị mặc định cho số lượng item mỗi trang.
fn default_per_page() -> i64 { 20 }

/// Response DTO cho một Change Request, bao gồm metadata và số request liên kết.
#[derive(Debug, Serialize)]
pub struct CrResponse {
    pub id: Uuid,
    pub cr_number: String,
    pub cr_url: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub scheduled_start_time: Option<String>,
    pub scheduled_end_time: Option<String>,
    pub synced_at: Option<String>,
    pub created_by: Uuid,
    pub created_at: String,
    pub request_count: i64,
}

/// Response DTO cho danh sách Change Requests kèm thông tin phân trang.
#[derive(Debug, Serialize)]
pub struct CrListResponse {
    pub data: Vec<CrResponse>,
    pub pagination: PaginationMeta,
}

/// Metadata phân trang cho list responses.
#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
    pub total_pages: i64,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `POST /api/v1/change-requests` — Tạo CR từ URL ManageEngine.
///
/// Parse URL → extract change_id → gọi ManageEngine API fetch metadata → lưu DB.
/// Nếu CR đã tồn tại (same cr_number) → update metadata (upsert).
pub async fn create_cr(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Json(body): Json<CreateCrBody>,
) -> Result<(StatusCode, Json<CrResponse>), AppError> {
    body.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Parse URL để extract change_id
    let cr_number = manageengine::parse_cr_url(&body.cr_url)
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    // Fetch metadata từ ManageEngine (nếu enabled)
    let detail = fetch_cr_metadata(&state, &cr_number).await;

    // Lưu vào DB (upsert)
    let cr = db::change_requests::create(
        &state.db,
        &cr_number,
        &body.cr_url,
        detail.as_ref().and_then(|d| d.title.as_deref()),
        detail.as_ref().and_then(|d| d.status.as_deref()),
        detail.as_ref().and_then(|d| d.scheduled_start_time),
        detail.as_ref().and_then(|d| d.scheduled_end_time),
        claims.user_id,
    )
    .await
    .map_err(AppError::Database)?;

    let request_count = db::change_requests::count_requests(&state.db, cr.id)
        .await
        .unwrap_or(0);

    Ok((StatusCode::CREATED, Json(to_cr_response(&cr, request_count))))
}

/// `GET /api/v1/change-requests` — List tất cả CR (pagination).
pub async fn list_crs(
    State(state): State<AppState>,
    RequireAuth(_claims): RequireAuth,
    Query(query): Query<ListCrQuery>,
) -> Result<Json<CrListResponse>, AppError> {
    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 100);
    let offset = (page - 1) * per_page;

    let total = db::change_requests::count_all(&state.db)
        .await
        .map_err(AppError::Database)?;

    let crs = db::change_requests::list_all(&state.db, per_page, offset)
        .await
        .map_err(AppError::Database)?;

    let mut data = Vec::with_capacity(crs.len());
    for cr in &crs {
        let count = db::change_requests::count_requests(&state.db, cr.id)
            .await
            .unwrap_or(0);
        data.push(to_cr_response(cr, count));
    }

    let total_pages = (total + per_page - 1) / per_page;

    Ok(Json(CrListResponse {
        data,
        pagination: PaginationMeta {
            page,
            per_page,
            total,
            total_pages,
        },
    }))
}

/// `GET /api/v1/change-requests/:id` — Chi tiết CR.
pub async fn get_cr_detail(
    State(state): State<AppState>,
    RequireAuth(_claims): RequireAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<CrResponse>, AppError> {
    let cr = db::change_requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    let request_count = db::change_requests::count_requests(&state.db, cr.id)
        .await
        .unwrap_or(0);

    Ok(Json(to_cr_response(&cr, request_count)))
}

/// `POST /api/v1/change-requests/:id/sync` — Re-sync metadata từ ManageEngine.
pub async fn sync_cr(
    State(state): State<AppState>,
    RequireAuth(_claims): RequireAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<CrResponse>, AppError> {
    let cr = db::change_requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Fetch fresh metadata
    let detail = fetch_cr_metadata(&state, &cr.cr_number).await;

    let updated = match detail {
        Some(d) => {
            db::change_requests::update_from_sync(
                &state.db,
                id,
                d.title.as_deref(),
                d.status.as_deref(),
                d.scheduled_start_time,
                d.scheduled_end_time,
            )
            .await
            .map_err(AppError::Database)?
        }
        None => cr,
    };

    let request_count = db::change_requests::count_requests(&state.db, updated.id)
        .await
        .unwrap_or(0);

    Ok(Json(to_cr_response(&updated, request_count)))
}

/// `DELETE /api/v1/change-requests/:id` — Xóa CR (Admin only).
/// Các sql_requests liên kết sẽ tự động set cr_id = NULL.
pub async fn delete_cr(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // Kiểm tra tồn tại
    db::change_requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    db::change_requests::delete(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Fetch metadata từ ManageEngine. Trả None nếu disabled hoặc lỗi (không block flow).
async fn fetch_cr_metadata(
    state: &AppState,
    cr_number: &str,
) -> Option<manageengine::ChangeDetail> {
    if !state.config.manageengine.enabled {
        return None;
    }

    let client = ManageEngineClient::new(&state.config.manageengine);
    match client.get_change(cr_number).await {
        Ok(detail) => Some(detail),
        Err(ManageEngineError::Disabled) => None,
        Err(e) => {
            tracing::warn!(cr_number, error = %e, "Failed to fetch CR metadata from ManageEngine");
            None
        }
    }
}

/// Chuyển đổi model [`ChangeRequest`] thành response DTO [`CrResponse`].
///
/// Các trường datetime được format sang RFC 3339 string.
fn to_cr_response(
    cr: &crate::models::change_request::ChangeRequest,
    request_count: i64,
) -> CrResponse {
    CrResponse {
        id: cr.id,
        cr_number: cr.cr_number.clone(),
        cr_url: cr.cr_url.clone(),
        title: cr.title.clone(),
        status: cr.status.clone(),
        scheduled_start_time: cr.scheduled_start_time.map(|t| t.to_rfc3339()),
        scheduled_end_time: cr.scheduled_end_time.map(|t| t.to_rfc3339()),
        synced_at: cr.synced_at.map(|t| t.to_rfc3339()),
        created_by: cr.created_by,
        created_at: cr.created_at.to_rfc3339(),
        request_count,
    }
}

/// Kiểm tra maintenance window cho một request.
/// Trả Ok(()) nếu:
/// - Request không có CR
/// - CR không có time window
/// - Hiện tại nằm trong window
///
/// Trả Err nếu hiện tại ngoài maintenance window.
pub async fn check_maintenance_window(
    pool: &sqlx::PgPool,
    cr_id: Option<Uuid>,
) -> Result<(), AppError> {
    let cr_id = match cr_id {
        Some(id) => id,
        None => return Ok(()), // Không có CR → không enforce
    };

    let cr = match db::change_requests::get_by_id(pool, cr_id).await {
        Ok(Some(cr)) => cr,
        Ok(None) => return Ok(()), // CR đã bị xóa → không block
        Err(_) => return Ok(()),   // DB error → graceful, không block
    };

    // Chỉ enforce nếu CR có cả start và end time
    if let (Some(start), Some(end)) = (cr.scheduled_start_time, cr.scheduled_end_time) {
        let now = Utc::now();
        if now < start || now > end {
            return Err(AppError::CrWindowViolation {
                cr_number: cr.cr_number,
                window_start: start,
                window_end: end,
            });
        }
    }

    Ok(())
}
