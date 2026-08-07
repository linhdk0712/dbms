use sqlx::PgPool;
use uuid::Uuid;

use crate::models::data_query::DataQueryRequest;

/// Liệt kê data queries chờ approve tại approver_unit hiện tại.
/// Chỉ trả về requests có level PENDING matching unit.
pub async fn list_pending_by_unit(
    pool: &PgPool,
    approver_unit: &str,
) -> Result<Vec<DataQueryRequest>, sqlx::Error> {
    sqlx::query_as::<_, DataQueryRequest>(
        r#"SELECT dqr.id, dqr.title, dqr.database_id, dqr.schema_name,
                  dqr.sql_query, dqr.tables_accessed, dqr.columns_accessed,
                  dqr.query_mode, dqr.ticket_id, dqr.status, dqr.created_by,
                  dqr.current_approval_level, dqr.total_approval_levels, dqr.expires_at,
                  dqr.created_at, dqr.updated_at, dqr.approved_at, dqr.revoked_at, dqr.revoke_reason
           FROM data_query_requests dqr
           WHERE dqr.status = 'PENDING_APPROVAL'
             AND EXISTS (
                 SELECT 1 FROM data_query_approval_tracking dqat
                 WHERE dqat.query_request_id = dqr.id
                   AND dqat.level_order = dqr.current_approval_level
                   AND dqat.approver_unit = $1
                   AND dqat.status = 'PENDING'
             )
           ORDER BY dqr.created_at ASC"#,
    )
    .bind(approver_unit)
    .fetch_all(pool)
    .await
}

/// Approve current level cho một request.
/// Nếu tất cả levels đã approved → mark request APPROVED.
/// Nếu còn level tiếp → advance current_approval_level.
pub async fn approve_level(
    pool: &PgPool,
    request_id: Uuid,
    approver_id: Uuid,
    comment: Option<&str>,
) -> Result<bool, sqlx::Error> {
    // Get current level info
    let request = sqlx::query_as::<_, DataQueryRequest>(
        r#"SELECT id, title, database_id, schema_name, sql_query, tables_accessed,
                  columns_accessed, query_mode, ticket_id, status, created_by,
                  current_approval_level, total_approval_levels, expires_at,
                  created_at, updated_at, approved_at, revoked_at, revoke_reason
           FROM data_query_requests WHERE id = $1"#,
    )
    .bind(request_id)
    .fetch_one(pool)
    .await?;

    let current_level = request.current_approval_level;

    // Mark current tracking as APPROVED
    sqlx::query(
        r#"UPDATE data_query_approval_tracking
           SET status = 'APPROVED', approved_by = $3, comment = $4, acted_at = NOW()
           WHERE query_request_id = $1 AND level_order = $2"#,
    )
    .bind(request_id)
    .bind(current_level)
    .bind(approver_id)
    .bind(comment)
    .execute(pool)
    .await?;

    // Check if all levels done
    if current_level >= request.total_approval_levels {
        // All approved → mark request APPROVED
        sqlx::query(
            r#"UPDATE data_query_requests
               SET status = 'APPROVED', approved_at = NOW(), updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(request_id)
        .execute(pool)
        .await?;
        Ok(true) // fully approved
    } else {
        // Advance to next level
        sqlx::query(
            r#"UPDATE data_query_requests
               SET current_approval_level = current_approval_level + 1, updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(request_id)
        .execute(pool)
        .await?;
        Ok(false) // partial — next level pending
    }
}

/// Reject request tại current level.
pub async fn reject_request(
    pool: &PgPool,
    request_id: Uuid,
    approver_id: Uuid,
    comment: &str,
) -> Result<(), sqlx::Error> {
    // Get current level
    let (current_level,): (i32,) = sqlx::query_as(
        "SELECT current_approval_level FROM data_query_requests WHERE id = $1",
    )
    .bind(request_id)
    .fetch_one(pool)
    .await?;

    // Mark tracking as REJECTED
    sqlx::query(
        r#"UPDATE data_query_approval_tracking
           SET status = 'REJECTED', approved_by = $3, comment = $4, acted_at = NOW()
           WHERE query_request_id = $1 AND level_order = $2"#,
    )
    .bind(request_id)
    .bind(current_level)
    .bind(approver_id)
    .bind(comment)
    .execute(pool)
    .await?;

    // Mark request as REJECTED
    sqlx::query(
        r#"UPDATE data_query_requests
           SET status = 'REJECTED', updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(request_id)
    .execute(pool)
    .await?;

    Ok(())
}
