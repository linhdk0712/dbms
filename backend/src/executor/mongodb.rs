//! MongoDB Executor — sử dụng `mongodb` crate (async native).
//!
//! # Architecture
//! - MongoDB không dùng SQL → input là JSON command(s)
//! - Mỗi command là một JSON object, multi-command: JSON array
//! - Thực thi qua `Database::run_command()` — hỗ trợ mọi database command
//! - Transaction: multi-command được wrap trong session transaction (all-or-nothing)
//! - Timeout: dùng `tokio::time::timeout` bọc toàn bộ execution
//!
//! # Input Format
//! - Single command: `{"insert": "collection", "documents": [...]}`
//! - Multi-command (array): `[{"cmd1": ...}, {"cmd2": ...}]`
//!
//! # Rows Affected
//! - Đếm từ response fields: `n`, `nModified`, `nInserted`, `nRemoved`

use std::time::Duration;
use tokio::time::timeout;

use super::{DatabaseExecutor, DbCredentials, ExecutionResult, ExecutorError};

/// MongoDB executor — kết nối đến MongoDB qua official driver.
pub struct MongoExecutor;

impl DatabaseExecutor for MongoExecutor {
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
        let uri = build_connection_uri(&credentials.username, &credentials.password, host, port);

        // Parse commands từ input (JSON single object hoặc array)
        let commands = parse_commands(sql)?;

        let result = timeout(timeout_duration, async {
            let client = create_client(&uri).await?;
            let db = client.database(database_name);

            if commands.len() == 1 {
                // Single command — không cần transaction
                let result = execute_single_command(&db, &commands[0]).await?;
                Ok(result)
            } else {
                // Multi-command — wrap trong transaction
                execute_in_transaction(&client, &db, &commands).await
            }
        })
        .await
        .map_err(|_| ExecutorError::Timeout(timeout_duration.as_secs()))?;

        result
    }

    async fn test_connection(
        &self,
        credentials: &DbCredentials,
        host: &str,
        port: i32,
        database_name: &str,
    ) -> Result<(), ExecutorError> {
        let uri = build_connection_uri(&credentials.username, &credentials.password, host, port);

        let client = create_client(&uri).await?;
        let db = client.database(database_name);

        // Test bằng ping command
        let ping_cmd = mongodb::bson::doc! { "ping": 1 };
        db.run_command(ping_cmd)
            .await
            .map_err(|e| ExecutorError::QueryFailed(format!("ping failed: {e}")))?;

        Ok(())
    }
}

/// Build MongoDB connection URI.
/// Format: mongodb://user:pass@host:port/?tls=true&authSource=admin
/// Nếu env var DISABLE_TARGET_DB_TLS=true → bỏ tls (chỉ cho dev/testing).
fn build_connection_uri(username: &str, password: &str, host: &str, port: i32) -> String {
    let tls_disabled = std::env::var("DISABLE_TARGET_DB_TLS")
        .unwrap_or_default()
        .eq_ignore_ascii_case("true");

    let tls_param = if tls_disabled {
        "tls=false"
    } else {
        "tls=true"
    };

    format!(
        "mongodb://{}:{}@{}:{}/?{}&authSource=admin",
        urlencoding::encode(username),
        urlencoding::encode(password),
        host,
        port,
        tls_param
    )
}

/// Tạo MongoDB client với connection timeout.
async fn create_client(uri: &str) -> Result<mongodb::Client, ExecutorError> {
    let mut client_options = mongodb::options::ClientOptions::parse(uri)
        .await
        .map_err(|e| ExecutorError::ConnectionFailed(format!("invalid URI: {e}")))?;

    // Connection timeout 10s, giống pattern các executor khác
    client_options.connect_timeout = Some(Duration::from_secs(10));
    client_options.server_selection_timeout = Some(Duration::from_secs(10));

    mongodb::Client::with_options(client_options)
        .map_err(|e| ExecutorError::ConnectionFailed(e.to_string()))
}

/// Parse input thành danh sách commands.
/// Hỗ trợ:
/// - Single object: `{"insert": "coll", ...}` → vec![doc]
/// - Array: `[{"cmd1": ...}, {"cmd2": ...}]` → vec![doc1, doc2]
fn parse_commands(input: &str) -> Result<Vec<mongodb::bson::Document>, ExecutorError> {
    let trimmed = input.trim();

    if trimmed.starts_with('[') {
        // Array of commands
        let values: Vec<serde_json::Value> = serde_json::from_str(trimmed)
            .map_err(|e| ExecutorError::QueryFailed(format!("invalid JSON array: {e}")))?;

        let mut docs = Vec::with_capacity(values.len());
        for value in values {
            let doc = json_value_to_bson_doc(value)?;
            docs.push(doc);
        }

        if docs.is_empty() {
            return Err(ExecutorError::QueryFailed(
                "empty command array".to_string(),
            ));
        }

        Ok(docs)
    } else if trimmed.starts_with('{') {
        // Single command
        let value: serde_json::Value = serde_json::from_str(trimmed)
            .map_err(|e| ExecutorError::QueryFailed(format!("invalid JSON object: {e}")))?;
        let doc = json_value_to_bson_doc(value)?;
        Ok(vec![doc])
    } else {
        Err(ExecutorError::QueryFailed(
            "input must be a JSON object or array of objects".to_string(),
        ))
    }
}

