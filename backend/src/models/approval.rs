use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Các hành động được ghi vào audit log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditAction {
    Created,
    Submitted,
    Approved,
    Rejected,
    Executed,
    Failed,
    Cancelled,
    /// Trạng thái EXECUTING trước khi SQL thực sự chạy (Req 11)
    Executing,
    /// Ghi nhận thay đổi role của user (Req 12)
    RoleChanged,
    /// Ghi nhận thay đổi trạng thái active/inactive của user (Req 12)
    UserStatusChanged,
    /// Ghi nhận credential rotation cho database (Req 2)
    CredentialRotated,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AuditAction::Created => "CREATED",
            AuditAction::Submitted => "SUBMITTED",
            AuditAction::Approved => "APPROVED",
            AuditAction::Rejected => "REJECTED",
            AuditAction::Executed => "EXECUTED",
            AuditAction::Failed => "FAILED",
            AuditAction::Cancelled => "CANCELLED",
            AuditAction::Executing => "EXECUTING",
            AuditAction::RoleChanged => "ROLE_CHANGED",
            AuditAction::UserStatusChanged => "USER_STATUS_CHANGED",
            AuditAction::CredentialRotated => "CREDENTIAL_ROTATED",
        };
        write!(f, "{s}")
    }
}

/// Bản ghi audit log — append-only, không bao giờ UPDATE/DELETE.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    /// Nullable — admin actions (role change, credential rotation) không gắn với request
    pub request_id: Option<Uuid>,
    pub action: AuditAction,
    pub actor_id: Uuid,
    /// Lý do reject, error message khi FAILED, v.v.
    pub detail: Option<String>,
    /// Số rows affected khi EXECUTED
    pub execution_result: Option<i64>,
    pub ip_address: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_action_display() {
        assert_eq!(AuditAction::Created.to_string(), "CREATED");
        assert_eq!(AuditAction::Executed.to_string(), "EXECUTED");
        assert_eq!(AuditAction::Failed.to_string(), "FAILED");
        assert_eq!(AuditAction::Executing.to_string(), "EXECUTING");
        assert_eq!(AuditAction::RoleChanged.to_string(), "ROLE_CHANGED");
        assert_eq!(AuditAction::UserStatusChanged.to_string(), "USER_STATUS_CHANGED");
        assert_eq!(AuditAction::CredentialRotated.to_string(), "CREDENTIAL_ROTATED");
    }

    // PBT: AuditAction serialization round-trip
    #[test]
    fn test_audit_action_serde_roundtrip() {
        let actions = [
            AuditAction::Created,
            AuditAction::Submitted,
            AuditAction::Approved,
            AuditAction::Rejected,
            AuditAction::Executed,
            AuditAction::Failed,
            AuditAction::Cancelled,
            AuditAction::Executing,
            AuditAction::RoleChanged,
            AuditAction::UserStatusChanged,
            AuditAction::CredentialRotated,
        ];
        for action in &actions {
            let json = serde_json::to_string(action).unwrap();
            let back: AuditAction = serde_json::from_str(&json).unwrap();
            assert_eq!(action, &back, "round-trip failed for {action}");
        }
    }
}
