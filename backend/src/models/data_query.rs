use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Enums ──────────────────────────────────────────────────────────────

/// Trạng thái của Data Query Request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DataQueryStatus {
    Draft,
    PendingApproval,
    Approved,
    Rejected,
    Expired,
    Revoked,
}

impl std::fmt::Display for DataQueryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Draft => "DRAFT",
            Self::PendingApproval => "PENDING_APPROVAL",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
            Self::Expired => "EXPIRED",
            Self::Revoked => "REVOKED",
        };
        write!(f, "{s}")
    }
}

/// Chế độ query: Basic (UI builder) hoặc Advanced (free-form SQL).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueryMode {
    Basic,
    Advanced,
}

impl std::fmt::Display for QueryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Basic => "BASIC",
            Self::Advanced => "ADVANCED",
        };
        write!(f, "{s}")
    }
}

/// Đơn vị phê duyệt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApproverUnit {
    Ito,
    Its,
    Compliance,
}

impl std::fmt::Display for ApproverUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Ito => "ITO",
            Self::Its => "ITS",
            Self::Compliance => "COMPLIANCE",
        };
        write!(f, "{s}")
    }
}

/// Trạng thái approval tracking cho một level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovalTrackingStatus {
    Pending,
    Approved,
    Rejected,
    Skipped,
}

impl std::fmt::Display for ApprovalTrackingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Pending => "PENDING",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
            Self::Skipped => "SKIPPED",
        };
        write!(f, "{s}")
    }
}

// ── Structs ────────────────────────────────────────────────────────────

/// Data Query Request — trung tâm của flow.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataQueryRequest {
    pub id: Uuid,
    pub title: String,
    pub database_id: Uuid,
    pub schema_name: String,
    pub sql_query: String,
    pub tables_accessed: Vec<String>,
    pub columns_accessed: Vec<String>,
    pub query_mode: QueryMode,
    pub ticket_id: String,
    pub status: DataQueryStatus,
    pub created_by: Uuid,
    pub current_approval_level: i32,
    pub total_approval_levels: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoke_reason: Option<String>,
}

/// User được assign để run query.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataQueryAssignedUser {
    pub id: Uuid,
    pub query_request_id: Uuid,
    pub user_id: Uuid,
    pub assigned_at: DateTime<Utc>,
}

/// Approval rule (DBA config).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataQueryApprovalRule {
    pub id: Uuid,
    pub database_id: Uuid,
    pub schema_name: Option<String>,
    pub table_name: Option<String>,
    pub created_by: Uuid,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Một level trong approval chain.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataQueryApprovalLevel {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub level_order: i32,
    pub approver_unit: ApproverUnit,
    pub required: bool,
}

/// Track approval cho từng request × level.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataQueryApprovalTracking {
    pub id: Uuid,
    pub query_request_id: Uuid,
    pub level_order: i32,
    pub approver_unit: ApproverUnit,
    pub status: ApprovalTrackingStatus,
    pub approved_by: Option<Uuid>,
    pub comment: Option<String>,
    pub acted_at: Option<DateTime<Utc>>,
}

/// Audit record cho mỗi lần execute.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataQueryExecution {
    pub id: Uuid,
    pub query_request_id: Uuid,
    pub executed_by: Uuid,
    pub rows_returned: i64,
    pub execution_time_ms: i64,
    pub client_ip: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub executed_at: DateTime<Utc>,
}
