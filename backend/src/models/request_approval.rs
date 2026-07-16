use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Bản ghi approval cho multi-level approval workflow.
/// Mỗi checker chỉ approve 1 lần per request (UNIQUE constraint in DB).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RequestApproval {
    pub id: Uuid,
    pub request_id: Uuid,
    pub approver_id: Uuid,
    pub level: i32,
    pub approved_at: DateTime<Utc>,
}
