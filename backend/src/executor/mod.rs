pub mod encryption;
pub mod mongodb;
pub mod mysql;
pub mod oracle;
pub mod postgres;
pub mod sql_analyzer;

use std::time::Duration;
use thiserror::Error;

/// Kết quả thực thi SQL thành công.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Tổng số rows bị ảnh hưởng bởi tất cả statements.
    pub rows_affected: i64,
}

/// Kết quả validate SQL (EXPLAIN) — không thực thi, chỉ kiểm tra.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationResult {
    /// SQL hợp lệ (syntax + schema đều OK)?
    pub valid: bool,
    /// Execution plan raw text (output của EXPLAIN)
    pub execution_plan: Vec<String>,
    /// Cảnh báo hiệu năng (seq scan, high cost, v.v.)
    pub warnings: Vec<String>,
    /// Lỗi nếu có (syntax sai, bảng không tồn tại, v.v.)
    pub error: Option<String>,
}

/// Lỗi khi thực thi SQL trên database đích.
#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("query execution failed: {0}")]
    QueryFailed(String),

    #[error("execution timed out after {0} seconds")]
    Timeout(u64),

    #[error("unsupported database type: {0}")]
    UnsupportedDbType(String),

    #[error("credential decryption failed: {0}")]
    DecryptionFailed(String),
}

/// Thông tin credentials đã giải mã để kết nối database đích.
#[derive(Debug, Clone)]
pub struct DbCredentials {
    pub username: String,
    pub password: String,
}

