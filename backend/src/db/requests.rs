use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::request::SqlRequest;
use crate::models::request_approval::RequestApproval;

/// Tạo SQL request mới (status: DRAFT hoặc PENDING tùy submit flag).
/// Nếu submit=true, snapshot `approval_levels_required` từ database_connections (Req 10.8).
pub async fn create_request(
    pool: &PgPool,
    description: &str,
    sql_content: &str,
    database_id: Uuid,
    created_by: Uuid,
    submit: bool,
) -> Result<SqlRequest, sqlx::Error> {
    let id = Uuid::new_v4();
    let status = if submit { "PENDING" } else { "DRAFT" };
    let submitted_at = if submit { Some(Utc::now()) } else { None };

    if submit {
        // Snapshot approval_levels_required from database_connections at submit time (Req 10.8)
        sqlx::query_as::<_, SqlRequest>(
            r#"INSERT INTO sql_requests (id, description, sql_content, status, database_id, created_by, submitted_at, approval_levels_required, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7,
                       (SELECT approval_levels_required FROM database_connections WHERE id = $5),
                       NOW(), NOW())
               RETURNING id, description, sql_content, status, database_id, created_by, approved_by,
                         rejection_reason, executed_at, rows_affected, execution_error,
                         created_at, updated_at, submitted_at, approval_levels_required"#,
        )
        .bind(id)
        .bind(description)
        .bind(sql_content)
        .bind(status)
        .bind(database_id)
        .bind(created_by)
        .bind(submitted_at)
        .fetch_one(pool)
        .await
    } else {
        // DRAFT — approval_levels_required defaults to 1, will be snapshotted on submit
        sqlx::query_as::<_, SqlRequest>(
            r#"INSERT INTO sql_requests (id, description, sql_content, status, database_id, created_by, submitted_at, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
               RETURNING id, description, sql_content, status, database_id, created_by, approved_by,
                         rejection_reason, executed_at, rows_affected, execution_error,
                         created_at, updated_at, submitted_at, approval_levels_required"#,
        )
        .bind(id)
        .bind(description)
        .bind(sql_content)
        .bind(status)
        .bind(database_id)
        .bind(created_by)
        .bind(submitted_at)
        .fetch_one(pool)
        .await
    }
}

