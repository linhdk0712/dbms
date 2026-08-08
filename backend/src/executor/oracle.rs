//! Oracle Database Executor — sử dụng `oracle` crate (OCI native).
//!
//! # Yêu cầu
//! - Oracle Instant Client phải được cài trên máy
//! - Feature `oracle-support` phải enabled (default)
//!
//! # Architecture
//! - `oracle` crate là synchronous → wrap trong `tokio::task::spawn_blocking`
//! - Multi-statement SQL: tách theo `;`, execute lần lượt trong 1 connection
//! - Transaction: explicit COMMIT ở cuối, ROLLBACK nếu có lỗi
//! - Timeout: `tokio::time::timeout` bọc toàn bộ blocking task

use std::time::Duration;
use tokio::time::timeout;

use super::{DatabaseExecutor, DbCredentials, ExecutionResult, ExecutorError};

/// Oracle executor — kết nối đến Oracle Database qua OCI.
pub struct OracleExecutor;

impl DatabaseExecutor for OracleExecutor {
    async fn execute(
        &self,
        sql: &str,
        credentials: &DbCredentials,
        host: &str,
        port: i32,
        database_name: &str,
        schema_name: Option<&str>,
        timeout_duration: Duration,
    ) -> Result<ExecutionResult, ExecutorError> {
        let connect_string = build_connect_string(host, port, database_name);
        let username = credentials.username.clone();
        let password = credentials.password.clone();
        let sql_owned = sql.to_string();
        let schema_owned = schema_name.map(|s| s.to_string());

        // oracle crate is synchronous — run in blocking thread
        let result = timeout(timeout_duration, tokio::task::spawn_blocking(move || {
            execute_blocking(&connect_string, &username, &password, &sql_owned, schema_owned.as_deref())
        }))
        .await
        .map_err(|_| ExecutorError::Timeout(timeout_duration.as_secs()))?
        .map_err(|e| ExecutorError::QueryFailed(format!("task join error: {e}")))?;

        result
    }

    async fn test_connection(
        &self,
        credentials: &DbCredentials,
        host: &str,
        port: i32,
        database_name: &str,
    ) -> Result<(), ExecutorError> {
        let connect_string = build_connect_string(host, port, database_name);
        let username = credentials.username.clone();
        let password = credentials.password.clone();

        let result = tokio::task::spawn_blocking(move || {
            test_connection_blocking(&connect_string, &username, &password)
        })
        .await
        .map_err(|e| ExecutorError::QueryFailed(format!("task join error: {e}")))?;

        result
    }
}

/// Build Oracle connect string (TNS descriptor format with TCPS).
/// Enforces TLS encryption via TCPS protocol and SSL_SERVER_DN_MATCH.
fn build_connect_string(host: &str, port: i32, service_name: &str) -> String {
    format!(
        "(DESCRIPTION=(ADDRESS=(PROTOCOL=TCPS)(HOST={host})(PORT={port}))\
         (CONNECT_DATA=(SERVICE_NAME={service_name}))\
         (SECURITY=(SSL_SERVER_DN_MATCH=YES)))"
    )
}

/// Thực thi SQL blocking trên Oracle connection.
/// Multi-statement: tách theo `;`, execute tuần tự, COMMIT cuối cùng.
/// Bất kỳ lỗi nào → ROLLBACK toàn bộ.
#[cfg(feature = "oracle-support")]
fn execute_blocking(
    connect_string: &str,
    username: &str,
    password: &str,
    sql: &str,
    schema: Option<&str>,
) -> Result<ExecutionResult, ExecutorError> {
    use oracle::Connection;

    // Kết nối
    let conn = Connection::connect(username, password, connect_string)
        .map_err(|e| ExecutorError::ConnectionFailed(e.to_string()))?;

    // Set schema nếu cần (ALTER SESSION SET CURRENT_SCHEMA)
    if let Some(schema_name) = schema {
        // Validate schema name to prevent SQL injection
        super::validate_identifier(schema_name)?;
        conn.execute(
            &format!("ALTER SESSION SET CURRENT_SCHEMA = \"{}\"", schema_name),
            &[],
        )
        .map_err(|e| ExecutorError::QueryFailed(format!("failed to set schema: {e}")))?;
    }

    // Tách multi-statement
    let statements: Vec<&str> = sql
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut total_rows: i64 = 0;

    for stmt in &statements {
        match conn.execute(stmt, &[]) {
            Ok(()) => {
                // Lấy rows affected từ statement
                let rows = conn.row_count()
                    .map_err(|e| ExecutorError::QueryFailed(format!("failed to get row count: {e}")))?;
                total_rows += rows as i64;
            }
            Err(e) => {
                // Rollback on error
                let _ = conn.rollback();
                return Err(ExecutorError::QueryFailed(e.to_string()));
            }
        }
    }

    // Commit toàn bộ
    conn.commit()
        .map_err(|e| ExecutorError::QueryFailed(format!("commit failed: {e}")))?;

    Ok(ExecutionResult {
        rows_affected: total_rows,
    })
}

