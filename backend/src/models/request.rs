use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trạng thái của một SqlRequest.
/// State transitions được kiểm soát bởi `can_transition_to()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequestStatus {
    Draft,
    Pending,
    Approved,
    Rejected,
    Executed,
    Failed,
    Cancelled,
}

impl RequestStatus {
    /// Kiểm tra transition hợp lệ theo state machine.
    /// `EXECUTED`, `FAILED`, `REJECTED`, `CANCELLED` là terminal states.
    pub fn can_transition_to(&self, next: &RequestStatus) -> bool {
        use RequestStatus::*;
        matches!(
            (self, next),
            (Draft, Pending)
                | (Pending, Approved)
                | (Pending, Rejected)
                | (Pending, Cancelled)
                | (Approved, Executed)
                | (Approved, Failed)
        )
    }

    /// Kiểm tra xem đây có phải terminal state không.
    pub fn is_terminal(&self) -> bool {
        use RequestStatus::*;
        matches!(self, Executed | Failed | Rejected | Cancelled)
    }
}

impl std::fmt::Display for RequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RequestStatus::Draft => "DRAFT",
            RequestStatus::Pending => "PENDING",
            RequestStatus::Approved => "APPROVED",
            RequestStatus::Rejected => "REJECTED",
            RequestStatus::Executed => "EXECUTED",
            RequestStatus::Failed => "FAILED",
            RequestStatus::Cancelled => "CANCELLED",
        };
        write!(f, "{s}")
    }
}

/// SQL Request — đơn vị trung tâm của hệ thống.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SqlRequest {
    pub id: Uuid,
    pub description: String,
    /// Không trả về trong list API — chỉ trong detail response
    #[serde(skip_serializing)]
    pub sql_content: String,
    pub status: RequestStatus,
    pub database_id: Uuid,
    pub created_by: Uuid,
    pub approved_by: Option<Uuid>,
    pub rejection_reason: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub rows_affected: Option<i64>,
    /// Error message khi FAILED — không log ra console nếu sensitive
    pub execution_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    /// Số cấp approval cần thiết — snapshot từ database_connections khi submit (Req 10.8).
    /// Default 1 cho backward compatibility.
    pub approval_levels_required: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        use RequestStatus::*;
        assert!(Draft.can_transition_to(&Pending));
        assert!(Pending.can_transition_to(&Approved));
        assert!(Pending.can_transition_to(&Rejected));
        assert!(Pending.can_transition_to(&Cancelled));
        assert!(Approved.can_transition_to(&Executed));
        assert!(Approved.can_transition_to(&Failed));
    }

    #[test]
    fn test_invalid_transitions() {
        use RequestStatus::*;
        // Không thể nhảy cóc
        assert!(!Draft.can_transition_to(&Approved));
        assert!(!Draft.can_transition_to(&Executed));
        // Terminal states không đi đâu
        assert!(!Executed.can_transition_to(&Draft));
        assert!(!Failed.can_transition_to(&Pending));
        assert!(!Rejected.can_transition_to(&Approved));
        assert!(!Cancelled.can_transition_to(&Pending));
        // Không thể self-transition
        assert!(!Pending.can_transition_to(&Pending));
        assert!(!Draft.can_transition_to(&Draft));
    }

    #[test]
    fn test_terminal_states() {
        use RequestStatus::*;
        assert!(Executed.is_terminal());
        assert!(Failed.is_terminal());
        assert!(Rejected.is_terminal());
        assert!(Cancelled.is_terminal());
        assert!(!Draft.is_terminal());
        assert!(!Pending.is_terminal());
        assert!(!Approved.is_terminal());
    }

    #[test]
    fn test_no_cycle_from_any_status() {
        // Từ bất kỳ terminal state nào, không thể quay lại Draft hoặc Pending
        use RequestStatus::*;
        let terminal = [Executed, Failed, Rejected, Cancelled];
        let all = [Draft, Pending, Approved, Rejected, Executed, Failed, Cancelled];
        for t in &terminal {
            for s in &all {
                assert!(!t.can_transition_to(s),
                    "{t} should not transition to {s}");
            }
        }
    }

    // PBT: terminal states không có outgoing transition nào
    proptest::proptest! {
        #[test]
        fn prop_terminal_states_have_no_outgoing_transitions(
            terminal in proptest::sample::select(vec![
                RequestStatus::Executed,
                RequestStatus::Failed,
                RequestStatus::Rejected,
                RequestStatus::Cancelled,
            ]),
            next in proptest::sample::select(vec![
                RequestStatus::Draft,
                RequestStatus::Pending,
                RequestStatus::Approved,
                RequestStatus::Rejected,
                RequestStatus::Executed,
                RequestStatus::Failed,
                RequestStatus::Cancelled,
            ]),
        ) {
            proptest::prop_assert!(!terminal.can_transition_to(&next));
        }

        // PBT: RequestStatus serialization round-trip
        #[test]
        fn prop_request_status_serde_roundtrip(
            status in proptest::sample::select(vec![
                RequestStatus::Draft,
                RequestStatus::Pending,
                RequestStatus::Approved,
                RequestStatus::Rejected,
                RequestStatus::Executed,
                RequestStatus::Failed,
                RequestStatus::Cancelled,
            ]),
        ) {
            let json = serde_json::to_string(&status).unwrap();
            let back: RequestStatus = serde_json::from_str(&json).unwrap();
            proptest::prop_assert_eq!(status, back);
        }
    }
}
