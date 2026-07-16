use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Loại thông báo realtime gửi qua WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationType {
    NewRequest,
    RequestApproved,
    RequestRejected,
    RequestExecuted,
    RequestFailed,
    RequestCancelled,
    /// Partial approval — chưa đủ số cấp approve cần thiết.
    PartialApproval,
}

/// Thông báo được broadcast tới các client WebSocket đang kết nối.
/// Không chứa `sql_content` hay dữ liệu nhạy cảm — chỉ metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub request_id: Uuid,
    /// User đã tạo request — dùng cho WS role-based filtering
    pub created_by: Option<Uuid>,
    /// Tên database (an toàn để hiển thị)
    pub database_name: Option<String>,
    /// User liên quan đến hành động (maker hoặc checker)
    pub actor_id: Option<Uuid>,
    /// Rows affected khi EXECUTED
    pub rows_affected: Option<i64>,
    /// Lý do khi REJECTED
    pub detail: Option<String>,
}

impl Notification {
    /// Tạo notification tối giản chỉ với type và request_id.
    pub fn new(notification_type: NotificationType, request_id: Uuid) -> Self {
        Self {
            notification_type,
            request_id,
            created_by: None,
            database_name: None,
            actor_id: None,
            rows_affected: None,
            detail: None,
        }
    }
}
