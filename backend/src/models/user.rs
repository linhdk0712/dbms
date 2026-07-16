use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Role hệ thống của user.
/// Một user có thể có nhiều role (lưu dưới dạng Vec<Role> trong JWT claims).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Admin,
    Maker,
    Checker,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "ADMIN"),
            Role::Maker => write!(f, "MAKER"),
            Role::Checker => write!(f, "CHECKER"),
        }
    }
}

/// User trong hệ thống — được tạo/cập nhật khi OIDC login.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    /// Subject từ OIDC provider (unique identifier)
    pub oidc_subject: String,
    pub email: String,
    pub display_name: String,
    /// Soft delete — không xóa để giữ audit trail
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Quyền của user trên một DatabaseConnection cụ thể.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserDbPermission {
    pub id: Uuid,
    pub user_id: Uuid,
    pub database_id: Uuid,
    pub granted_by: Uuid,
    pub granted_at: DateTime<Utc>,
}
