use std::time::Duration;
use tokio::time::timeout;

use super::{DatabaseExecutor, DbCredentials, ExecutionResult, ExecutorError};

/// MySQL executor — kết nối trực tiếp đến target MySQL database.
pub struct MySqlExecutor;

impl DatabaseExecutor for MySqlExecutor {
    async fn execute(
        &self,
        sql: &str,
        credentials: &DbCredentials,
        host: &str,
        port: i32,
        database_name: &str,
        _schema_name: Option<&str>,
        timeout_duration: Duration,
    ) -> Result<ExecutionResult, ExecutorError> {
        let conn_string = build_connection_string(
            &credentials.username,
            &credentials.password,
            host,
            port,
            database_name,
        );

        // Kết nối với timeout
        let pool = timeout(timeout_duration, async {
            sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_secs(10))
                .connect(&conn_string)
                .await
        })
        .await
        .map_err(|_| ExecutorError::Timeout(timeout_duration.as_secs()))?
        .map_err(|e| ExecutorError::ConnectionFailed(e.to_string()))?;

        // Thực thi SQL trong transaction với timeout
        let result = timeout(timeout_duration, async {
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| ExecutorError::QueryFailed(format!("failed to begin transaction: {e}")))?;

            let mut total_rows: i64 = 0;

            // Split multi-statement SQL by semicolons
            let statements: Vec<&str> = sql
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            for stmt in &statements {
                let result = sqlx::query(stmt)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| ExecutorError::QueryFailed(e.to_string()))?;
                total_rows += result.rows_affected() as i64;
            }

            tx.commit()
                .await
                .map_err(|e| ExecutorError::QueryFailed(format!("failed to commit: {e}")))?;

            Ok::<ExecutionResult, ExecutorError>(ExecutionResult {
                rows_affected: total_rows,
            })
        })
        .await
        .map_err(|_| ExecutorError::Timeout(timeout_duration.as_secs()))?;

        pool.close().await;

        result
    }

    async fn test_connection(
        &self,
        credentials: &DbCredentials,
        host: &str,
        port: i32,
        database_name: &str,
    ) -> Result<(), ExecutorError> {
        let conn_string = build_connection_string(
            &credentials.username,
            &credentials.password,
            host,
            port,
            database_name,
        );

        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&conn_string)
            .await
            .map_err(|e| ExecutorError::ConnectionFailed(e.to_string()))?;

        sqlx::query("SELECT 1")
            .execute(&pool)
            .await
            .map_err(|e| ExecutorError::QueryFailed(e.to_string()))?;

        pool.close().await;
        Ok(())
    }
}

/// Build MySQL connection string.
/// Enforce TLS (ssl-mode=REQUIRED) theo security rule.
/// Nếu env var DISABLE_TARGET_DB_TLS=true → bỏ ssl-mode (chỉ cho dev/testing).
fn build_connection_string(
    username: &str,
    password: &str,
    host: &str,
    port: i32,
    database_name: &str,
) -> String {
    let tls_disabled = std::env::var("DISABLE_TARGET_DB_TLS")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");

    let ssl_param = if tls_disabled {
        "" // không enforce TLS cho dev/testing
    } else {
        "?ssl-mode=REQUIRED"
    };

    format!(
        "mysql://{}:{}@{}:{}/{}{}",
        urlencoding::encode(username),
        urlencoding::encode(password),
        host,
        port,
        database_name,
        ssl_param
    )
}
