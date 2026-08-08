//! Schema Browser API — cho phép frontend browse tables/columns từ database đích.
//!
//! Endpoints:
//! - `GET /api/v1/databases/:id/tables?schema=X` — liệt kê tables
//! - `GET /api/v1/databases/:id/columns?schema=X&table=Y` — liệt kê columns của table

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::auth::extractors::RequireAuth;
use crate::db;
use crate::error::AppError;
use crate::executor::encryption::decrypt_credentials;
use crate::executor::DbCredentials;
use crate::models::database::DbType;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

/// Query parameters cho endpoint list tables.
#[derive(Debug, Deserialize)]
pub struct ListTablesQuery {
    /// Schema cần browse. Nếu không truyền, dùng schema mặc định theo DB type.
    pub schema: Option<String>,
}

/// Query parameters cho endpoint list columns.
#[derive(Debug, Deserialize)]
pub struct ListColumnsQuery {
    /// Schema cần browse. Nếu không truyền, dùng schema mặc định theo DB type.
    pub schema: Option<String>,
    /// Tên table cần liệt kê columns.
    pub table: String,
}

/// Response chứa danh sách tên tables trong database đích.
#[derive(Debug, Serialize)]
pub struct TableListResponse {
    /// Danh sách tên tables, đã sắp xếp theo alphabet.
    pub tables: Vec<String>,
}

/// Thông tin metadata của một column trong table.
#[derive(Debug, Serialize)]
pub struct ColumnInfo {
    /// Tên column.
    pub name: String,
    /// Kiểu dữ liệu (ví dụ: `varchar`, `integer`, `timestamp`).
    pub data_type: String,
    /// `true` nếu column cho phép NULL.
    pub is_nullable: bool,
    /// `true` nếu column là primary key.
    pub is_primary_key: bool,
}

