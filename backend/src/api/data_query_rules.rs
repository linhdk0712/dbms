use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::extractors::RequireAdmin;
use crate::db;
use crate::error::AppError;
use crate::state::AppState;

// ── DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateRuleBody {
    pub database_id: Uuid,
    pub schema_name: Option<String>,
    pub table_name: Option<String>,
    pub levels: Vec<RuleLevelInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRuleBody {
    pub schema_name: Option<String>,
    pub table_name: Option<String>,
    pub levels: Vec<RuleLevelInput>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RuleLevelInput {
    pub level_order: i32,
    pub approver_unit: String,
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct RuleResponse {
    pub id: Uuid,
    pub database_id: Uuid,
    pub schema_name: Option<String>,
    pub table_name: Option<String>,
    pub is_active: bool,
    pub levels: Vec<RuleLevelOutput>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct RuleLevelOutput {
    pub level_order: i32,
    pub approver_unit: String,
    pub required: bool,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// `GET /api/v1/data-query-rules` — List all rules (Admin/DBA only).
pub async fn list_rules(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
) -> Result<Json<Vec<RuleResponse>>, AppError> {
    let rules = db::data_query_rules::list_rules(&state.db)
        .await
        .map_err(AppError::Database)?;

    let mut response = Vec::with_capacity(rules.len());
    for rule in &rules {
        let levels = db::data_query_rules::get_rule_levels(&state.db, rule.id)
            .await
            .map_err(AppError::Database)?;

        response.push(RuleResponse {
            id: rule.id,
            database_id: rule.database_id,
            schema_name: rule.schema_name.clone(),
            table_name: rule.table_name.clone(),
            is_active: rule.is_active,
            levels: levels
                .iter()
                .map(|l| RuleLevelOutput {
                    level_order: l.level_order,
                    approver_unit: l.approver_unit.to_string(),
                    required: l.required,
                })
                .collect(),
            created_at: rule.created_at,
            updated_at: rule.updated_at,
        });
    }

    Ok(Json(response))
}

/// `POST /api/v1/data-query-rules` — Create new rule (Admin/DBA only).
pub async fn create_rule(
    State(state): State<AppState>,
    RequireAdmin(claims): RequireAdmin,
    Json(body): Json<CreateRuleBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.levels.is_empty() {
        return Err(AppError::ValidationError(
            "At least one approval level is required".to_string(),
        ));
    }

    let levels: Vec<(i32, String, bool)> = body
        .levels
        .iter()
        .map(|l| (l.level_order, l.approver_unit.clone(), l.required))
        .collect();

    let rule = db::data_query_rules::create_rule(
        &state.db,
        body.database_id,
        body.schema_name.as_deref(),
        body.table_name.as_deref(),
        claims.user_id,
        &levels,
    )
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "id": rule.id,
        "message": "Approval rule created"
    })))
}

/// `PUT /api/v1/data-query-rules/:id` — Update rule (Admin/DBA only).
pub async fn update_rule(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRuleBody>,
) -> Result<Json<serde_json::Value>, AppError> {
    if body.levels.is_empty() {
        return Err(AppError::ValidationError(
            "At least one approval level is required".to_string(),
        ));
    }

    let levels: Vec<(i32, String, bool)> = body
        .levels
        .iter()
        .map(|l| (l.level_order, l.approver_unit.clone(), l.required))
        .collect();

    let rule = db::data_query_rules::update_rule(
        &state.db,
        id,
        body.schema_name.as_deref(),
        body.table_name.as_deref(),
        &levels,
    )
    .await
    .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "id": rule.id,
        "message": "Approval rule updated"
    })))
}

/// `DELETE /api/v1/data-query-rules/:id` — Delete rule (Admin/DBA only).
pub async fn delete_rule(
    State(state): State<AppState>,
    RequireAdmin(_claims): RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    db::data_query_rules::delete_rule(&state.db, id)
        .await
        .map_err(AppError::Database)?;

    Ok(Json(serde_json::json!({
        "message": "Approval rule deleted"
    })))
}
