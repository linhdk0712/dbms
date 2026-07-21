//! Schema Introspector — lấy metadata schema từ database đích.
//!
//! Introspect chỉ các tables được reference trong SQL để:
//! - Tiết kiệm tokens khi gửi prompt cho AI
//! - Giảm noise, tăng chất lượng recommendations
//!
//! Hỗ trợ: PostgreSQL, MySQL, Oracle (placeholder), MongoDB (placeholder).

use std::time::Duration;

use regex::Regex;
use serde::Serialize;

use crate::executor::DbCredentials;
use super::AiError;

// ── Schema Models ──────────────────────────────────────────────────────

/// Schema context tổng hợp — input cho prompt builder.
#[derive(Debug, Clone, Serialize)]
pub struct SchemaContext {
    pub tables: Vec<TableInfo>,
    pub indexes: Vec<IndexInfo>,
}

/// Thông tin một table.
#[derive(Debug, Clone, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub row_count_estimate: Option<i64>,
}

/// Thông tin một column.
#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

/// Thông tin một index.
#[derive(Debug, Clone, Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}

// ── Table Name Extraction ──────────────────────────────────────────────

/// Extract table names từ SQL statement (best-effort parsing).
///
/// Không dùng full SQL parser — regex-based, đủ tốt cho hầu hết cases.
/// Detect các patterns: FROM table, JOIN table, INTO table, UPDATE table,
/// TABLE table (DDL), TRUNCATE table.
pub fn extract_table_names(sql: &str) -> Vec<String> {
    let patterns = [
        r"(?i)\bFROM\s+([a-zA-Z_][a-zA-Z0-9_.]*)",
        r"(?i)\bJOIN\s+([a-zA-Z_][a-zA-Z0-9_.]*)",
        r"(?i)\bINTO\s+([a-zA-Z_][a-zA-Z0-9_.]*)",
        r"(?i)\bUPDATE\s+([a-zA-Z_][a-zA-Z0-9_.]*)",
        r"(?i)\bTABLE\s+([a-zA-Z_][a-zA-Z0-9_.]*)",
        r"(?i)\bTRUNCATE\s+(?:TABLE\s+)?([a-zA-Z_][a-zA-Z0-9_.]*)",
    ];

    let mut tables: Vec<String> = Vec::new();

    for pattern in &patterns {
        let re = Regex::new(pattern).expect("invalid regex");
        for cap in re.captures_iter(sql) {
            if let Some(m) = cap.get(1) {
                let name = m.as_str().to_lowercase();
                // Bỏ qua SQL keywords bị match nhầm
                if !is_sql_keyword(&name) && !tables.contains(&name) {
                    tables.push(name);
                }
            }
        }
    }

    tables
}

/// Kiểm tra xem tên có phải SQL keyword không (tránh false positives).
fn is_sql_keyword(name: &str) -> bool {
    matches!(
        name,
        "select" | "where" | "set" | "values" | "on" | "and" | "or"
        | "not" | "null" | "true" | "false" | "as" | "in" | "is"
        | "like" | "between" | "exists" | "case" | "when" | "then"
        | "else" | "end" | "group" | "order" | "having" | "limit"
        | "offset" | "union" | "all" | "distinct" | "index"
    )
}

// ── Introspection Functions ────────────────────────────────────────────

/// Introspect schema từ PostgreSQL — lấy tables, columns, indexes.
pub async fn introspect_postgres(
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    schema_name: Option<&str>,
    table_names: &[String],
    timeout_duration: Duration,
) -> Result<SchemaContext, AiError> {
    use tokio::time::timeout;

    if table_names.is_empty() {
        return Ok(SchemaContext { tables: vec![], indexes: vec![] });
    }

    let tls_disabled = std::env::var("DISABLE_TARGET_DB_TLS")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");
    let sslmode = if tls_disabled { "disable" } else { "require" };

    let mut conn_str = format!(
        "postgres://{}:{}@{}:{}/{}?sslmode={}",
        urlencoding::encode(&credentials.username),
        urlencoding::encode(&credentials.password),
        host,
        port,
        database_name,
        sslmode
    );
    if let Some(schema) = schema_name {
        conn_str.push_str(&format!("&options=-c%20search_path%3D{schema}"));
    }

    let pool = match timeout(timeout_duration, async {
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&conn_str)
            .await
    }).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return Err(AiError::SchemaError(format!("Connection failed: {e}"))),
        Err(_) => return Err(AiError::SchemaError("Connection timed out".to_string())),
    };

    let schema = schema_name.unwrap_or("public");

    // Query columns cho các tables được reference
    let mut tables: Vec<TableInfo> = Vec::new();
    for table_name in table_names {
        let columns = sqlx::query_as::<_, PgColumnRow>(
            "SELECT c.column_name, c.data_type, c.is_nullable, \
             CASE WHEN tc.constraint_type = 'PRIMARY KEY' THEN true ELSE false END as is_pk \
             FROM information_schema.columns c \
             LEFT JOIN information_schema.key_column_usage kcu \
               ON c.table_schema = kcu.table_schema \
               AND c.table_name = kcu.table_name \
               AND c.column_name = kcu.column_name \
             LEFT JOIN information_schema.table_constraints tc \
               ON kcu.constraint_name = tc.constraint_name \
               AND tc.constraint_type = 'PRIMARY KEY' \
             WHERE c.table_schema = $1 AND c.table_name = $2 \
             ORDER BY c.ordinal_position"
        )
        .bind(schema)
        .bind(table_name)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        if columns.is_empty() {
            continue; // Table không tồn tại hoặc không có columns
        }

        // Estimate row count
        let row_count: Option<i64> = sqlx::query_scalar(
            "SELECT reltuples::bigint FROM pg_class \
             WHERE relname = $1 AND relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = $2)"
        )
        .bind(table_name)
        .bind(schema)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

        tables.push(TableInfo {
            name: table_name.clone(),
            columns: columns.into_iter().map(|c| ColumnInfo {
                name: c.column_name,
                data_type: c.data_type,
                is_nullable: c.is_nullable == "YES",
                is_primary_key: c.is_pk.unwrap_or(false),
            }).collect(),
            row_count_estimate: row_count,
        });
    }

    // Query indexes
    let table_list: Vec<&str> = table_names.iter().map(|s| s.as_str()).collect();
    let indexes = query_pg_indexes(&pool, schema, &table_list).await;

    pool.close().await;

    Ok(SchemaContext { tables, indexes })
}

