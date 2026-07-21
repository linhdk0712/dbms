use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::UserDbPermission;

/// Kiểm tra user có quyền trên database không.
pub async fn check_permission(
    pool: &PgPool,
    user_id: Uuid,
    database_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result: Option<(i32,)> = sqlx::query_as(
        r#"SELECT 1 as exists_flag FROM user_db_permissions
           WHERE user_id = $1 AND database_id = $2"#,
    )
    .bind(user_id)
    .bind(database_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.is_some())
}

/// Cấp quyền cho user trên database.
pub async fn grant_permission(
    pool: &PgPool,
    user_id: Uuid,
    database_id: Uuid,
    granted_by: Uuid,
) -> Result<UserDbPermission, sqlx::Error> {
    sqlx::query_as::<_, UserDbPermission>(
        r#"INSERT INTO user_db_permissions (id, user_id, database_id, granted_by, granted_at)
           VALUES (gen_random_uuid(), $1, $2, $3, NOW())
           RETURNING id, user_id, database_id, granted_by, granted_at"#,
    )
    .bind(user_id)
    .bind(database_id)
    .bind(granted_by)
    .fetch_one(pool)
    .await
}

/// Thu hồi quyền của user trên database.
pub async fn revoke_permission(
    pool: &PgPool,
    user_id: Uuid,
    database_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"DELETE FROM user_db_permissions WHERE user_id = $1 AND database_id = $2"#,
    )
    .bind(user_id)
    .bind(database_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Liệt kê quyền của user (danh sách databases user có quyền).
pub async fn list_user_permissions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserDbPermission>, sqlx::Error> {
    sqlx::query_as::<_, UserDbPermission>(
        r#"SELECT id, user_id, database_id, granted_by, granted_at
           FROM user_db_permissions
           WHERE user_id = $1
           ORDER BY granted_at ASC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Liệt kê users có quyền trên một database cụ thể.
pub async fn list_database_users(
    pool: &PgPool,
    database_id: Uuid,
) -> Result<Vec<UserDbPermission>, sqlx::Error> {
    sqlx::query_as::<_, UserDbPermission>(
        r#"SELECT id, user_id, database_id, granted_by, granted_at
           FROM user_db_permissions
           WHERE database_id = $1
           ORDER BY granted_at ASC"#,
    )
    .bind(database_id)
    .fetch_all(pool)
    .await
}
