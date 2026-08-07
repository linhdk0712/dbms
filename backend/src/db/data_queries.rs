use sqlx::PgPool;
use uuid::Uuid;

use crate::models::data_query::{DataQueryRequest, DataQueryExecution, DataQueryApprovalTracking};

/// Tạo data query request mới (status: DRAFT).
pub async fn create_request(
    pool: &PgPool,
    title: &str,
    database_id: Uuid,
    schema_name: &str,
    sql_query: &str,
    tables_accessed: &[String],
    columns_accessed: &[String],
    query_mode: &str,
    ticket_id: &str,
    created_by: Uuid,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<DataQueryRequest, sqlx::Error> {
    sqlx::query_as::<_, DataQueryRequest>(
        r#"INSERT INTO data_query_requests
           (title, database_id, schema_name, sql_query, tables_accessed, columns_accessed,
            query_mode, ticket_id, created_by, expires_at, status, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'DRAFT', NOW(), NOW())
           RETURNING id, title, database_id, schema_name, sql_query, tables_accessed,
                     columns_accessed, query_mode, ticket_id, status, created_by,
                     current_approval_level, total_approval_levels, expires_at,
                     created_at, updated_at, approved_at, revoked_at, revoke_reason"#,
    )
    .bind(title)
    .bind(database_id)
    .bind(schema_name)
    .bind(sql_query)
    .bind(tables_accessed)
    .bind(columns_accessed)
    .bind(query_mode)
    .bind(ticket_id)
    .bind(created_by)
    .bind(expires_at)
    .fetch_one(pool)
    .await
}

/// Lấy data query theo ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<DataQueryRequest>, sqlx::Error> {
    sqlx::query_as::<_, DataQueryRequest>(
        r#"SELECT id, title, database_id, schema_name, sql_query, tables_accessed,
                  columns_accessed, query_mode, ticket_id, status, created_by,
                  current_approval_level, total_approval_levels, expires_at,
                  created_at, updated_at, approved_at, revoked_at, revoke_reason
           FROM data_query_requests WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Liệt kê data queries của user (hoặc assigned to user).
pub async fn list_by_user(
    pool: &PgPool,
    user_id: Uuid,
    status_filter: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<DataQueryRequest>, sqlx::Error> {
    if let Some(status) = status_filter {
        sqlx::query_as::<_, DataQueryRequest>(
            r#"SELECT DISTINCT dqr.id, dqr.title, dqr.database_id, dqr.schema_name,
                      dqr.sql_query, dqr.tables_accessed, dqr.columns_accessed,
                      dqr.query_mode, dqr.ticket_id, dqr.status, dqr.created_by,
                      dqr.current_approval_level, dqr.total_approval_levels, dqr.expires_at,
                      dqr.created_at, dqr.updated_at, dqr.approved_at, dqr.revoked_at, dqr.revoke_reason
               FROM data_query_requests dqr
               LEFT JOIN data_query_assigned_users dqau ON dqau.query_request_id = dqr.id
               WHERE (dqr.created_by = $1 OR dqau.user_id = $1)
                 AND dqr.status = $2
               ORDER BY dqr.updated_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(user_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, DataQueryRequest>(
            r#"SELECT DISTINCT dqr.id, dqr.title, dqr.database_id, dqr.schema_name,
                      dqr.sql_query, dqr.tables_accessed, dqr.columns_accessed,
                      dqr.query_mode, dqr.ticket_id, dqr.status, dqr.created_by,
                      dqr.current_approval_level, dqr.total_approval_levels, dqr.expires_at,
                      dqr.created_at, dqr.updated_at, dqr.approved_at, dqr.revoked_at, dqr.revoke_reason
               FROM data_query_requests dqr
               LEFT JOIN data_query_assigned_users dqau ON dqau.query_request_id = dqr.id
               WHERE (dqr.created_by = $1 OR dqau.user_id = $1)
               ORDER BY dqr.updated_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }
}

/// Đếm data queries cho pagination.
pub async fn count_by_user(
    pool: &PgPool,
    user_id: Uuid,
    status_filter: Option<&str>,
) -> Result<i64, sqlx::Error> {
    if let Some(status) = status_filter {
        let (count,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(DISTINCT dqr.id)
               FROM data_query_requests dqr
               LEFT JOIN data_query_assigned_users dqau ON dqau.query_request_id = dqr.id
               WHERE (dqr.created_by = $1 OR dqau.user_id = $1) AND dqr.status = $2"#,
        )
        .bind(user_id)
        .bind(status)
        .fetch_one(pool)
        .await?;
        Ok(count)
    } else {
        let (count,): (i64,) = sqlx::query_as(
            r#"SELECT COUNT(DISTINCT dqr.id)
               FROM data_query_requests dqr
               LEFT JOIN data_query_assigned_users dqau ON dqau.query_request_id = dqr.id
               WHERE (dqr.created_by = $1 OR dqau.user_id = $1)"#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }
}

/// Update draft data query.
pub async fn update_draft(
    pool: &PgPool,
    id: Uuid,
    title: &str,
    schema_name: &str,
    sql_query: &str,
    tables_accessed: &[String],
    columns_accessed: &[String],
    query_mode: &str,
    ticket_id: &str,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<DataQueryRequest, sqlx::Error> {
    sqlx::query_as::<_, DataQueryRequest>(
        r#"UPDATE data_query_requests
           SET title = $2, schema_name = $3, sql_query = $4, tables_accessed = $5,
               columns_accessed = $6, query_mode = $7, ticket_id = $8,
               expires_at = $9, updated_at = NOW()
           WHERE id = $1 AND status = 'DRAFT'
           RETURNING id, title, database_id, schema_name, sql_query, tables_accessed,
                     columns_accessed, query_mode, ticket_id, status, created_by,
                     current_approval_level, total_approval_levels, expires_at,
                     created_at, updated_at, approved_at, revoked_at, revoke_reason"#,
    )
    .bind(id)
    .bind(title)
    .bind(schema_name)
    .bind(sql_query)
    .bind(tables_accessed)
    .bind(columns_accessed)
    .bind(query_mode)
    .bind(ticket_id)
    .bind(expires_at)
    .fetch_one(pool)
    .await
}

/// Submit data query: DRAFT → PENDING_APPROVAL.
/// Cập nhật total_approval_levels và tạo approval tracking records.
pub async fn submit_request(
    pool: &PgPool,
    id: Uuid,
    approval_levels: &[(i32, String)], // (level_order, approver_unit)
) -> Result<DataQueryRequest, sqlx::Error> {
    let total_levels = approval_levels.len() as i32;

    // Update status
    let request = sqlx::query_as::<_, DataQueryRequest>(
        r#"UPDATE data_query_requests
           SET status = 'PENDING_APPROVAL',
               current_approval_level = 1,
               total_approval_levels = $2,
               updated_at = NOW()
           WHERE id = $1 AND status = 'DRAFT'
           RETURNING id, title, database_id, schema_name, sql_query, tables_accessed,
                     columns_accessed, query_mode, ticket_id, status, created_by,
                     current_approval_level, total_approval_levels, expires_at,
                     created_at, updated_at, approved_at, revoked_at, revoke_reason"#,
    )
    .bind(id)
    .bind(total_levels)
    .fetch_one(pool)
    .await?;

    // Create approval tracking records cho mỗi level
    for (level_order, approver_unit) in approval_levels {
        sqlx::query(
            r#"INSERT INTO data_query_approval_tracking
               (query_request_id, level_order, approver_unit, status)
               VALUES ($1, $2, $3, 'PENDING')"#,
        )
        .bind(id)
        .bind(level_order)
        .bind(approver_unit)
        .execute(pool)
        .await?;
    }

    Ok(request)
}

/// Kiểm tra user có được assign để run query hay không.
pub async fn is_user_assigned(
    pool: &PgPool,
    request_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let (exists,): (bool,) = sqlx::query_as(
        r#"SELECT EXISTS(
            SELECT 1 FROM data_query_assigned_users
            WHERE query_request_id = $1 AND user_id = $2
        )"#,
    )
    .bind(request_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Insert assigned users cho một request.
pub async fn insert_assigned_users(
    pool: &PgPool,
    request_id: Uuid,
    user_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    for user_id in user_ids {
        sqlx::query(
            r#"INSERT INTO data_query_assigned_users (query_request_id, user_id)
               VALUES ($1, $2)
               ON CONFLICT (query_request_id, user_id) DO NOTHING"#,
        )
        .bind(request_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Lấy approval tracking cho một request.
pub async fn get_approval_tracking(
    pool: &PgPool,
    request_id: Uuid,
) -> Result<Vec<DataQueryApprovalTracking>, sqlx::Error> {
    sqlx::query_as::<_, DataQueryApprovalTracking>(
        r#"SELECT id, query_request_id, level_order, approver_unit, status,
                  approved_by, comment, acted_at
           FROM data_query_approval_tracking
           WHERE query_request_id = $1
           ORDER BY level_order ASC"#,
    )
    .bind(request_id)
    .fetch_all(pool)
    .await
}

/// Insert execution audit record.
pub async fn insert_execution(
    pool: &PgPool,
    query_request_id: Uuid,
    executed_by: Uuid,
    rows_returned: i64,
    execution_time_ms: i64,
    client_ip: Option<&str>,
    status: &str,
    error_message: Option<&str>,
) -> Result<DataQueryExecution, sqlx::Error> {
    sqlx::query_as::<_, DataQueryExecution>(
        r#"INSERT INTO data_query_executions
           (query_request_id, executed_by, rows_returned, execution_time_ms,
            client_ip, status, error_message)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, query_request_id, executed_by, rows_returned,
                     execution_time_ms, client_ip, status, error_message, executed_at"#,
    )
    .bind(query_request_id)
    .bind(executed_by)
    .bind(rows_returned)
    .bind(execution_time_ms)
    .bind(client_ip)
    .bind(status)
    .bind(error_message)
    .fetch_one(pool)
    .await
}

/// Lấy execution history cho một query.
pub async fn list_executions(
    pool: &PgPool,
    query_request_id: Uuid,
) -> Result<Vec<DataQueryExecution>, sqlx::Error> {
    sqlx::query_as::<_, DataQueryExecution>(
        r#"SELECT id, query_request_id, executed_by, rows_returned,
                  execution_time_ms, client_ip, status, error_message, executed_at
           FROM data_query_executions
           WHERE query_request_id = $1
           ORDER BY executed_at DESC"#,
    )
    .bind(query_request_id)
    .fetch_all(pool)
    .await
}

/// Mark request as APPROVED (tất cả levels đã approve).
pub async fn mark_approved(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE data_query_requests
           SET status = 'APPROVED', approved_at = NOW(), updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Mark request as REJECTED.
pub async fn mark_rejected(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE data_query_requests
           SET status = 'REJECTED', updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
