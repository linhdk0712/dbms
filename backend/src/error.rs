use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

/// Lỗi ứng dụng trung tâm.
/// - 4xx errors: trả message cụ thể cho client
/// - 5xx errors: log nội bộ, trả generic message — không expose details
#[derive(Error, Debug)]
pub enum AppError {
    // ── 400 Bad Request ────────────────────────────────────────────────
    #[error("4-eyes principle violated: maker cannot approve their own request")]
    FourEyesViolation,

    #[error("duplicate approval: you have already approved this request")]
    DuplicateApproval,

    #[error("invalid state transition: cannot move from '{from}' to '{to}'")]
    InvalidStateTransition { from: String, to: String },

    #[error("request is immutable after submission: cancel and create a new request to make changes")]
    ImmutableRequest,

    #[error("validation error: {0}")]
    ValidationError(String),

    // ── 401 Unauthorized ───────────────────────────────────────────────
    #[error("unauthorized: missing or invalid authentication token")]
    Unauthorized,

    // ── 403 Forbidden ──────────────────────────────────────────────────
    #[error("forbidden: insufficient permissions")]
    Forbidden,

    #[error("permission denied: user does not have access to database {db_id}")]
    DatabasePermissionDenied { db_id: Uuid },

    // ── 404 Not Found ──────────────────────────────────────────────────
    #[error("resource not found")]
    NotFound,

    // ── 500 Internal Server Error (không expose details ra client) ─────
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("jwt error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("encryption error: {0}")]
    Encryption(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            // 4xx — trả message có ý nghĩa
            AppError::FourEyesViolation
            | AppError::InvalidStateTransition { .. }
            | AppError::ImmutableRequest => (StatusCode::BAD_REQUEST, self.to_string()),

            AppError::DuplicateApproval => (StatusCode::CONFLICT, self.to_string()),

            AppError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),

            AppError::Unauthorized | AppError::Jwt(_) => {
                (StatusCode::UNAUTHORIZED, "unauthorized".to_string())
            }

            AppError::Forbidden | AppError::DatabasePermissionDenied { .. } => {
                (StatusCode::FORBIDDEN, self.to_string())
            }

            AppError::NotFound => (StatusCode::NOT_FOUND, "resource not found".to_string()),

            // 5xx — log nội bộ, generic message ra client
            AppError::Database(_)
            | AppError::Encryption(_)
            | AppError::Internal(_) => {
                tracing::error!(error = %self, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}

/// Shorthand cho Result trong handlers
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_four_eyes_returns_400() {
        let err = AppError::FourEyesViolation;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_unauthorized_returns_401() {
        let err = AppError::Unauthorized;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_not_found_returns_404() {
        let err = AppError::NotFound;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_forbidden_returns_403() {
        let err = AppError::Forbidden;
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_invalid_state_transition_returns_400() {
        let err = AppError::InvalidStateTransition {
            from: "DRAFT".into(),
            to: "EXECUTED".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
