use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Loại database được hỗ trợ.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum DbType {
    PostgreSQL,
    MySQL,
    Oracle,
    MongoDB,
}

impl std::fmt::Display for DbType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbType::PostgreSQL => write!(f, "postgresql"),
            DbType::MySQL => write!(f, "mysql"),
            DbType::Oracle => write!(f, "oracle"),
            DbType::MongoDB => write!(f, "mongodb"),
        }
    }
}

/// Cấu hình kết nối đến một database đích.
/// `encrypted_credentials` chứa JSON được mã hóa AES-256-GCM.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DatabaseConnection {
    pub id: Uuid,
    /// Tên hiển thị duy nhất, VD: "ORACLE_PROD_ACCOUNTS"
    pub name: String,
    pub db_type: DbType,
    pub host: String,
    pub port: i32,
    pub database_name: String,
    pub schema_name: Option<String>,
    /// AES-256-GCM encrypted JSON — không bao giờ trả về trong API response
    #[serde(skip_serializing)]
    pub encrypted_credentials: String,
    /// Timeout tính bằng giây (default: 30, max: 300)
    pub timeout_secs: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Số cấp approval cần thiết cho request trên DB này (1-5, default 1).
    pub approval_levels_required: i32,
}

/// Response-safe version — không chứa credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConnectionResponse {
    pub id: Uuid,
    pub name: String,
    pub db_type: DbType,
    pub host: String,
    pub port: i32,
    pub database_name: String,
    pub schema_name: Option<String>,
    pub timeout_secs: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<DatabaseConnection> for DatabaseConnectionResponse {
    fn from(db: DatabaseConnection) -> Self {
        Self {
            id: db.id,
            name: db.name,
            db_type: db.db_type,
            host: db.host,
            port: db.port,
            database_name: db.database_name,
            schema_name: db.schema_name,
            timeout_secs: db.timeout_secs,
            is_active: db.is_active,
            created_at: db.created_at,
        }
    }
}

/// Response DTO cho Maker — chỉ chứa metadata, không chứa credentials/timeout/timestamps.
/// Dùng cho endpoint GET /api/v1/databases/my-permissions
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MakerDatabaseResponse {
    pub id: Uuid,
    pub name: String,
    pub db_type: DbType,
    pub host: String,
    pub port: i32,
    pub database_name: String,
    pub schema_name: Option<String>,
}

/// Body cho credential update: PATCH /api/v1/databases/:id/credentials
#[derive(Debug, Deserialize, validator::Validate)]
pub struct UpdateCredentialsBody {
    #[validate(length(min = 1, max = 128, message = "Username must be between 1 and 128 characters"))]
    pub username: String,
    #[validate(length(min = 1, max = 256, message = "Password must be between 1 and 256 characters"))]
    pub password: String,
}
