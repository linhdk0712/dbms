use sqlx::PgPool;
use uuid::Uuid;

use crate::models::data_query::{DataQueryApprovalRule, DataQueryApprovalLevel};

/// Liệt kê tất cả approval rules, sắp xếp theo database, schema, table.
///
/// Dùng bởi Admin/DBA để xem toàn bộ ma trận phê duyệt đã cấu hình.
///
/// # Errors
///
/// Trả về `sqlx::Error` nếu truy vấn database thất bại.
pub async fn list_rules(pool: &PgPool) -> Result<Vec<DataQueryApprovalRule>, sqlx::Error> {
    sqlx::query_as::<_, DataQueryApprovalRule>(
        r#"SELECT id, database_id, schema_name, table_name, created_by,
                  is_active, created_at, updated_at
           FROM data_query_approval_rules
           ORDER BY database_id, schema_name, table_name"#,
    )
    .fetch_all(pool)
    .await
}

/// Lấy danh sách approval levels cho một rule cụ thể, sắp xếp theo thứ tự cấp.
///
/// # Parameters
///
/// * `pool` - PostgreSQL connection pool.
/// * `rule_id` - UUID của approval rule cần lấy levels.
///
/// # Errors
///
/// Trả về `sqlx::Error` nếu truy vấn database thất bại.
pub async fn get_rule_levels(pool: &PgPool, rule_id: Uuid) -> Result<Vec<DataQueryApprovalLevel>, sqlx::Error> {
    sqlx::query_as::<_, DataQueryApprovalLevel>(
        r#"SELECT id, rule_id, level_order, approver_unit, required
           FROM data_query_approval_levels
           WHERE rule_id = $1
           ORDER BY level_order ASC"#,
    )
    .bind(rule_id)
    .fetch_all(pool)
    .await
}

