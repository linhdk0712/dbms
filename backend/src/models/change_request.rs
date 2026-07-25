use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Change Request — metadata cached từ ManageEngine ServiceDesk Plus.
/// Một CR có thể gắn với nhiều SQL requests (1-N).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ChangeRequest {
    pub id: Uuid,
    /// Change ID extracted từ ManageEngine URL (e.g. "12345")
    pub cr_number: String,
    /// Full URL user nhập
    pub cr_url: String,
    /// Title lấy từ ManageEngine API
    pub title: Option<String>,
    /// Status từ ManageEngine (e.g. "Approved", "Requested")
    pub status: Option<String>,
    /// Maintenance window start — block execution nếu ngoài window
    pub scheduled_start_time: Option<DateTime<Utc>>,
    /// Maintenance window end
    pub scheduled_end_time: Option<DateTime<Utc>>,
    /// Lần cuối sync từ ManageEngine
    pub synced_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}