/// Trait chung cho tất cả database executors.
/// Mỗi loại DB (PostgreSQL, MySQL, Oracle, MongoDB) implement trait này.
///
/// Open/Closed Principle: thêm DB mới → impl trait, không sửa code cũ.
pub trait DatabaseExecutor: Send + Sync {
    /// Thực thi SQL trên database đích với timeout.
    /// Multi-statement SQL được wrap trong transaction (all-or-nothing).
    fn execute(
        &self,
        sql: &str,
        credentials: &DbCredentials,
        host: &str,
        port: i32,
        database_name: &str,
        schema_name: Option<&str>,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<ExecutionResult, ExecutorError>> + Send;

    /// Test kết nối đến database (không thực thi SQL).
    fn test_connection(
        &self,
        credentials: &DbCredentials,
        host: &str,
        port: i32,
        database_name: &str,
    ) -> impl std::future::Future<Output = Result<(), ExecutorError>> + Send;
}

/// Thực thi SQL dựa trên db_type, dispatch tới executor phù hợp.
///
/// Hàm này là entry-point chính để thực thi câu SQL trên database đích.
/// Dựa vào `db_type` sẽ chọn executor tương ứng (PostgreSQL, MySQL, Oracle, MongoDB).
///
/// # Parameters
/// - `db_type` — Loại database: `"postgresql"`, `"mysql"`, `"oracle"`, `"mongodb"`
/// - `sql` — Câu lệnh SQL cần thực thi (có thể multi-statement)
/// - `credentials` — Username/password đã giải mã
/// - `host` — Hostname hoặc IP của database đích
/// - `port` — Port kết nối
/// - `database_name` — Tên database
/// - `schema_name` — Schema (optional, tuỳ loại DB)
/// - `timeout` — Thời gian tối đa cho phép thực thi
///
/// # Errors
/// - [`ExecutorError::UnsupportedDbType`] nếu `db_type` không được hỗ trợ
/// - [`ExecutorError::ConnectionFailed`] nếu không kết nối được
/// - [`ExecutorError::QueryFailed`] nếu câu SQL lỗi
/// - [`ExecutorError::Timeout`] nếu quá thời gian cho phép
pub async fn execute_sql(
    db_type: &str,
    sql: &str,
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    schema_name: Option<&str>,
    timeout: Duration,
) -> Result<ExecutionResult, ExecutorError> {
    match db_type {
        "postgresql" => {
            postgres::PgExecutor
                .execute(sql, credentials, host, port, database_name, schema_name, timeout)
                .await
        }
        "mysql" => {
            mysql::MySqlExecutor
                .execute(sql, credentials, host, port, database_name, schema_name, timeout)
                .await
        }
        "oracle" => {
            oracle::OracleExecutor
                .execute(sql, credentials, host, port, database_name, schema_name, timeout)
                .await
        }
        "mongodb" => {
            mongodb::MongoExecutor
                .execute(sql, credentials, host, port, database_name, schema_name, timeout)
                .await
        }
        other => Err(ExecutorError::UnsupportedDbType(other.to_string())),
    }
}

/// Test kết nối đến database đích dựa trên db_type mà không thực thi SQL.
///
/// Dùng để xác minh thông tin kết nối (host, port, credentials) là hợp lệ
/// trước khi cho phép tạo hoặc cập nhật database connection trong hệ thống.
///
/// # Parameters
/// - `db_type` — Loại database: `"postgresql"`, `"mysql"`, `"oracle"`, `"mongodb"`
/// - `credentials` — Username/password đã giải mã
/// - `host` — Hostname hoặc IP của database đích
/// - `port` — Port kết nối
/// - `database_name` — Tên database
///
/// # Errors
/// - [`ExecutorError::UnsupportedDbType`] nếu `db_type` không được hỗ trợ
/// - [`ExecutorError::ConnectionFailed`] nếu không kết nối được
pub async fn test_db_connection(
    db_type: &str,
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
) -> Result<(), ExecutorError> {
    match db_type {
        "postgresql" => {
            postgres::PgExecutor
                .test_connection(credentials, host, port, database_name)
                .await
        }
        "mysql" => {
            mysql::MySqlExecutor
                .test_connection(credentials, host, port, database_name)
                .await
        }
        "oracle" => {
            oracle::OracleExecutor
                .test_connection(credentials, host, port, database_name)
                .await
        }
        "mongodb" => {
            mongodb::MongoExecutor
                .test_connection(credentials, host, port, database_name)
                .await
        }
        other => Err(ExecutorError::UnsupportedDbType(other.to_string())),
    }
}

/// Validate SQL bằng EXPLAIN — kiểm tra syntax, schema, và execution plan.
///
/// Không thực thi SQL, chỉ parse và generate execution plan.
/// Trả kết quả validation gồm: valid/invalid, execution plan, performance warnings.
///
/// # Hỗ trợ theo DB type
/// - **PostgreSQL**: `EXPLAIN <statement>` — full syntax + schema + cost
/// - **MySQL**: `EXPLAIN <statement>` — full syntax + schema + index usage
/// - **Oracle**: `EXPLAIN PLAN FOR <statement>` — full syntax + schema + cost
/// - **MongoDB**: Validate JSON structure (schema-less nên chỉ check format)
pub async fn validate_sql(
    db_type: &str,
    sql: &str,
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    schema_name: Option<&str>,
    timeout: Duration,
) -> ValidationResult {
    match db_type {
        "postgresql" => {
            validate_sql_postgres(sql, credentials, host, port, database_name, schema_name, timeout).await
        }
        "mysql" => {
            validate_sql_mysql(sql, credentials, host, port, database_name, timeout).await
        }
        "oracle" => {
            validate_sql_oracle(sql, credentials, host, port, database_name, schema_name, timeout).await
        }
        "mongodb" => {
            validate_sql_mongodb(sql)
        }
        other => ValidationResult {
            valid: false,
            execution_plan: vec![],
            warnings: vec![],
            error: Some(format!("Unsupported database type: {other}")),
        },
    }
}

/// Validate SQL trên PostgreSQL bằng EXPLAIN.
async fn validate_sql_postgres(
    sql: &str,
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    schema_name: Option<&str>,
    timeout_duration: Duration,
) -> ValidationResult {
    use tokio::time::timeout;

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
    })
    .await
    {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            return ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some(format!("Connection failed: {e}")),
            };
        }
        Err(_) => {
            return ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some("Connection timed out".to_string()),
            };
        }
    };

    // Split multi-statement
    let statements: Vec<&str> = sql.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let mut all_plans: Vec<String> = Vec::new();
    let mut all_warnings: Vec<String> = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        let explain_query = format!("EXPLAIN {stmt}");
        match sqlx::query_scalar::<_, String>(&explain_query)
            .fetch_all(&pool)
            .await
        {
            Ok(rows) => {
                if statements.len() > 1 {
                    all_plans.push(format!("--- Statement {} ---", i + 1));
                }
                // Analyze execution plan for performance warnings
                for row in &rows {
                    let lower = row.to_lowercase();
                    if lower.contains("seq scan") {
                        all_warnings.push(format!("Statement {}: Sequential Scan detected — có thể thiếu index", i + 1));
                    }
                    if lower.contains("cost=") {
                        if let Some(cost_start) = lower.find("cost=") {
                            let cost_part = &lower[cost_start..];
                            if let Some(end) = cost_part.find(' ') {
                                let cost_str = &cost_part[5..end];
                                if let Some(total_cost) = cost_str.split("..").last() {
                                    if let Ok(cost) = total_cost.parse::<f64>() {
                                        if cost > 10000.0 {
                                            all_warnings.push(format!("Statement {}: High estimated cost ({:.0}) — query có thể chậm", i + 1, cost));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                all_plans.extend(rows);
            }
            Err(e) => {
                pool.close().await;
                return ValidationResult {
                    valid: false,
                    execution_plan: all_plans,
                    warnings: all_warnings,
                    error: Some(format!("Statement {}: {}", i + 1, e)),
                };
            }
        }
    }

    pool.close().await;

    ValidationResult {
        valid: true,
        execution_plan: all_plans,
        warnings: all_warnings,
        error: None,
    }
}

/// Validate SQL trên MySQL bằng EXPLAIN.
async fn validate_sql_mysql(
    sql: &str,
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    timeout_duration: Duration,
) -> ValidationResult {
    use tokio::time::timeout;

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
    })
    .await
    {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            return ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some(format!("Connection failed: {e}")),
            };
        }
        Err(_) => {
            return ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some("Connection timed out".to_string()),
            };
        }
    };

    let statements: Vec<&str> = sql.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let mut all_plans: Vec<String> = Vec::new();
    let mut all_warnings: Vec<String> = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        let explain_query = format!("EXPLAIN {stmt}");
        match sqlx::query(&explain_query).fetch_all(&pool).await {
            Ok(rows) => {
                if statements.len() > 1 {
                    all_plans.push(format!("--- Statement {} ---", i + 1));
                }
                for row in &rows {
                    use sqlx::Row;
                    let select_type: Option<String> = row.try_get("select_type").ok().flatten();
                    let table: Option<String> = row.try_get("table").ok().flatten();
                    let access_type: Option<String> = row.try_get("type").ok().flatten();
                    let key: Option<String> = row.try_get("key").ok().flatten();
                    let rows_est: Option<i64> = row.try_get("rows").ok().flatten();
                    let extra: Option<String> = row.try_get("Extra").ok().flatten();

                    let plan_line = format!(
                        "type={}, table={}, access={}, key={}, rows={}, extra={}",
                        select_type.as_deref().unwrap_or("-"),
                        table.as_deref().unwrap_or("-"),
                        access_type.as_deref().unwrap_or("-"),
                        key.as_deref().unwrap_or("NULL"),
                        rows_est.map(|r| r.to_string()).unwrap_or("?".to_string()),
                        extra.as_deref().unwrap_or("")
                    );
                    all_plans.push(plan_line);

                    if access_type.as_deref() == Some("ALL") {
                        all_warnings.push(format!("Statement {}: Full table scan on '{}' — thiếu index", i + 1, table.as_deref().unwrap_or("?")));
                    }
                    if let Some(r) = rows_est {
                        if r > 100_000 {
                            all_warnings.push(format!("Statement {}: Estimated {} rows trên '{}' — query có thể chậm", i + 1, r, table.as_deref().unwrap_or("?")));
                        }
                    }
                }
            }
            Err(e) => {
                pool.close().await;
                return ValidationResult {
                    valid: false,
                    execution_plan: all_plans,
                    warnings: all_warnings,
                    error: Some(format!("Statement {}: {}", i + 1, e)),
                };
            }
        }
    }

    pool.close().await;

    ValidationResult {
        valid: true,
        execution_plan: all_plans,
        warnings: all_warnings,
        error: None,
    }
}

