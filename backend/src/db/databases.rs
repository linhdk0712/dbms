use sqlx::PgPool;
use uuid::Uuid;

use crate::models::database::DatabaseConnection;

/// Tạo database connection mới.
pub async fn create(
    pool: &PgPool,
    name: &str,
    db_type: &str,
    host: &str,
    port: i32,
    database_name: &str,
    schema_name: Option<&str>,
    encrypted_credentials: &str,
    timeout_secs: i32,
) -> Result<DatabaseConnection, sqlx::Error> {
    sqlx::query_as::<_, DatabaseConnection>(
        r#"INSERT INTO database_connections
               (id, name, db_type, host, port, database_name, schema_name, encrypted_credentials, timeout_secs, is_active, created_at, updated_at)
           VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8, true, NOW(), NOW())
           RETURNING id, name, db_type, host, port, database_name, schema_name,
                     encrypted_credentials, timeout_secs, is_active, created_at, updated_at,
                     approval_levels_required"#,
    )
    .bind(name)
    .bind(db_type)
    .bind(host)
    .bind(port)
    .bind(database_name)
    .bind(schema_name)
    .bind(encrypted_credentials)
    .bind(timeout_secs)
    .fetch_one(pool)
    .await
}

/// Lấy database connection theo ID.
pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<DatabaseConnection>, sqlx::Error> {
    sqlx::query_as::<_, DatabaseConnection>(
        r#"SELECT id, name, db_type, host, port, database_name, schema_name,
                  encrypted_credentials, timeout_secs, is_active, created_at, updated_at,
                  approval_levels_required
           FROM database_connections WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Liệt kê tất cả database connections (Admin view).
pub async fn list_all(pool: &PgPool) -> Result<Vec<DatabaseConnection>, sqlx::Error> {
    sqlx::query_as::<_, DatabaseConnection>(
        r#"SELECT id, name, db_type, host, port, database_name, schema_name,
                  encrypted_credentials, timeout_secs, is_active, created_at, updated_at,
                  approval_levels_required
           FROM database_connections
           ORDER BY name ASC"#,
    )
    .fetch_all(pool)
    .await
}

/// Liệt kê database connections active (cho Maker chọn khi tạo request).
pub async fn list_active(pool: &PgPool) -> Result<Vec<DatabaseConnection>, sqlx::Error> {
    sqlx::query_as::<_, DatabaseConnection>(
        r#"SELECT id, name, db_type, host, port, database_name, schema_name,
                  encrypted_credentials, timeout_secs, is_active, created_at, updated_at,
                  approval_levels_required
           FROM database_connections
           WHERE is_active = true
           ORDER BY name ASC"#,
    )
    .fetch_all(pool)
    .await
}

/// Cập nhật thông tin database connection (không thay đổi credentials nếu không cung cấp).
pub async fn update(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    host: Option<&str>,
    port: Option<i32>,
    database_name: Option<&str>,
    schema_name: Option<&str>,
    timeout_secs: Option<i32>,
    is_active: Option<bool>,
) -> Result<DatabaseConnection, sqlx::Error> {
    // Dynamic update — chỉ update fields được cung cấp
    sqlx::query_as::<_, DatabaseConnection>(
        r#"UPDATE database_connections
           SET name = COALESCE($2, name),
               host = COALESCE($3, host),
               port = COALESCE($4, port),
               database_name = COALESCE($5, database_name),
               schema_name = COALESCE($6, schema_name),
               timeout_secs = COALESCE($7, timeout_secs),
               is_active = COALESCE($8, is_active),
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, db_type, host, port, database_name, schema_name,
                     encrypted_credentials, timeout_secs, is_active, created_at, updated_at,
                     approval_levels_required"#,
    )
    .bind(id)
    .bind(name)
    .bind(host)
    .bind(port)
    .bind(database_name)
    .bind(schema_name)
    .bind(timeout_secs)
    .bind(is_active)
    .fetch_one(pool)
    .await
}

/// Liệt kê databases mà user có quyền truy cập (Maker endpoint).
/// Chỉ trả về databases active, join với user_db_permissions.
pub async fn list_by_user_permission(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<crate::models::database::MakerDatabaseResponse>, sqlx::Error> {
    sqlx::query_as::<_, crate::models::database::MakerDatabaseResponse>(
        r#"SELECT dc.id, dc.name, dc.db_type, dc.host, dc.port, dc.database_name, dc.schema_name
           FROM database_connections dc
           INNER JOIN user_db_permissions udp ON dc.id = udp.database_id
           WHERE udp.user_id = $1 AND dc.is_active = true
           ORDER BY dc.name ASC
           LIMIT 100"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Cập nhật encrypted credentials.
pub async fn update_credentials(
    pool: &PgPool,
    id: Uuid,
    encrypted_credentials: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE database_connections
           SET encrypted_credentials = $2, updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(encrypted_credentials)
    .execute(pool)
    .await?;
    Ok(())
}
