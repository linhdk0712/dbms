use sqlx::PgPool;
use uuid::Uuid;

/// Thống kê tổng quan hệ thống (Admin view).
/// Trả về tuple: (total, pending, approved, executed, failed, rejected, cancelled).
pub async fn get_admin_stats(
    pool: &PgPool,
) -> Result<(i64, i64, i64, i64, i64, i64, i64), sqlx::Error> {
    sqlx::query_as(
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
}

/// Thống kê cho Checker: pending chờ review (loại trừ request của chính mình).
pub async fn count_pending_for_review(
    pool: &PgPool,
    checker_user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let (count,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*)
           FROM sql_requests
           WHERE status = 'PENDING' AND created_by != $1"#,
    )
    .bind(checker_user_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// Đếm số lần Checker đã approve (từ bảng request_approvals).
pub async fn count_approved_by_checker(
    pool: &PgPool,
    checker_user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let (count,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*)
           FROM request_approvals
           WHERE approver_id = $1"#,
    )
    .bind(checker_user_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// Đếm số lần Checker đã reject (từ audit_log).
pub async fn count_rejected_by_checker(
    pool: &PgPool,
    checker_user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let (count,): (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*)
           FROM audit_log
           WHERE action = 'REJECTED' AND actor_id = $1"#,
    )
    .bind(checker_user_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// Thống kê requests của Maker (chỉ requests do mình tạo).
/// Trả về tuple: (total, pending, approved, executed, failed, rejected, cancelled).
pub async fn get_maker_stats(
    pool: &PgPool,
    maker_user_id: Uuid,
) -> Result<(i64, i64, i64, i64, i64, i64, i64), sqlx::Error> {
    sqlx::query_as(
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
    .bind(maker_user_id)
    .fetch_one(pool)
    .await
}