/// Convert serde_json::Value thành bson::Document.
fn json_value_to_bson_doc(
    value: serde_json::Value,
) -> Result<mongodb::bson::Document, ExecutorError> {
    let bson_value = mongodb::bson::to_bson(&value)
        .map_err(|e| ExecutorError::QueryFailed(format!("JSON to BSON conversion failed: {e}")))?;

    match bson_value {
        mongodb::bson::Bson::Document(doc) => Ok(doc),
        _ => Err(ExecutorError::QueryFailed(
            "command must be a JSON object".to_string(),
        )),
    }
}

/// Thực thi single command, trả về rows affected.
async fn execute_single_command(
    db: &mongodb::Database,
    command: &mongodb::bson::Document,
) -> Result<ExecutionResult, ExecutorError> {
    let result = db
        .run_command(command.clone())
        .await
        .map_err(|e| ExecutorError::QueryFailed(e.to_string()))?;

    let rows_affected = extract_rows_affected(&result);

    Ok(ExecutionResult { rows_affected })
}

/// Thực thi multi-command trong transaction (all-or-nothing).
/// Yêu cầu MongoDB replica set hoặc sharded cluster.
async fn execute_in_transaction(
    client: &mongodb::Client,
    db: &mongodb::Database,
    commands: &[mongodb::bson::Document],
) -> Result<ExecutionResult, ExecutorError> {
    let mut session = client
        .start_session()
        .await
        .map_err(|e| ExecutorError::QueryFailed(format!("failed to start session: {e}")))?;

    session
        .start_transaction()
        .await
        .map_err(|e| ExecutorError::QueryFailed(format!("failed to start transaction: {e}")))?;

    let mut total_rows: i64 = 0;

    for command in commands {
        let result = db
            .run_command(command.clone())
            .session(&mut session)
            .await;

        match result {
            Ok(doc) => {
                // Check if command reported an error via "ok" field
                let ok = doc.get_f64("ok").unwrap_or(1.0);
                if ok < 1.0 {
                    let errmsg = doc
                        .get_str("errmsg")
                        .unwrap_or("unknown command error");
                    // Abort transaction
                    let _ = session.abort_transaction().await;
                    return Err(ExecutorError::QueryFailed(errmsg.to_string()));
                }
                total_rows += extract_rows_affected(&doc);
            }
            Err(e) => {
                // Abort transaction on error
                let _ = session.abort_transaction().await;
                return Err(ExecutorError::QueryFailed(e.to_string()));
            }
        }
    }

    // Commit transaction
    session
        .commit_transaction()
        .await
        .map_err(|e| ExecutorError::QueryFailed(format!("commit failed: {e}")))?;

    Ok(ExecutionResult {
        rows_affected: total_rows,
    })
}

/// Extract rows affected từ MongoDB command result document.
/// Kiểm tra các fields phổ biến: `n`, `nModified`, `nInserted`, `nRemoved`.
fn extract_rows_affected(doc: &mongodb::bson::Document) -> i64 {
    // Ưu tiên `n` (common cho insert, delete, update commands)
    if let Ok(n) = doc.get_i64("n") {
        return n;
    }
    if let Ok(n) = doc.get_i32("n") {
        return n as i64;
    }

    // Fallback: nModified (update result)
    if let Ok(n) = doc.get_i64("nModified") {
        return n;
    }
    if let Ok(n) = doc.get_i32("nModified") {
        return n as i64;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_connection_uri() {
        // Đảm bảo TLS enabled (default)
        std::env::remove_var("DISABLE_TARGET_DB_TLS");
        let uri = build_connection_uri("admin", "s3cr3t!", "mongo.company.com", 27017);
        assert_eq!(
            uri,
            "mongodb://admin:s3cr3t%21@mongo.company.com:27017/?tls=true&authSource=admin"
        );
    }

    #[test]
    fn test_build_connection_uri_tls_disabled() {
        std::env::set_var("DISABLE_TARGET_DB_TLS", "true");
        let uri = build_connection_uri("admin", "pass", "localhost", 27017);
        assert!(uri.contains("tls=false"));
        std::env::remove_var("DISABLE_TARGET_DB_TLS");
    }

    #[test]
    fn test_build_connection_uri_special_chars() {
        std::env::remove_var("DISABLE_TARGET_DB_TLS");
        let uri = build_connection_uri("user@domain", "p@ss/word", "10.0.1.5", 27018);
        assert!(uri.contains("user%40domain"));
        assert!(uri.contains("p%40ss%2Fword"));
        assert!(uri.contains("10.0.1.5:27018"));
    }

    #[test]
    fn test_parse_commands_single_object() {
        let input = r#"{"insert": "users", "documents": [{"name": "test"}]}"#;
        let commands = parse_commands(input).unwrap();
        assert_eq!(commands.len(), 1);
        assert!(commands[0].contains_key("insert"));
    }

    #[test]
    fn test_parse_commands_array() {
        let input = r#"[{"insert": "users", "documents": [{"name": "a"}]}, {"delete": "users", "deletes": [{"q": {"name": "b"}, "limit": 1}]}]"#;
        let commands = parse_commands(input).unwrap();
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_parse_commands_invalid_json() {
        let result = parse_commands("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_commands_empty_array() {
        let result = parse_commands("[]");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_rows_affected_with_n() {
        let doc = mongodb::bson::doc! { "ok": 1.0, "n": 5 };
        assert_eq!(extract_rows_affected(&doc), 5);
    }

    #[test]
    fn test_extract_rows_affected_with_n_modified() {
        let doc = mongodb::bson::doc! { "ok": 1.0, "nModified": 3 };
        assert_eq!(extract_rows_affected(&doc), 3);
    }

    #[test]
    fn test_extract_rows_affected_no_field() {
        let doc = mongodb::bson::doc! { "ok": 1.0 };
        assert_eq!(extract_rows_affected(&doc), 0);
    }
}