/// Response chứa danh sách columns của một table.
#[derive(Debug, Serialize)]
pub struct ColumnListResponse {
    /// Danh sách columns, sắp xếp theo ordinal position.
    pub columns: Vec<ColumnInfo>,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/databases/:id/tables` — Liệt kê tables trong database đích.
///
/// User phải có permission trên database này.
/// Query param `schema` xác định schema cần browse (default: public / dbo / database_name tuỳ DB type).
pub async fn list_tables(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
    Query(params): Query<ListTablesQuery>,
) -> Result<Json<TableListResponse>, AppError> {
    // Check permission
    let has_perm = db::permissions::check_permission(&state.db, claims.user_id, id)
        .await
        .map_err(AppError::Database)?;
    if !has_perm {
        return Err(AppError::DatabasePermissionDenied { db_id: id });
    }

    // Get database connection info
    let conn = db::databases::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Decrypt credentials
    let key_bytes = state.config.encryption_key_bytes()
        .map_err(|e| AppError::Encryption(e.to_string()))?;
    let credentials = decrypt_credentials(&conn.encrypted_credentials, &key_bytes)
        .map_err(|e| AppError::Encryption(e.to_string()))?;

    let schema = params.schema
        .or(conn.schema_name.clone())
        .unwrap_or_else(|| default_schema(&conn.db_type));

    let timeout = Duration::from_secs(15);

    let tables = match &conn.db_type {
        DbType::PostgreSQL => {
            list_pg_tables(&credentials, &conn.host, conn.port, &conn.database_name, &schema, timeout).await?
        }
        DbType::MySQL => {
            list_mysql_tables(&credentials, &conn.host, conn.port, &conn.database_name, timeout).await?
        }
        DbType::Oracle => {
            list_oracle_tables(&credentials, &conn.host, conn.port, &conn.database_name, &schema, timeout).await?
        }
        DbType::MongoDB => {
            list_mongodb_collections(&credentials, &conn.host, conn.port, &conn.database_name, timeout).await?
        }
    };

    Ok(Json(TableListResponse { tables }))
}

/// `GET /api/v1/databases/:id/columns` — Liệt kê columns của table trong database đích.
///
/// User phải có permission trên database này.
pub async fn list_columns(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
    Query(params): Query<ListColumnsQuery>,
) -> Result<Json<ColumnListResponse>, AppError> {
    // Check permission
    let has_perm = db::permissions::check_permission(&state.db, claims.user_id, id)
        .await
        .map_err(AppError::Database)?;
    if !has_perm {
        return Err(AppError::DatabasePermissionDenied { db_id: id });
    }

    // Get database connection info
    let conn = db::databases::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Decrypt credentials
    let key_bytes = state.config.encryption_key_bytes()
        .map_err(|e| AppError::Encryption(e.to_string()))?;
    let credentials = decrypt_credentials(&conn.encrypted_credentials, &key_bytes)
        .map_err(|e| AppError::Encryption(e.to_string()))?;

    let schema = params.schema
        .or(conn.schema_name.clone())
        .unwrap_or_else(|| default_schema(&conn.db_type));

    let timeout = Duration::from_secs(15);

    let columns = match &conn.db_type {
        DbType::PostgreSQL => {
            list_pg_columns(&credentials, &conn.host, conn.port, &conn.database_name, &schema, &params.table, timeout).await?
        }
        DbType::MySQL => {
            list_mysql_columns(&credentials, &conn.host, conn.port, &conn.database_name, &params.table, timeout).await?
        }
        DbType::Oracle => {
            list_oracle_columns(&credentials, &conn.host, conn.port, &conn.database_name, &schema, &params.table, timeout).await?
        }
        DbType::MongoDB => {
            // MongoDB collections don't have fixed schema — return empty
            vec![]
        }
    };

    Ok(Json(ColumnListResponse { columns }))
}

// ── Internal Helpers ───────────────────────────────────────────────────

/// Trả về schema mặc định dựa trên loại database.
///
/// - PostgreSQL: `"public"`
/// - Oracle / các loại khác: chuỗi rỗng (dùng owner hoặc database name).
fn default_schema(db_type: &DbType) -> String {
    match db_type {
        DbType::PostgreSQL => "public".to_string(),
        DbType::Oracle => "".to_string(),
        _ => "".to_string(),
    }
}

/// List tables từ PostgreSQL qua `information_schema.tables`.
///
/// Kết nối đến PostgreSQL đích, query danh sách base tables trong schema chỉ định,
/// trả về danh sách tên tables đã sắp xếp theo alphabet.
///
/// # Errors
///
/// - `AppError::ValidationError` nếu connection timeout, connection failed, hoặc query lỗi.
async fn list_pg_tables(
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    schema: &str,
    timeout: Duration,
) -> Result<Vec<String>, AppError> {
    use tokio::time::timeout as tokio_timeout;

    let tls_disabled = std::env::var("DISABLE_TARGET_DB_TLS")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");
    let sslmode = if tls_disabled { "disable" } else { "require" };

    let conn_str = format!(
        "postgres://{}:{}@{}:{}/{}?sslmode={}",
        urlencoding::encode(&credentials.username),
        urlencoding::encode(&credentials.password),
        host,
        port,
        database_name,
        sslmode
    );

    let pool = tokio_timeout(timeout, async {
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&conn_str)
            .await
    })
    .await
    .map_err(|_| AppError::ValidationError("Connection timed out".to_string()))?
    .map_err(|e| AppError::ValidationError(format!("Connection failed: {e}")))?;

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = $1 AND table_type = 'BASE TABLE' \
         ORDER BY table_name"
    )
    .bind(schema)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::ValidationError(format!("Query failed: {e}")))?;

    pool.close().await;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// List columns từ PostgreSQL qua `information_schema.columns`.
///
/// Kết nối đến PostgreSQL đích, query metadata columns (tên, kiểu dữ liệu, nullable, primary key)
/// của table chỉ định, trả về danh sách sắp xếp theo ordinal position.
///
/// # Errors
///
/// - `AppError::ValidationError` nếu connection timeout, connection failed, hoặc query lỗi.
async fn list_pg_columns(
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    schema: &str,
    table: &str,
    timeout: Duration,
) -> Result<Vec<ColumnInfo>, AppError> {
    use tokio::time::timeout as tokio_timeout;

    let tls_disabled = std::env::var("DISABLE_TARGET_DB_TLS")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");
    let sslmode = if tls_disabled { "disable" } else { "require" };

    let conn_str = format!(
        "postgres://{}:{}@{}:{}/{}?sslmode={}",
        urlencoding::encode(&credentials.username),
        urlencoding::encode(&credentials.password),
        host,
        port,
        database_name,
        sslmode
    );

    let pool = tokio_timeout(timeout, async {
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&conn_str)
            .await
    })
    .await
    .map_err(|_| AppError::ValidationError("Connection timed out".to_string()))?
    .map_err(|e| AppError::ValidationError(format!("Connection failed: {e}")))?;

    #[derive(sqlx::FromRow)]
    struct ColRow {
        column_name: String,
        data_type: String,
        is_nullable: String,
        is_pk: Option<bool>,
    }

    let rows: Vec<ColRow> = sqlx::query_as(
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
    .bind(table)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::ValidationError(format!("Query failed: {e}")))?;

    pool.close().await;

    Ok(rows.into_iter().map(|r| ColumnInfo {
        name: r.column_name,
        data_type: r.data_type,
        is_nullable: r.is_nullable == "YES",
        is_primary_key: r.is_pk.unwrap_or(false),
    }).collect())
}

/// List tables từ MySQL qua `information_schema.TABLES`.
///
/// Kết nối đến MySQL đích, query danh sách base tables trong database chỉ định.
/// MySQL không có khái niệm schema tách biệt — `database_name` đóng vai trò schema.
///
/// # Errors
///
/// - `AppError::ValidationError` nếu connection timeout, connection failed, hoặc query lỗi.
async fn list_mysql_tables(
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    timeout: Duration,
) -> Result<Vec<String>, AppError> {
    use tokio::time::timeout as tokio_timeout;

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

    let pool = tokio_timeout(timeout, async {
        sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&conn_str)
            .await
    })
    .await
    .map_err(|_| {
        tracing::warn!(host, port, database_name, "MySQL table listing timed out");
        AppError::ValidationError("Connection timed out".to_string())
    })?
    .map_err(|e| {
        tracing::warn!(host, port, database_name, error = %e, "MySQL connection failed for table listing");
        AppError::ValidationError(format!("Connection failed: {e}"))
    })?;

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT CAST(TABLE_NAME AS CHAR(255)) FROM information_schema.TABLES \
         WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = 'BASE TABLE' \
         ORDER BY TABLE_NAME"
    )
    .bind(database_name)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        tracing::warn!(database_name, error = %e, "MySQL table listing failed");
        AppError::ValidationError(format!("Query failed: {e}"))
    })?;

    pool.close().await;
    Ok(rows.into_iter().map(|(name,)| name).collect())
}

/// List columns từ MySQL qua `information_schema.COLUMNS`.
///
/// Kết nối đến MySQL đích, query metadata columns (tên, kiểu dữ liệu, nullable, primary key)
/// của table chỉ định, trả về danh sách sắp xếp theo ordinal position.
///
/// # Errors
///
/// - `AppError::ValidationError` nếu connection timeout, connection failed, hoặc query lỗi.
async fn list_mysql_columns(
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    table: &str,
    timeout: Duration,
) -> Result<Vec<ColumnInfo>, AppError> {
    use tokio::time::timeout as tokio_timeout;

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

    let pool = tokio_timeout(timeout, async {
        sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&conn_str)
            .await
    })
    .await
    .map_err(|_| AppError::ValidationError("Connection timed out".to_string()))?
    .map_err(|e| AppError::ValidationError(format!("Connection failed: {e}")))?;

    #[derive(sqlx::FromRow)]
    struct ColRow {
        field: String,
        col_type: String,
        null_val: String,
        key_val: String,
    }

    let rows: Vec<ColRow> = sqlx::query_as(
        "SELECT CAST(COLUMN_NAME AS CHAR(255)) as field, \
                CAST(COLUMN_TYPE AS CHAR(255)) as col_type, \
                CAST(IS_NULLABLE AS CHAR(10)) as null_val, \
                CAST(COLUMN_KEY AS CHAR(10)) as key_val \
         FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? \
         ORDER BY ORDINAL_POSITION"
    )
    .bind(database_name)
    .bind(table)
    .fetch_all(&pool)
    .await
    .map_err(|e| AppError::ValidationError(format!("Query failed: {e}")))?;

    pool.close().await;

    Ok(rows.into_iter().map(|r| ColumnInfo {
        name: r.field,
        data_type: r.col_type,
        is_nullable: r.null_val == "YES",
        is_primary_key: r.key_val == "PRI",
    }).collect())
}

/// List tables từ Oracle — placeholder (chưa implement).
///
/// Khi hoàn thiện sẽ query `ALL_TABLES` để lấy danh sách tables thuộc schema chỉ định.
/// Hiện tại trả về danh sách rỗng và log warning.
async fn list_oracle_tables(
    _credentials: &DbCredentials,
    _host: &str,
    _port: i32,
    _database_name: &str,
    _schema: &str,
    _timeout: Duration,
) -> Result<Vec<String>, AppError> {
    // TODO: Implement Oracle table listing via ALL_TABLES
    tracing::warn!("Oracle table listing not yet implemented");
    Ok(vec![])
}

/// List columns từ Oracle — placeholder (chưa implement).
///
/// Khi hoàn thiện sẽ query `ALL_TAB_COLUMNS` để lấy metadata columns.
/// Hiện tại trả về danh sách rỗng và log warning.
async fn list_oracle_columns(
    _credentials: &DbCredentials,
    _host: &str,
    _port: i32,
    _database_name: &str,
    _schema: &str,
    _table: &str,
    _timeout: Duration,
) -> Result<Vec<ColumnInfo>, AppError> {
    // TODO: Implement Oracle column listing via ALL_TAB_COLUMNS
    tracing::warn!("Oracle column listing not yet implemented");
    Ok(vec![])
}

/// List collections từ MongoDB database đích.
///
/// Kết nối đến MongoDB, gọi `list_collection_names()` để lấy danh sách collections
/// trong database chỉ định.
///
/// # Errors
///
/// - `AppError::ValidationError` nếu connection timeout, parse options lỗi,
///   hoặc không thể list collections.
async fn list_mongodb_collections(
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    timeout: Duration,
) -> Result<Vec<String>, AppError> {
    use tokio::time::timeout as tokio_timeout;

    let tls_disabled = std::env::var("DISABLE_TARGET_DB_TLS")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");
    let tls_param = if tls_disabled { "tls=false" } else { "tls=true" };

    let conn_str = format!(
        "mongodb://{}:{}@{}:{}/{}?{}",
        urlencoding::encode(&credentials.username),
        urlencoding::encode(&credentials.password),
        host,
        port,
        database_name,
        tls_param
    );

    let client = tokio_timeout(timeout, async {
        let options = mongodb::options::ClientOptions::parse(&conn_str)
            .await
            .map_err(|e| AppError::ValidationError(format!("MongoDB connection failed: {e}")))?;
        Ok::<_, AppError>(mongodb::Client::with_options(options)
            .map_err(|e| AppError::ValidationError(format!("MongoDB client error: {e}")))?)
    })
    .await
    .map_err(|_| AppError::ValidationError("Connection timed out".to_string()))??;

    let db = client.database(database_name);
    let collections = db.list_collection_names()
        .await
        .map_err(|e| AppError::ValidationError(format!("Failed to list collections: {e}")))?;

    Ok(collections)
}