/// Fallback khi feature oracle-support bị disable.
#[cfg(not(feature = "oracle-support"))]
fn execute_blocking(
    _connect_string: &str,
    _username: &str,
    _password: &str,
    _sql: &str,
    _schema: Option<&str>,
) -> Result<ExecutionResult, ExecutorError> {
    Err(ExecutorError::UnsupportedDbType(
        "Oracle support not compiled (enable feature 'oracle-support')".to_string(),
    ))
}

/// Test connection blocking — thực hiện SELECT 1 FROM DUAL.
#[cfg(feature = "oracle-support")]
fn test_connection_blocking(
    connect_string: &str,
    username: &str,
    password: &str,
) -> Result<(), ExecutorError> {
    use oracle::Connection;

    let conn = Connection::connect(username, password, connect_string)
        .map_err(|e| ExecutorError::ConnectionFailed(e.to_string()))?;

    // Oracle standard test query
    conn.query_row_as::<i32>("SELECT 1 FROM DUAL", &[])
        .map_err(|e| ExecutorError::QueryFailed(e.to_string()))?;

    Ok(())
}

/// Fallback khi feature oracle-support bị disable.
#[cfg(not(feature = "oracle-support"))]
fn test_connection_blocking(
    _connect_string: &str,
    _username: &str,
    _password: &str,
) -> Result<(), ExecutorError> {
    Err(ExecutorError::UnsupportedDbType(
        "Oracle support not compiled (enable feature 'oracle-support')".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_connect_string_uses_tcps() {
        let result = build_connect_string("oracle-host.company.com", 1521, "PRODDB");
        assert_eq!(
            result,
            "(DESCRIPTION=(ADDRESS=(PROTOCOL=TCPS)(HOST=oracle-host.company.com)(PORT=1521))\
             (CONNECT_DATA=(SERVICE_NAME=PRODDB))\
             (SECURITY=(SSL_SERVER_DN_MATCH=YES)))"
        );
        assert!(result.contains("PROTOCOL=TCPS"));
        assert!(result.contains("SSL_SERVER_DN_MATCH=YES"));
        assert!(!result.contains("//"));
    }

    #[test]
    fn test_build_connect_string_with_custom_port() {
        let result = build_connect_string("10.0.1.50", 1522, "TESTDB");
        assert_eq!(
            result,
            "(DESCRIPTION=(ADDRESS=(PROTOCOL=TCPS)(HOST=10.0.1.50)(PORT=1522))\
             (CONNECT_DATA=(SERVICE_NAME=TESTDB))\
             (SECURITY=(SSL_SERVER_DN_MATCH=YES)))"
        );
        assert!(result.contains("PROTOCOL=TCPS"));
        assert!(result.contains("SSL_SERVER_DN_MATCH=YES"));
    }

    #[test]
    fn test_build_connect_string_no_plain_tcp() {
        let result = build_connect_string("db.example.com", 1521, "SVC");
        // Must NOT contain plain TCP — only TCPS
        assert!(!result.contains("PROTOCOL=TCP)"));
        assert!(result.contains("PROTOCOL=TCPS)"));
    }
}