/// Validate SQL trên Oracle bằng EXPLAIN PLAN FOR.
async fn validate_sql_oracle(
    sql: &str,
    credentials: &DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    schema_name: Option<&str>,
    timeout_duration: Duration,
) -> ValidationResult {
    use tokio::time::timeout;

    let connect_string = format!(
        "(DESCRIPTION=(ADDRESS=(PROTOCOL=TCPS)(HOST={host})(PORT={port}))\
         (CONNECT_DATA=(SERVICE_NAME={database_name}))\
         (SECURITY=(SSL_SERVER_DN_MATCH=YES)))"
    );
    let username = credentials.username.clone();
    let password = credentials.password.clone();
    let sql_owned = sql.to_string();
    let schema_owned = schema_name.map(|s| s.to_string());

    let result = timeout(timeout_duration, tokio::task::spawn_blocking(move || {
        validate_sql_oracle_blocking(&connect_string, &username, &password, &sql_owned, schema_owned.as_deref())
    }))
    .await;

    match result {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => ValidationResult {
            valid: false,
            execution_plan: vec![],
            warnings: vec![],
            error: Some(format!("Task error: {e}")),
        },
        Err(_) => ValidationResult {
            valid: false,
            execution_plan: vec![],
            warnings: vec![],
            error: Some("Validation timed out".to_string()),
        },
    }
}