/// Tạo approval rule mới cùng với các approval levels tương ứng.
///
/// Rule xác định cần bao nhiêu cấp phê duyệt cho một database/schema/table cụ thể.
/// Nếu `schema_name` là `None`, rule áp dụng cho toàn database.
/// Nếu `table_name` là `None`, rule áp dụng cho toàn schema.
///
/// # Parameters
///
/// * `pool` - PostgreSQL connection pool.
/// * `database_id` - UUID của database connection mà rule áp dụng.
/// * `schema_name` - Tên schema (hoặc `None` cho database-level rule).
/// * `table_name` - Tên table (hoặc `None` cho schema-level rule).
/// * `created_by` - UUID của user (Admin/DBA) tạo rule.
/// * `levels` - Danh sách các level dưới dạng `(level_order, approver_unit, required)`.
///
/// # Errors
///
/// Trả về `sqlx::Error` nếu insert thất bại (vd: duplicate constraint vi phạm).
pub async fn create_rule(
    pool: &PgPool,
    database_id: Uuid,
    schema_name: Option<&str>,
    table_name: Option<&str>,
    created_by: Uuid,
    levels: &[(i32, String, bool)], // (level_order, approver_unit, required)
) -> Result<DataQueryApprovalRule, sqlx::Error> {
    let rule = sqlx::query_as::<_, DataQueryApprovalRule>(
        r#"INSERT INTO data_query_approval_rules
           (database_id, schema_name, table_name, created_by, is_active, created_at, updated_at)
           VALUES ($1, $2, $3, $4, TRUE, NOW(), NOW())
           RETURNING id, database_id, schema_name, table_name, created_by,
                     is_active, created_at, updated_at"#,
    )
    .bind(database_id)
    .bind(schema_name)
    .bind(table_name)
    .bind(created_by)
    .fetch_one(pool)
    .await?;

    // Insert levels
    for (level_order, approver_unit, required) in levels {
        sqlx::query(
            r#"INSERT INTO data_query_approval_levels (rule_id, level_order, approver_unit, required)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(rule.id)
        .bind(level_order)
        .bind(approver_unit)
        .bind(required)
        .execute(pool)
        .await?;
    }

    Ok(rule)
}

/// Cập nhật approval rule: thay đổi schema/table scope và thay thế toàn bộ levels.
///
/// Thao tác xóa toàn bộ levels cũ rồi insert levels mới (replace strategy).
///
/// # Parameters
///
/// * `pool` - PostgreSQL connection pool.
/// * `rule_id` - UUID của rule cần cập nhật.
/// * `schema_name` - Tên schema mới (hoặc `None` cho database-level rule).
/// * `table_name` - Tên table mới (hoặc `None` cho schema-level rule).
/// * `levels` - Danh sách levels mới dưới dạng `(level_order, approver_unit, required)`.
///
/// # Errors
///
/// Trả về `sqlx::Error` nếu rule_id không tồn tại hoặc truy vấn thất bại.
pub async fn update_rule(
    pool: &PgPool,
    rule_id: Uuid,
    schema_name: Option<&str>,
    table_name: Option<&str>,
    levels: &[(i32, String, bool)],
) -> Result<DataQueryApprovalRule, sqlx::Error> {
    // Update rule metadata
    let rule = sqlx::query_as::<_, DataQueryApprovalRule>(
        r#"UPDATE data_query_approval_rules
           SET schema_name = $2, table_name = $3, updated_at = NOW()
           WHERE id = $1
           RETURNING id, database_id, schema_name, table_name, created_by,
                     is_active, created_at, updated_at"#,
    )
    .bind(rule_id)
    .bind(schema_name)
    .bind(table_name)
    .fetch_one(pool)
    .await?;

    // Delete old levels
    sqlx::query("DELETE FROM data_query_approval_levels WHERE rule_id = $1")
        .bind(rule_id)
        .execute(pool)
        .await?;

    // Insert new levels
    for (level_order, approver_unit, required) in levels {
        sqlx::query(
            r#"INSERT INTO data_query_approval_levels (rule_id, level_order, approver_unit, required)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(rule_id)
        .bind(level_order)
        .bind(approver_unit)
        .bind(required)
        .execute(pool)
        .await?;
    }

    Ok(rule)
}

/// Xóa approval rule và tất cả levels liên quan (cascade).
///
/// # Parameters
///
/// * `pool` - PostgreSQL connection pool.
/// * `rule_id` - UUID của rule cần xóa.
///
/// # Errors
///
/// Trả về `sqlx::Error` nếu truy vấn thất bại.
pub async fn delete_rule(pool: &PgPool, rule_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM data_query_approval_rules WHERE id = $1")
        .bind(rule_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Resolve approval chain cho một data query dựa trên danh sách tables accessed.
///
/// Áp dụng chiến lược "most specific wins":
/// - Table-level rule > Schema-level rule > Database-level rule > Default (ITO only).
///
/// Khi query truy cập nhiều tables (JOIN), merge tất cả approval units
/// thành một chain duy nhất (union), sắp xếp theo priority: ITO → COMPLIANCE → ITS.
///
/// # Parameters
///
/// * `pool` - PostgreSQL connection pool.
/// * `database_id` - UUID của database connection mà query target.
/// * `schema_name` - Tên schema chứa các tables.
/// * `tables` - Danh sách tên tables mà query truy cập (từ SQL parsing).
///
/// # Returns
///
/// Trả về `Vec<(level_order, approver_unit)>` — chuỗi phê duyệt tuần tự.
/// Mặc định trả về `[(1, "ITO")]` nếu không có rule nào match.
///
/// # Errors
///
/// Trả về `sqlx::Error` nếu truy vấn database thất bại.
pub async fn resolve_approval_levels(
    pool: &PgPool,
    database_id: Uuid,
    schema_name: &str,
    tables: &[String],
) -> Result<Vec<(i32, String)>, sqlx::Error> {
    let mut all_units: Vec<String> = Vec::new();

    for table in tables {
        // Try most specific first: exact table match
        let levels = sqlx::query_as::<_, DataQueryApprovalLevel>(
            r#"SELECT dal.id, dal.rule_id, dal.level_order, dal.approver_unit, dal.required
               FROM data_query_approval_levels dal
               JOIN data_query_approval_rules dar ON dar.id = dal.rule_id
               WHERE dar.database_id = $1
                 AND dar.is_active = TRUE
                 AND dar.schema_name = $2
                 AND dar.table_name = $3
               ORDER BY dal.level_order"#,
        )
        .bind(database_id)
        .bind(schema_name)
        .bind(table)
        .fetch_all(pool)
        .await?;

        if !levels.is_empty() {
            for level in &levels {
                let unit = level.approver_unit.to_string();
                if !all_units.contains(&unit) {
                    all_units.push(unit);
                }
            }
            continue;
        }

        // Fallback: schema-level rule
        let levels = sqlx::query_as::<_, DataQueryApprovalLevel>(
            r#"SELECT dal.id, dal.rule_id, dal.level_order, dal.approver_unit, dal.required
               FROM data_query_approval_levels dal
               JOIN data_query_approval_rules dar ON dar.id = dal.rule_id
               WHERE dar.database_id = $1
                 AND dar.is_active = TRUE
                 AND dar.schema_name = $2
                 AND dar.table_name IS NULL
               ORDER BY dal.level_order"#,
        )
        .bind(database_id)
        .bind(schema_name)
        .fetch_all(pool)
        .await?;

        if !levels.is_empty() {
            for level in &levels {
                let unit = level.approver_unit.to_string();
                if !all_units.contains(&unit) {
                    all_units.push(unit);
                }
            }
            continue;
        }

        // Fallback: database-level rule
        let levels = sqlx::query_as::<_, DataQueryApprovalLevel>(
            r#"SELECT dal.id, dal.rule_id, dal.level_order, dal.approver_unit, dal.required
               FROM data_query_approval_levels dal
               JOIN data_query_approval_rules dar ON dar.id = dal.rule_id
               WHERE dar.database_id = $1
                 AND dar.is_active = TRUE
                 AND dar.schema_name IS NULL
                 AND dar.table_name IS NULL
               ORDER BY dal.level_order"#,
        )
        .bind(database_id)
        .fetch_all(pool)
        .await?;

        if !levels.is_empty() {
            for level in &levels {
                let unit = level.approver_unit.to_string();
                if !all_units.contains(&unit) {
                    all_units.push(unit);
                }
            }
        }
    }

    // Default: ITO only nếu không tìm thấy rule nào
    if all_units.is_empty() {
        all_units.push("ITO".to_string());
    }

    // Sort by priority: ITO → COMPLIANCE → ITS
    all_units.sort_by_key(|u| match u.as_str() {
        "ITO" => 0,
        "COMPLIANCE" => 1,
        "ITS" => 2,
        _ => 99,
    });

    // Return as (level_order, approver_unit)
    Ok(all_units
        .into_iter()
        .enumerate()
        .map(|(i, unit)| ((i + 1) as i32, unit))
        .collect())
}
