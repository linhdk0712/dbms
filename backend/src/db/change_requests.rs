use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::change_request::ChangeRequest;

/// Tạo mới một Change Request hoặc cập nhật nếu `cr_number` đã tồn tại (upsert).
///
/// Khi conflict trên `cr_number`, chỉ cập nhật các field metadata (title, status,
/// scheduled times) nếu giá trị mới không NULL (COALESCE logic).
///
/// Trả về bản ghi sau khi insert hoặc update.
pub async fn create(
    pool: &PgPool,
    cr_number: &str,
    cr_url: &str,
    title: Option<&str>,
    status: Option<&str>,
    scheduled_start_time: Option<DateTime<Utc>>,
    scheduled_end_time: Option<DateTime<Utc>>,
    created_by: Uuid,
) -> Result<ChangeRequest, sqlx::Error> {
    sqlx::query_as::<_, ChangeRequest>(
        r#"
        INSERT INTO change_requests (cr_number, cr_url, title, status, scheduled_start_time, scheduled_end_time, synced_at, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), $7)
        ON CONFLICT (cr_number) DO UPDATE SET
            title = COALESCE(EXCLUDED.title, change_requests.title),
            status = COALESCE(EXCLUDED.status, change_requests.status),
            scheduled_start_time = COALESCE(EXCLUDED.scheduled_start_time, change_requests.scheduled_start_time),
            scheduled_end_time = COALESCE(EXCLUDED.scheduled_end_time, change_requests.scheduled_end_time),
            synced_at = NOW()
        RETURNING id, cr_number, cr_url, title, status, scheduled_start_time, scheduled_end_time, synced_at, created_by, created_at
        "#,
    )
    .bind(cr_number)
    .bind(cr_url)
    .bind(title)
    .bind(status)
    .bind(scheduled_start_time)
    .bind(scheduled_end_time)
    .bind(created_by)
    .fetch_one(pool)
    .await
}

/// Lấy CR theo ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<ChangeRequest>, sqlx::Error> {
    sqlx::query_as::<_, ChangeRequest>(
        "SELECT id, cr_number, cr_url, title, status, scheduled_start_time, scheduled_end_time, synced_at, created_by, created_at FROM change_requests WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Lấy CR theo cr_number.
pub async fn get_by_cr_number(pool: &PgPool, cr_number: &str) -> Result<Option<ChangeRequest>, sqlx::Error> {
    sqlx::query_as::<_, ChangeRequest>(
        "SELECT id, cr_number, cr_url, title, status, scheduled_start_time, scheduled_end_time, synced_at, created_by, created_at FROM change_requests WHERE cr_number = $1",
    )
    .bind(cr_number)
    .fetch_optional(pool)
    .await
}

/// List tất cả CR, sắp xếp theo created_at DESC.
pub async fn list_all(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<ChangeRequest>, sqlx::Error> {
    sqlx::query_as::<_, ChangeRequest>(
        "SELECT id, cr_number, cr_url, title, status, scheduled_start_time, scheduled_end_time, synced_at, created_by, created_at FROM change_requests ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Đếm tổng số CR.
pub async fn count_all(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM change_requests")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Cập nhật metadata CR sau khi sync từ ManageEngine.
///
/// Title và status dùng COALESCE (giữ giá trị cũ nếu mới là NULL).
/// Scheduled times được ghi đè trực tiếp (cho phép xóa window bằng None).
/// `synced_at` tự động set thành NOW().
pub async fn update_from_sync(
    pool: &PgPool,
    id: Uuid,
    title: Option<&str>,
    status: Option<&str>,
    scheduled_start_time: Option<DateTime<Utc>>,
    scheduled_end_time: Option<DateTime<Utc>>,
) -> Result<ChangeRequest, sqlx::Error> {
    sqlx::query_as::<_, ChangeRequest>(
        r#"
        UPDATE change_requests
        SET title = COALESCE($2, title),
            status = COALESCE($3, status),
            scheduled_start_time = $4,
            scheduled_end_time = $5,
            synced_at = NOW()
        WHERE id = $1
        RETURNING id, cr_number, cr_url, title, status, scheduled_start_time, scheduled_end_time, synced_at, created_by, created_at
        "#,
    )
    .bind(id)
    .bind(title)
    .bind(status)
    .bind(scheduled_start_time)
    .bind(scheduled_end_time)
    .fetch_one(pool)
    .await
}

/// Xóa CR (Admin only). Các sql_requests liên kết sẽ set cr_id = NULL (ON DELETE SET NULL).
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM change_requests WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Đếm số SQL requests gắn với một CR.
pub async fn count_requests(pool: &PgPool, cr_id: Uuid) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sql_requests WHERE cr_id = $1")
        .bind(cr_id)
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}