/// Introspect schema từ MySQL.
pub async fn introspect_mysql(
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    table_names: &[String],
    timeout_duration: Duration,
) -> Result<SchemaContext, AiError> {
    use tokio::time::timeout;

    if table_names.is_empty() {
        return Ok(SchemaContext { tables: vec![], indexes: vec![] });
    }

    let tls_disabled = std::env::var("DISABLE_TARGET_DB_TLS")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");
    let ssl_param = if tls_disabled { "" } else { "?ssl-mode=REQUIRED" };

    let conn_str = format!(
        "mysql://{}:{}@{}:{}/{}{}",
        urlencoding::encode(&credentials.username),
        urlencoding::encode(&credentials.password),
        host,
        port,
        database_name,
        ssl_param
    );

    let pool = match timeout(timeout_duration, async {
        sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&conn_str)
            .await
    }).await {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => return Err(AiError::SchemaError(format!("Connection failed: {e}"))),
        Err(_) => return Err(AiError::SchemaError("Connection timed out".to_string())),
    };

    let mut tables: Vec<TableInfo> = Vec::new();
    for table_name in table_names {
        let columns = sqlx::query_as::<_, MysqlColumnRow>(
            "SELECT c.COLUMN_NAME, c.DATA_TYPE, c.IS_NULLABLE, c.COLUMN_KEY \
             FROM information_schema.COLUMNS c \
             WHERE c.TABLE_SCHEMA = ? AND c.TABLE_NAME = ? \
             ORDER BY c.ORDINAL_POSITION"
        )
        .bind(database_name)
        .bind(table_name)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        if columns.is_empty() {
            continue;
        }

        // Row count estimate
        let row_count: Option<i64> = sqlx::query_scalar(
            "SELECT TABLE_ROWS FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?"
        )
        .bind(database_name)
        .bind(table_name)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

        tables.push(TableInfo {
            name: table_name.clone(),
            columns: columns.into_iter().map(|c| ColumnInfo {
                name: c.column_name,
                data_type: c.data_type,
                is_nullable: c.is_nullable == "YES",
                is_primary_key: c.column_key == "PRI",
            }).collect(),
            row_count_estimate: row_count,
        });
    }

    // Query indexes
    let mut indexes: Vec<IndexInfo> = Vec::new();
    for table_name in table_names {
        let idx_rows = sqlx::query_as::<_, MysqlIndexRow>(
            "SELECT INDEX_NAME, COLUMN_NAME, NON_UNIQUE \
             FROM information_schema.STATISTICS \
             WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
             ORDER BY INDEX_NAME, SEQ_IN_INDEX"
        )
        .bind(database_name)
        .bind(table_name)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        // Group by index name
        let mut idx_map: std::collections::HashMap<String, (Vec<String>, bool)> = std::collections::HashMap::new();
        for row in idx_rows {
            let entry = idx_map.entry(row.index_name.clone()).or_insert_with(|| (Vec::new(), row.non_unique == 0));
            entry.0.push(row.column_name);
        }

        for (idx_name, (cols, is_unique)) in idx_map {
            indexes.push(IndexInfo {
                name: idx_name,
                table_name: table_name.clone(),
                columns: cols,
                is_unique,
            });
        }
    }

    pool.close().await;

    Ok(SchemaContext { tables, indexes })
}

/// Introspect schema cho Oracle — placeholder, trả empty context.
pub async fn introspect_oracle(
    _credentials: &DbCredentials,
    _host: &str,
    _port: i32,
    _database_name: &str,
    _schema_name: Option<&str>,
    table_names: &[String],
    _timeout_duration: Duration,
) -> Result<SchemaContext, AiError> {
    // TODO: Implement Oracle introspection via oracle crate
    // Sử dụng ALL_TAB_COLUMNS, ALL_INDEXES, ALL_IND_COLUMNS
    if table_names.is_empty() {
        return Ok(SchemaContext { tables: vec![], indexes: vec![] });
    }
    tracing::warn!("Oracle schema introspection not yet implemented — AI analysis will proceed without schema context");
    Ok(SchemaContext { tables: vec![], indexes: vec![] })
}

