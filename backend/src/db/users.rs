use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::{Role, User};

/// Tìm user theo OIDC subject. Trả về None nếu chưa tồn tại.
pub async fn find_by_subject(pool: &PgPool, subject: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"SELECT id, oidc_subject, email, display_name, is_active, created_at, updated_at
           FROM users
           WHERE oidc_subject = $1"#,
    )
    .bind(subject)
    .fetch_optional(pool)
    .await
}

/// Tạo user mới hoặc cập nhật thông tin nếu đã tồn tại.
/// Trả về user sau upsert.
///
/// Logic:
/// - Nếu user đã tồn tại (theo oidc_subject HOẶC email) → UPDATE oidc_subject, email, display_name
/// - Nếu user chưa tồn tại → INSERT mới với is_active = true
/// - Nếu là user đầu tiên trong hệ thống → tự gán ADMIN + MAKER + CHECKER
///
/// Handle cả 2 trường hợp conflict:
/// 1. Cùng oidc_subject (user login lại) → update email/display_name
/// 2. Cùng email nhưng khác oidc_subject (user recreated trong IdP) → update oidc_subject
pub async fn upsert_from_oidc(
    pool: &PgPool,
    subject: &str,
    email: &str,
    display_name: &str,
) -> Result<User, sqlx::Error> {
    // Trước tiên thử tìm user theo email (handle trường hợp oidc_subject thay đổi)
    let existing_by_email = sqlx::query_as::<_, User>(
        r#"SELECT id, oidc_subject, email, display_name, is_active, created_at, updated_at
           FROM users
           WHERE email = $1"#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    if let Some(_existing) = existing_by_email {
        // User tồn tại với email này — cập nhật oidc_subject (có thể đã thay đổi từ IdP)
        let user = sqlx::query_as::<_, User>(
            r#"UPDATE users
               SET oidc_subject = $1, display_name = $2, updated_at = NOW()
               WHERE email = $3
               RETURNING id, oidc_subject, email, display_name, is_active, created_at, updated_at"#,
        )
        .bind(subject)
        .bind(display_name)
        .bind(email)
        .fetch_one(pool)
        .await?;

        // Nếu user chưa có role nào → kiểm tra xem có phải là user duy nhất không
        // Nếu đúng thì gán ADMIN (bootstrap scenario)
        ensure_first_user_has_admin(pool, user.id).await?;

        Ok(user)
    } else {
        // User chưa tồn tại — thử insert, ON CONFLICT handle trường hợp race condition trên oidc_subject
        let user = sqlx::query_as::<_, User>(
            r#"INSERT INTO users (id, oidc_subject, email, display_name, is_active, created_at, updated_at)
               VALUES ($1, $2, $3, $4, true, NOW(), NOW())
               ON CONFLICT (oidc_subject)
               DO UPDATE SET
                   email = EXCLUDED.email,
                   display_name = EXCLUDED.display_name,
                   updated_at = NOW()
               RETURNING id, oidc_subject, email, display_name, is_active, created_at, updated_at"#,
        )
        .bind(Uuid::new_v4())
        .bind(subject)
        .bind(email)
        .bind(display_name)
        .fetch_one(pool)
        .await?;

        // Nếu là user đầu tiên → gán tất cả roles (bootstrap)
        ensure_first_user_has_admin(pool, user.id).await?;

        Ok(user)
    }
}

/// Nếu hệ thống chưa có ai được gán role ADMIN, gán ADMIN + MAKER + CHECKER cho user này.
/// Đây là bootstrap logic để user đầu tiên có thể quản lý hệ thống.
async fn ensure_first_user_has_admin(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    // Kiểm tra xem đã có bất kỳ ADMIN nào trong hệ thống chưa
    let admin_exists: Option<(i64,)> = sqlx::query_as(
        r#"SELECT COUNT(*) FROM user_roles WHERE role = 'ADMIN'"#,
    )
    .fetch_optional(pool)
    .await?;

    let has_admin = admin_exists.map(|(count,)| count > 0).unwrap_or(false);

    if !has_admin {
        tracing::info!(user_id = %user_id, "No ADMIN found in system — bootstrapping first user with all roles");

        // Gán tất cả roles cho user đầu tiên
        for role in &["ADMIN", "MAKER", "CHECKER"] {
            sqlx::query(
                r#"INSERT INTO user_roles (id, user_id, role, granted_at)
                   VALUES ($1, $2, $3, NOW())
                   ON CONFLICT (user_id, role) DO NOTHING"#,
            )
            .bind(Uuid::new_v4())
            .bind(user_id)
            .bind(*role)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Lấy danh sách roles của user từ bảng user_roles.
pub async fn get_user_roles(pool: &PgPool, user_id: Uuid) -> Result<Vec<Role>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT role FROM user_roles WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let roles = rows
        .into_iter()
        .filter_map(|(r,)| match r.as_str() {
            "ADMIN" => Some(Role::Admin),
            "MAKER" => Some(Role::Maker),
            "CHECKER" => Some(Role::Checker),
            _ => None,
        })
        .collect();

    Ok(roles)
}

/// Kiểm tra user có active không.
pub async fn is_user_active(pool: &PgPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result: Option<(bool,)> = sqlx::query_as(
        r#"SELECT is_active FROM users WHERE id = $1"#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(active,)| active).unwrap_or(false))
}
