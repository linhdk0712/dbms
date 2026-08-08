use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Approver unit row.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ApproverUnit {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// User-approver-unit assignment row.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct UserApproverUnit {
    pub id: Uuid,
    pub user_id: Uuid,
    pub unit_id: Uuid,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
}

// ── Approver Units CRUD ────────────────────────────────────────────────

/// List all approver units.
pub async fn list_all(pool: &PgPool) -> Result<Vec<ApproverUnit>, sqlx::Error> {
    sqlx::query_as::<_, ApproverUnit>(
        "SELECT id, name, description, is_active, created_at \
         FROM approver_units ORDER BY name"
    )
    .fetch_all(pool)
    .await
}

/// List active approver units only.
pub async fn list_active(pool: &PgPool) -> Result<Vec<ApproverUnit>, sqlx::Error> {
    sqlx::query_as::<_, ApproverUnit>(
        "SELECT id, name, description, is_active, created_at \
         FROM approver_units WHERE is_active = TRUE ORDER BY name"
    )
    .fetch_all(pool)
    .await
}

/// Create a new approver unit.
pub async fn create(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
) -> Result<ApproverUnit, sqlx::Error> {
    sqlx::query_as::<_, ApproverUnit>(
        "INSERT INTO approver_units (name, description) \
         VALUES ($1, $2) \
         RETURNING id, name, description, is_active, created_at"
    )
    .bind(name)
    .bind(description)
    .fetch_one(pool)
    .await
}

/// Update an approver unit.
pub async fn update(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    is_active: Option<bool>,
) -> Result<ApproverUnit, sqlx::Error> {
    sqlx::query_as::<_, ApproverUnit>(
        "UPDATE approver_units \
         SET name = COALESCE($2, name), \
             description = COALESCE($3, description), \
             is_active = COALESCE($4, is_active) \
         WHERE id = $1 \
         RETURNING id, name, description, is_active, created_at"
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(is_active)
    .fetch_one(pool)
    .await
}

/// Delete an approver unit.
pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM approver_units WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── User-Unit Assignments ──────────────────────────────────────────────

/// Get approver units assigned to a user (returns unit names).
pub async fn get_user_units(pool: &PgPool, user_id: Uuid) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT au.name FROM approver_units au \
         JOIN user_approver_units uau ON au.id = uau.unit_id \
         WHERE uau.user_id = $1 AND au.is_active = TRUE \
         ORDER BY au.name"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// Get full assignment details for a user.
pub async fn get_user_unit_details(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<ApproverUnit>, sqlx::Error> {
    sqlx::query_as::<_, ApproverUnit>(
        "SELECT au.id, au.name, au.description, au.is_active, au.created_at \
         FROM approver_units au \
         JOIN user_approver_units uau ON au.id = uau.unit_id \
         WHERE uau.user_id = $1 \
         ORDER BY au.name"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Assign a user to an approver unit.
pub async fn assign_user(
    pool: &PgPool,
    user_id: Uuid,
    unit_id: Uuid,
    granted_by: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_approver_units (user_id, unit_id, granted_by) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, unit_id) DO NOTHING"
    )
    .bind(user_id)
    .bind(unit_id)
    .bind(granted_by)
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove a user from an approver unit.
pub async fn unassign_user(
    pool: &PgPool,
    user_id: Uuid,
    unit_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM user_approver_units WHERE user_id = $1 AND unit_id = $2"
    )
    .bind(user_id)
    .bind(unit_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
