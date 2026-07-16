use sqlx::PgPool;
use uuid::Uuid;

use crate::models::approval::AuditLog;

/// Ghi một bản ghi audit log. Append-only — không bao giờ UPDATE/DELETE.
pub async fn insert(
    pool: &PgPool,
    request_id: Uuid,
    action: &str,
    actor_id: Uuid,
    detail: Option<&str>,
    execution_result: Option<i64>,
    ip_address: Option<&str>,
) -> Result<AuditLog, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        r#"INSERT INTO audit_log (id, request_id, action, actor_id, detail, execution_result, ip_address, timestamp)
           VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, NOW())
           RETURNING id, request_id, action, actor_id, detail, execution_result, ip_address, timestamp"#,
    )
    .bind(request_id)
    .bind(action)
    .bind(actor_id)
    .bind(detail)
    .bind(execution_result)
    .bind(ip_address)
    .fetch_one(pool)
    .await
}

/// Lấy audit log theo request_id, sắp xếp theo thời gian.
pub async fn get_by_request_id(
    pool: &PgPool,
    request_id: Uuid,
) -> Result<Vec<AuditLog>, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        r#"SELECT id, request_id, action, actor_id, detail, execution_result, ip_address, timestamp
           FROM audit_log
           WHERE request_id = $1
           ORDER BY timestamp ASC"#,
    )
    .bind(request_id)
    .fetch_all(pool)
    .await
}

/// Ghi audit log cho admin actions (không liên kết request_id).
/// Dùng cho các hành động như credential rotation, role change, v.v.
pub async fn insert_admin_action(
    pool: &PgPool,
    action: &str,
    actor_id: Uuid,
    detail: &str,
    ip_address: &str,
) -> Result<AuditLog, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        r#"INSERT INTO audit_log (id, request_id, action, actor_id, detail, execution_result, ip_address, timestamp)
           VALUES (gen_random_uuid(), NULL, $1, $2, $3, NULL, $4, NOW())
           RETURNING id, request_id, action, actor_id, detail, execution_result, ip_address, timestamp"#,
    )
    .bind(action)
    .bind(actor_id)
    .bind(detail)
    .bind(ip_address)
    .fetch_one(pool)
    .await
}

/// Lấy audit log theo actor_id (lịch sử hành động của user).
pub async fn get_by_actor_id(
    pool: &PgPool,
    actor_id: Uuid,
    limit: i64,
) -> Result<Vec<AuditLog>, sqlx::Error> {
    sqlx::query_as::<_, AuditLog>(
        r#"SELECT id, request_id, action, actor_id, detail, execution_result, ip_address, timestamp
           FROM audit_log
           WHERE actor_id = $1
           ORDER BY timestamp DESC
           LIMIT $2"#,
    )
    .bind(actor_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}