/// Introspect cho MongoDB — trả empty (schema-less).
pub async fn introspect_mongodb(
    _table_names: &[String],
) -> Result<SchemaContext, AiError> {
    // MongoDB là schema-less, chỉ có thể sample documents
    Ok(SchemaContext { tables: vec![], indexes: vec![] })
}

// ── Internal SQL Row Types ─────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct PgColumnRow {
    column_name: String,
    data_type: String,
    is_nullable: String,
    is_pk: Option<bool>,
}

#[derive(Debug, sqlx::FromRow)]
struct MysqlColumnRow {
    column_name: String,
    data_type: String,
    is_nullable: String,
    column_key: String,
}

#[derive(Debug, sqlx::FromRow)]
struct MysqlIndexRow {
    index_name: String,
    column_name: String,
    non_unique: i32,
}

/// Query indexes từ PostgreSQL.
async fn query_pg_indexes(
    pool: &sqlx::PgPool,
    schema: &str,
    table_names: &[&str],
) -> Vec<IndexInfo> {
    let mut indexes: Vec<IndexInfo> = Vec::new();

    for &table_name in table_names {
        let rows: Vec<PgIndexRow> = sqlx::query_as(
            "SELECT i.relname as index_name, \
                    a.attname as column_name, \
                    ix.indisunique as is_unique \
             FROM pg_index ix \
             JOIN pg_class t ON t.oid = ix.indrelid \
             JOIN pg_class i ON i.oid = ix.indexrelid \
             JOIN pg_namespace n ON n.oid = t.relnamespace \
             JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) \
             WHERE n.nspname = $1 AND t.relname = $2 \
             ORDER BY i.relname, a.attnum"
        )
        .bind(schema)
        .bind(table_name)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        // Group by index name
        let mut idx_map: std::collections::HashMap<String, (Vec<String>, bool)> = std::collections::HashMap::new();
        for row in rows {
            let entry = idx_map.entry(row.index_name.clone()).or_insert_with(|| (Vec::new(), row.is_unique));
            entry.0.push(row.column_name);
        }

        for (idx_name, (cols, is_unique)) in idx_map {
            indexes.push(IndexInfo {
                name: idx_name,
                table_name: table_name.to_string(),
                columns: cols,
                is_unique,
            });
        }
    }

    indexes
}

#[derive(Debug, sqlx::FromRow)]
struct PgIndexRow {
    index_name: String,
    column_name: String,
    is_unique: bool,
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_table_names_simple_select() {
        let sql = "SELECT * FROM users WHERE id = 1";
        let tables = extract_table_names(sql);
        assert_eq!(tables, vec!["users"]);
    }

    #[test]
    fn test_extract_table_names_join() {
        let sql = "SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id";
        let tables = extract_table_names(sql);
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));
    }

    #[test]
    fn test_extract_table_names_insert() {
        let sql = "INSERT INTO audit_log (action, user_id) VALUES ('login', 1)";
        let tables = extract_table_names(sql);
        assert!(tables.contains(&"audit_log".to_string()));
    }

    #[test]
    fn test_extract_table_names_update() {
        let sql = "UPDATE users SET active = false WHERE last_login < '2024-01-01'";
        let tables = extract_table_names(sql);
        assert!(tables.contains(&"users".to_string()));
    }

    #[test]
    fn test_extract_table_names_delete() {
        let sql = "DELETE FROM temp_data WHERE created_at < NOW() - INTERVAL '30 days'";
        let tables = extract_table_names(sql);
        assert!(tables.contains(&"temp_data".to_string()));
    }

    #[test]
    fn test_extract_table_names_ddl() {
        let sql = "DROP TABLE old_sessions; ALTER TABLE users ADD COLUMN age INT";
        let tables = extract_table_names(sql);
        assert!(tables.contains(&"old_sessions".to_string()));
        assert!(tables.contains(&"users".to_string()));
    }

    #[test]
    fn test_extract_table_names_no_duplicates() {
        let sql = "SELECT * FROM users u JOIN orders o ON u.id = o.user_id WHERE u.id IN (SELECT user_id FROM users)";
        let tables = extract_table_names(sql);
        // "users" should appear only once
        assert_eq!(tables.iter().filter(|t| *t == "users").count(), 1);
    }

    #[test]
    fn test_extract_table_names_schema_qualified() {
        let sql = "SELECT * FROM public.users";
        let tables = extract_table_names(sql);
        assert!(tables.contains(&"public.users".to_string()));
    }

    #[test]
    fn test_sql_keyword_not_treated_as_table() {
        let sql = "SELECT * FROM users WHERE id IN (SELECT user_id FROM orders)";
        let tables = extract_table_names(sql);
        assert!(!tables.contains(&"select".to_string()));
        assert!(!tables.contains(&"where".to_string()));
    }
}