/// Oracle EXPLAIN PLAN blocking implementation (chạy trên blocking thread).
///
/// Kết nối Oracle qua TCPS, set schema nếu có, sau đó chạy `EXPLAIN PLAN FOR`
/// cho từng statement. Kết quả plan được đọc từ `PLAN_TABLE` và phân tích
/// cảnh báo hiệu năng (full table access, high cost).
///
/// # Note
/// Hàm này chạy synchronous (blocking) — phải gọi qua `spawn_blocking`.
#[cfg(feature = "oracle-support")]
fn validate_sql_oracle_blocking(
    connect_string: &str,
    username: &str,
    password: &str,
    sql: &str,
    schema: Option<&str>,
) -> ValidationResult {
    use oracle::Connection;

    let conn = match Connection::connect(username, password, connect_string) {
        Ok(c) => c,
        Err(e) => {
            return ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some(format!("Connection failed: {e}")),
            };
        }
    };

    if let Some(schema_name) = schema {
        if let Err(e) = conn.execute(&format!("ALTER SESSION SET CURRENT_SCHEMA = {schema_name}"), &[]) {
            return ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some(format!("Failed to set schema: {e}")),
            };
        }
    }

    let statements: Vec<&str> = sql.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    let mut all_plans: Vec<String> = Vec::new();
    let mut all_warnings: Vec<String> = Vec::new();

    for (i, stmt) in statements.iter().enumerate() {
        let _ = conn.execute("DELETE FROM PLAN_TABLE WHERE STATEMENT_ID = 'SECS_VALIDATE'", &[]);
        let _ = conn.commit();

        let explain = format!("EXPLAIN PLAN SET STATEMENT_ID = 'SECS_VALIDATE' FOR {stmt}");
        if let Err(e) = conn.execute(&explain, &[]) {
            return ValidationResult {
                valid: false,
                execution_plan: all_plans,
                warnings: all_warnings,
                error: Some(format!("Statement {}: {}", i + 1, e)),
            };
        }

        match conn.query(
            "SELECT LPAD(' ', 2 * LEVEL) || OPERATION || ' ' || NVL(OPTIONS, '') || ' ' || NVL(OBJECT_NAME, '') AS PLAN_LINE, COST \
             FROM PLAN_TABLE WHERE STATEMENT_ID = 'SECS_VALIDATE' \
             CONNECT BY PRIOR ID = PARENT_ID START WITH PARENT_ID IS NULL \
             ORDER SIBLINGS BY ID",
            &[],
        ) {
            Ok(rows) => {
                if statements.len() > 1 {
                    all_plans.push(format!("--- Statement {} ---", i + 1));
                }
                for row_result in rows {
                    if let Ok(row) = row_result {
                        let plan_line: String = row.get(0).unwrap_or_default();
                        let cost: Option<f64> = row.get(1).ok();
                        all_plans.push(plan_line.clone());

                        if plan_line.contains("TABLE ACCESS FULL") {
                            all_warnings.push(format!("Statement {}: Full Table Access detected — thiếu index", i + 1));
                        }
                        if let Some(c) = cost {
                            if c > 10000.0 {
                                all_warnings.push(format!("Statement {}: High cost ({:.0})", i + 1, c));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return ValidationResult {
                    valid: false,
                    execution_plan: all_plans,
                    warnings: all_warnings,
                    error: Some(format!("Statement {}: Failed to read plan: {}", i + 1, e)),
                };
            }
        }
    }

    let _ = conn.execute("DELETE FROM PLAN_TABLE WHERE STATEMENT_ID = 'SECS_VALIDATE'", &[]);
    let _ = conn.commit();

    ValidationResult {
        valid: true,
        execution_plan: all_plans,
        warnings: all_warnings,
        error: None,
    }
}

/// Stub implementation khi feature `oracle-support` không được bật.
///
/// Trả về lỗi cho biết Oracle chưa được compile — người dùng cần enable feature.
#[cfg(not(feature = "oracle-support"))]
fn validate_sql_oracle_blocking(
    _connect_string: &str,
    _username: &str,
    _password: &str,
    _sql: &str,
    _schema: Option<&str>,
) -> ValidationResult {
    ValidationResult {
        valid: false,
        execution_plan: vec![],
        warnings: vec![],
        error: Some("Oracle support not compiled (enable feature 'oracle-support')".to_string()),
    }
}

/// Validate MongoDB command — kiểm tra JSON structure.
/// MongoDB là schema-less nên chỉ validate format, không check collection existence.
fn validate_sql_mongodb(input: &str) -> ValidationResult {
    let trimmed = input.trim();

    if trimmed.starts_with('[') {
        match serde_json::from_str::<Vec<serde_json::Value>>(trimmed) {
            Ok(values) => {
                if values.is_empty() {
                    return ValidationResult {
                        valid: false,
                        execution_plan: vec![],
                        warnings: vec![],
                        error: Some("Empty command array".to_string()),
                    };
                }
                for (i, v) in values.iter().enumerate() {
                    if !v.is_object() {
                        return ValidationResult {
                            valid: false,
                            execution_plan: vec![],
                            warnings: vec![],
                            error: Some(format!("Command {}: must be a JSON object", i + 1)),
                        };
                    }
                }
                ValidationResult {
                    valid: true,
                    execution_plan: vec![format!("{} command(s) — valid JSON structure", values.len())],
                    warnings: vec!["MongoDB: validation chỉ kiểm tra JSON format, không verify collection/field existence".to_string()],
                    error: None,
                }
            }
            Err(e) => ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some(format!("Invalid JSON array: {e}")),
            },
        }
    } else if trimmed.starts_with('{') {
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(v) if v.is_object() => ValidationResult {
                valid: true,
                execution_plan: vec!["1 command — valid JSON structure".to_string()],
                warnings: vec!["MongoDB: validation chỉ kiểm tra JSON format, không verify collection/field existence".to_string()],
                error: None,
            },
            Ok(_) => ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some("Command must be a JSON object".to_string()),
            },
            Err(e) => ValidationResult {
                valid: false,
                execution_plan: vec![],
                warnings: vec![],
                error: Some(format!("Invalid JSON: {e}")),
            },
        }
    } else {
        ValidationResult {
            valid: false,
            execution_plan: vec![],
            warnings: vec![],
            error: Some("Input must be a JSON object or array".to_string()),
        }
    }
}