/// Lấy request theo ID (bao gồm sql_content).
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<SqlRequest>, sqlx::Error> {
    sqlx::query_as::<_, SqlRequest>(
        r#"SELECT id, description, sql_content, status, database_id, created_by, approved_by,
                  rejection_reason, executed_at, rows_affected, execution_error,
                  created_at, updated_at, submitted_at, approval_levels_required
           FROM sql_requests WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Liệt kê requests của một user (Maker xem requests của mình).
/// Hỗ trợ pagination và filter theo status.
pub async fn list_by_user(
    pool: &PgPool,
    user_id: Uuid,
    status_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<SqlRequest>, sqlx::Error> {
    if let Some(status) = status_filter {
        sqlx::query_as::<_, SqlRequest>(
            r#"SELECT id, description, sql_content, status, database_id, created_by, approved_by,
                      rejection_reason, executed_at, rows_affected, execution_error,
                      created_at, updated_at, submitted_at, approval_levels_required
               FROM sql_requests
               WHERE created_by = $1 AND status = $2
               ORDER BY updated_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(user_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, SqlRequest>(
            r#"SELECT id, description, sql_content, status, database_id, created_by, approved_by,
                      rejection_reason, executed_at, rows_affected, execution_error,
                      created_at, updated_at, submitted_at, approval_levels_required
               FROM sql_requests
               WHERE created_by = $1
               ORDER BY updated_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }
}

/// Đếm tổng số requests của user (cho pagination metadata).
pub async fn count_by_user(
    pool: &PgPool,
    user_id: Uuid,
    status_filter: Option<&str>,
) -> Result<i64, sqlx::Error> {
    if let Some(status) = status_filter {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sql_requests WHERE created_by = $1 AND status = $2",
        )
        .bind(user_id)
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(count)
    } else {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sql_requests WHERE created_by = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }
}

/// Liệt kê requests đang PENDING (Checker review).
/// Exclude requests created by the checker themselves (4-eyes principle, Req 13.1, 13.2).
pub async fn list_pending(pool: &PgPool, checker_user_id: Uuid) -> Result<Vec<SqlRequest>, sqlx::Error> {
    sqlx::query_as::<_, SqlRequest>(
        r#"SELECT id, description, sql_content, status, database_id, created_by, approved_by,
                  rejection_reason, executed_at, rows_affected, execution_error,
                  created_at, updated_at, submitted_at, approval_levels_required
           FROM sql_requests
           WHERE status = 'PENDING' AND created_by != $1
           ORDER BY submitted_at ASC"#,
    )
    .bind(checker_user_id)
    .fetch_all(pool)
    .await
}

/// Submit request (DRAFT → PENDING).
/// Snapshot `approval_levels_required` từ database_connections tại thời điểm submit (Req 10.8).
pub async fn submit_request(pool: &PgPool, id: Uuid) -> Result<SqlRequest, sqlx::Error> {
    sqlx::query_as::<_, SqlRequest>(
        r#"UPDATE sql_requests
           SET status = 'PENDING',
               submitted_at = NOW(),
               updated_at = NOW(),
               approval_levels_required = (
                   SELECT dc.approval_levels_required
                   FROM database_connections dc
                   WHERE dc.id = sql_requests.database_id
               )
           WHERE id = $1
           RETURNING id, description, sql_content, status, database_id, created_by, approved_by,
                     rejection_reason, executed_at, rows_affected, execution_error,
                     created_at, updated_at, submitted_at, approval_levels_required"#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

/// Approve request (PENDING → APPROVED).
pub async fn approve_request(
    pool: &PgPool,
    id: Uuid,
    approved_by: Uuid,
) -> Result<SqlRequest, sqlx::Error> {
    sqlx::query_as::<_, SqlRequest>(
        r#"UPDATE sql_requests
           SET status = 'APPROVED', approved_by = $2, updated_at = NOW()
           WHERE id = $1
           RETURNING id, description, sql_content, status, database_id, created_by, approved_by,
                     rejection_reason, executed_at, rows_affected, execution_error,
                     created_at, updated_at, submitted_at, approval_levels_required"#,
    )
    .bind(id)
    .bind(approved_by)
    .fetch_one(pool)
    .await
}

/// Reject request (PENDING → REJECTED).
pub async fn reject_request(
    pool: &PgPool,
    id: Uuid,
    rejection_reason: &str,
) -> Result<SqlRequest, sqlx::Error> {
    sqlx::query_as::<_, SqlRequest>(
        r#"UPDATE sql_requests
           SET status = 'REJECTED', rejection_reason = $2, updated_at = NOW()
           WHERE id = $1
           RETURNING id, description, sql_content, status, database_id, created_by, approved_by,
                     rejection_reason, executed_at, rows_affected, execution_error,
                     created_at, updated_at, submitted_at, approval_levels_required"#,
    )
    .bind(id)
    .bind(rejection_reason)
    .fetch_one(pool)
    .await
}

/// Cancel request (PENDING → CANCELLED). Chỉ Maker tạo request mới được cancel.
pub async fn cancel_request(pool: &PgPool, id: Uuid) -> Result<SqlRequest, sqlx::Error> {
    sqlx::query_as::<_, SqlRequest>(
        r#"UPDATE sql_requests
           SET status = 'CANCELLED', updated_at = NOW()
           WHERE id = $1
           RETURNING id, description, sql_content, status, database_id, created_by, approved_by,
                     rejection_reason, executed_at, rows_affected, execution_error,
                     created_at, updated_at, submitted_at, approval_levels_required"#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

/// Đánh dấu request là EXECUTED thành công.
pub async fn mark_executed(
    pool: &PgPool,
    id: Uuid,
    rows_affected: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE sql_requests
           SET status = 'EXECUTED', executed_at = NOW(), rows_affected = $2, updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(rows_affected)
    .execute(pool)
    .await?;
    Ok(())
}

/// Đánh dấu request là FAILED.
pub async fn mark_failed(
    pool: &PgPool,
    id: Uuid,
    execution_error: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE sql_requests
           SET status = 'FAILED', execution_error = $2, executed_at = NOW(), updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(execution_error)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update request đang ở trạng thái DRAFT (BR-002: immutable sau khi submit).
/// Chỉ cho phép update description và sql_content.
pub async fn update_draft(
    pool: &PgPool,
    id: Uuid,
    description: &str,
    sql_content: &str,
) -> Result<SqlRequest, sqlx::Error> {
    sqlx::query_as::<_, SqlRequest>(
        r#"UPDATE sql_requests
           SET description = $2, sql_content = $3, updated_at = NOW()
           WHERE id = $1 AND status = 'DRAFT'
           RETURNING id, description, sql_content, status, database_id, created_by, approved_by,
                     rejection_reason, executed_at, rows_affected, execution_error,
                     created_at, updated_at, submitted_at, approval_levels_required"#,
    )
    .bind(id)
    .bind(description)
    .bind(sql_content)
    .fetch_one(pool)
    .await
}

/// Clone request — tạo bản copy mới từ request hiện có (status: DRAFT).
/// Cho phép Maker tạo lại request bị reject hoặc failed mà không cần nhập lại SQL.
pub async fn clone_request(
    pool: &PgPool,
    source_id: Uuid,
    created_by: Uuid,
) -> Result<SqlRequest, sqlx::Error> {
    let new_id = Uuid::new_v4();

    sqlx::query_as::<_, SqlRequest>(
        r#"INSERT INTO sql_requests (id, description, sql_content, status, database_id, created_by, created_at, updated_at)
           SELECT $1, description, sql_content, 'DRAFT', database_id, $2, NOW(), NOW()
           FROM sql_requests
           WHERE id = $3
           RETURNING id, description, sql_content, status, database_id, created_by, approved_by,
                     rejection_reason, executed_at, rows_affected, execution_error,
                     created_at, updated_at, submitted_at, approval_levels_required"#,
    )
    .bind(new_id)
    .bind(created_by)
    .bind(source_id)
    .fetch_one(pool)
    .await
}

// ── Multi-level Approval Functions (Req 10) ────────────────────────────

/// Insert approval record. Level = current_count + 1.
/// Returns the inserted RequestApproval.
/// DB has UNIQUE(request_id, approver_id) — duplicate will return sqlx error.
pub async fn insert_approval(
    pool: &PgPool,
    request_id: Uuid,
    approver_id: Uuid,
    level: i32,
) -> Result<RequestApproval, sqlx::Error> {
    sqlx::query_as::<_, RequestApproval>(
        r#"INSERT INTO request_approvals (id, request_id, approver_id, level, approved_at)
           VALUES (gen_random_uuid(), $1, $2, $3, NOW())
           RETURNING id, request_id, approver_id, level, approved_at"#,
    )
    .bind(request_id)
    .bind(approver_id)
    .bind(level)
    .fetch_one(pool)
    .await
}

/// Đếm số approvals hiện có cho một request.
pub async fn count_approvals(pool: &PgPool, request_id: Uuid) -> Result<i64, sqlx::Error> {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM request_approvals WHERE request_id = $1",
    )
    .bind(request_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

/// Kiểm tra user đã approve request này chưa (Req 10.6, 10.7).
pub async fn has_already_approved(
    pool: &PgPool,
    request_id: Uuid,
    approver_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let (exists,): (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM request_approvals WHERE request_id = $1 AND approver_id = $2)",
    )
    .bind(request_id)
    .bind(approver_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}
