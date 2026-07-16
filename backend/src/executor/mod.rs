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

/// Test kết nối dựa trên db_type.
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
