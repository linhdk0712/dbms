use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::state::AppState;

/// GET /health — kiểm tra server và kết nối database.
/// Trả 200 nếu DB pool còn sống, 503 nếu không.
/// Không yêu cầu authentication.
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "db": "connected" })),
        ),
        Err(e) => {
            tracing::error!(error = %e, "health check failed: database unreachable");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "error", "db": "disconnected" })),
            )
        }
    }
}
