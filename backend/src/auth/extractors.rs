use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};

use crate::auth::jwt::Claims;
use crate::error::AppError;
use crate::models::user::Role;
use crate::state::AppState;

/// Extractor lấy Claims từ request extensions.
/// Yêu cầu auth middleware đã chạy trước đó.
///
/// Dùng khi handler cần biết user nhưng không yêu cầu role cụ thể.
pub struct RequireAuth(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for RequireAuth {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or(AppError::Unauthorized)?;

        Ok(RequireAuth(claims))
    }
}

/// Extractor yêu cầu user phải có role Admin.
pub struct RequireAdmin(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or(AppError::Unauthorized)?;

        if !claims.has_role(&Role::Admin) {
            return Err(AppError::Forbidden);
        }

        Ok(RequireAdmin(claims))
    }
}

/// Extractor yêu cầu user phải có role Maker.
pub struct RequireMaker(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for RequireMaker {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or(AppError::Unauthorized)?;

        if !claims.has_role(&Role::Maker) {
            return Err(AppError::Forbidden);
        }

        Ok(RequireMaker(claims))
    }
}

/// Extractor yêu cầu user phải có role Checker.
pub struct RequireChecker(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for RequireChecker {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or(AppError::Unauthorized)?;

        if !claims.has_role(&Role::Checker) {
            return Err(AppError::Forbidden);
        }

        Ok(RequireChecker(claims))
    }
}

/// Extractor yêu cầu user phải có role Approver (hoặc Admin).
/// Approver role cho phép truy cập quản lý Approval Matrix và Approver Units.
pub struct RequireApprover(pub Claims);

#[async_trait]
impl FromRequestParts<AppState> for RequireApprover {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let claims = parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or(AppError::Unauthorized)?;

        if !claims.has_role(&Role::Approver) && !claims.has_role(&Role::Admin) {
            return Err(AppError::Forbidden);
        }

        Ok(RequireApprover(claims))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_claims(roles: Vec<Role>) -> Claims {
        Claims {
            sub: Uuid::new_v4().to_string(),
            exp: 9999999999,
            iat: 1000000000,
            iss: "secs-backend".to_string(),
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            roles,
        }
    }

    #[test]
    fn test_claims_extraction_from_extensions() {
        let claims = make_claims(vec![Role::Maker]);
        let mut extensions = axum::http::Extensions::new();
        extensions.insert(claims.clone());

        let result = extensions.get::<Claims>().cloned();
        assert!(result.is_some());
        assert_eq!(result.unwrap().email, "test@example.com");
    }

    #[test]
    fn test_missing_claims_returns_none() {
        let extensions = axum::http::Extensions::new();
        let result = extensions.get::<Claims>().cloned();
        assert!(result.is_none());
    }

    #[test]
    fn test_admin_role_check_pass() {
        let claims = make_claims(vec![Role::Admin]);
        assert!(claims.has_role(&Role::Admin));
    }

    #[test]
    fn test_admin_role_check_fail() {
        let claims = make_claims(vec![Role::Maker]);
        assert!(!claims.has_role(&Role::Admin));
    }

    #[test]
    fn test_maker_role_check_pass() {
        let claims = make_claims(vec![Role::Maker, Role::Checker]);
        assert!(claims.has_role(&Role::Maker));
    }

    #[test]
    fn test_checker_role_check_fail() {
        let claims = make_claims(vec![Role::Maker]);
        assert!(!claims.has_role(&Role::Checker));
    }

    #[test]
    fn test_multiple_roles() {
        let claims = make_claims(vec![Role::Admin, Role::Maker, Role::Checker]);
        assert!(claims.has_role(&Role::Admin));
        assert!(claims.has_role(&Role::Maker));
        assert!(claims.has_role(&Role::Checker));
    }
}
