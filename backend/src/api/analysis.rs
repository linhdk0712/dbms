//! API handler cho AI SQL Analysis.
//!
//! Endpoint: `POST /api/v1/requests/:id/analyze`
//!
//! Trigger: Maker (sau submit) hoặc Checker (khi review).
//! Access control:
//! - Maker: chỉ analyze request của mình
//! - Checker: chỉ analyze request đang PENDING

use axum::{extract::{Path, State}, Json};
use std::time::Duration;
use uuid::Uuid;

use crate::ai::{
    self, AiAnalysisResponse, AiError,
    llm_client::LlmClient,
    prompt_builder,
    schema_introspector::{self, SchemaContext},
};
use crate::auth::extractors::RequireAuth;
use crate::db;
use crate::error::AppError;
use crate::executor::encryption::decrypt_credentials;
use crate::models::request::RequestStatus;
use crate::state::AppState;

/// `POST /api/v1/requests/:id/analyze` — AI phân tích SQL.
///
/// Flow:
/// 1. Lấy request + kiểm tra quyền
/// 2. Lấy DB connection → decrypt credentials
/// 3. Introspect schema (tables referenced in SQL)
/// 4. Build prompt → gọi LLM
/// 5. Parse response → trả AiAnalysisResponse
pub async fn analyze_request(
    State(state): State<AppState>,
    RequireAuth(claims): RequireAuth,
    Path(id): Path<Uuid>,
) -> Result<Json<AiAnalysisResponse>, AppError> {
    // 0. Check AI enabled
    if !state.ai_config.enabled {
        return Err(AppError::AiUnavailable("AI analysis is disabled".to_string()));
    }

    // 0.1. Rate limit check per user
    if !state.ai_rate_limiter.check_and_record(claims.user_id).await {
        return Err(AppError::AiRateLimited);
    }

    // 1. Lấy request
    let request = db::requests::get_by_id(&state.db, id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // Access control: Maker (owner) hoặc Checker (request đang PENDING)
    let is_owner = request.created_by == claims.user_id;
    let is_pending = request.status == RequestStatus::Pending;

    if !is_owner && !is_pending {
        return Err(AppError::Forbidden);
    }

    // Request phải ở trạng thái PENDING hoặc DRAFT (Maker xem trước khi submit)
    if request.status != RequestStatus::Pending && request.status != RequestStatus::Draft {
        return Err(AppError::ValidationError(
            "AI analysis only available for DRAFT or PENDING requests".to_string(),
        ));
    }

    // 1.5. Check cache — trả ngay nếu có kết quả cached
    if let Some(cached) = state.ai_cache.get(id, &request.sql_content).await {
        tracing::debug!(request_id = %id, "AI analysis served from cache");
        return Ok(Json(cached));
    }

    // 2. Lấy database connection info
    let db_conn = db::databases::get_by_id(&state.db, request.database_id)
        .await
        .map_err(AppError::Database)?
        .ok_or(AppError::NotFound)?;

    // 3. Decrypt credentials cho schema introspection
    let key_bytes = state
        .config
        .encryption_key_bytes()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("encryption key error: {e}")))?;

    let credentials = decrypt_credentials(&db_conn.encrypted_credentials, &key_bytes)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("credential decryption failed: {e}")))?;

    // 4. Extract table names từ SQL
    let table_names = schema_introspector::extract_table_names(&request.sql_content);

    // 5. Introspect schema (best-effort — nếu lỗi thì phân tích không có schema)
    let introspect_timeout = Duration::from_secs(15);
    let schema = introspect_schema(
        &db_conn.db_type.to_string(),
        &credentials,
        &db_conn.host,
        db_conn.port,
        &db_conn.database_name,
        db_conn.schema_name.as_deref(),
        &table_names,
        introspect_timeout,
    )
    .await
    .unwrap_or_else(|e| {
        tracing::warn!(error = %e, "schema introspection failed — proceeding without schema context");
        SchemaContext { tables: vec![], indexes: vec![] }
    });

    // 6. Build prompt
    let system_prompt = prompt_builder::build_system_prompt();
    let user_prompt = prompt_builder::build_user_prompt(
        &request.sql_content,
        &schema,
        &db_conn.db_type,
    );

    // 7. Call LLM
    let llm_client = LlmClient::new(state.ai_config.clone());
    let raw_response = llm_client
        .analyze(&system_prompt, &user_prompt)
        .await
        .map_err(|e| match e {
            AiError::Disabled => AppError::AiUnavailable("AI analysis is disabled".to_string()),
            AiError::Timeout(secs) => AppError::AiUnavailable(format!("AI request timed out after {secs}s")),
            AiError::RateLimited(secs) => AppError::AiUnavailable(format!("AI rate limited, retry after {secs}s")),
            AiError::ServiceUnavailable(msg) => AppError::AiUnavailable(msg),
            AiError::ParseError(msg) => AppError::AiUnavailable(format!("AI response error: {msg}")),
            AiError::SchemaError(msg) => AppError::AiUnavailable(format!("Schema error: {msg}")),
        })?;

    // 8. Parse AI response JSON
    let analysis: AiAnalysisResponse = serde_json::from_str(&raw_response)
        .map_err(|e| {
            tracing::error!(error = %e, "failed to parse AI response JSON");
            AppError::AiUnavailable(format!("AI returned invalid response format: {e}"))
        })?;

    // 9. Validate risk_score bounds
    let analysis = ai::AiAnalysisResponse {
        risk_score: analysis.risk_score.min(10).max(1),
        ..analysis
    };

    // 10. Cache result
    state.ai_cache.set(id, &request.sql_content, analysis.clone()).await;

    tracing::info!(
        request_id = %id,
        user_id = %claims.user_id,
        risk_score = analysis.risk_score,
        performance_issues = analysis.performance.len(),
        security_issues = analysis.security.len(),
        "AI analysis completed"
    );

    Ok(Json(analysis))
}

/// Dispatch schema introspection theo db_type.
async fn introspect_schema(
    db_type: &str,
    credentials: &crate::executor::DbCredentials,
    host: &str,
    port: i32,
    database_name: &str,
    schema_name: Option<&str>,
    table_names: &[String],
    timeout: Duration,
) -> Result<SchemaContext, AiError> {
    match db_type {
        "postgresql" => {
            schema_introspector::introspect_postgres(
                credentials, host, port, database_name, schema_name, table_names, timeout,
            ).await
        }
        "mysql" => {
            schema_introspector::introspect_mysql(
                credentials, host, port, database_name, table_names, timeout,
            ).await
        }
        "oracle" => {
            schema_introspector::introspect_oracle(
                credentials, host, port, database_name, schema_name, table_names, timeout,
            ).await
        }
        "mongodb" => {
            schema_introspector::introspect_mongodb(table_names).await
        }
        _ => Ok(SchemaContext { tables: vec![], indexes: vec![] }),
    }
}
