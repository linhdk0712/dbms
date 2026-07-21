//! AI SQL Analysis module — phân tích, tối ưu và đưa ra recommendations cho SQL.
//!
//! Module này cung cấp:
//! - Gọi LLM API (OpenAI / AWS Bedrock) để phân tích SQL
//! - Introspect schema từ database đích (tables, indexes, constraints)
//! - Xây dựng prompt tối ưu cho LLM
//! - Response types cho AI analysis results
//! - Rate limiting per user

pub mod llm_client;
pub mod prompt_builder;
pub mod schema_introspector;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

// ── Response Models ────────────────────────────────────────────────────

/// Kết quả phân tích AI cho một câu SQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysisResponse {
    /// Recommendations tối ưu performance
    pub performance: Vec<Recommendation>,
    /// Security issues (SQL injection patterns, privilege concerns)
    pub security: Vec<SecurityIssue>,
    /// Best practices violations
    pub best_practices: Vec<Recommendation>,
    /// Ước lượng cost/impact
    pub cost_estimation: CostEstimation,
    /// Overall risk score (1-10, 10 = highest risk)
    pub risk_score: u8,
    /// SQL được đề xuất (rewritten/optimized version)
    pub suggested_sql: Option<String>,
    /// Summary ngắn gọn
    pub summary: String,
}

/// Một recommendation cụ thể (performance hoặc best practice).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// "high", "medium", "low"
    pub severity: String,
    /// Category: "missing_index", "full_scan", "cartesian_join", "select_star", etc.
    pub category: String,
    /// Mô tả vấn đề
    pub message: String,
    /// Gợi ý fix
    pub suggestion: String,
}

/// Một security issue phát hiện trong SQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    /// "critical", "high", "medium", "low"
    pub severity: String,
    /// "potential_injection", "excessive_privilege", "data_exposure", "missing_where_clause"
    pub issue_type: String,
    /// Mô tả vấn đề
    pub message: String,
    /// Hướng khắc phục
    pub remediation: String,
}

/// Ước lượng cost/impact khi thực thi SQL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimation {
    /// Estimated rows affected
    pub estimated_rows: Option<String>,
    /// Lock duration estimation: "minimal", "moderate", "heavy"
    pub lock_impact: String,
    /// Replication impact note
    pub replication_note: Option<String>,
    /// Estimated execution time: "fast (<1s)", "moderate (1-10s)", "slow (>10s)"
    pub estimated_duration: Option<String>,
}

// ── Error Types ────────────────────────────────────────────────────────

/// Lỗi khi thực hiện AI analysis.
#[derive(Error, Debug)]
pub enum AiError {
    #[error("AI service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("AI request timeout after {0}s")]
    Timeout(u64),

    #[error("AI response parse error: {0}")]
    ParseError(String),

    #[error("AI rate limited, retry after {0}s")]
    RateLimited(u64),

    #[error("AI feature disabled")]
    Disabled,

    #[error("schema introspection failed: {0}")]
    SchemaError(String),
}

// ── AI Config ──────────────────────────────────────────────────────────

/// Cấu hình cho AI analysis module.
#[derive(Debug, Clone)]
pub struct AiConfig {
    pub enabled: bool,
    pub provider: AiProvider,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub rate_limit_per_user: u32,
}

/// AI provider được hỗ trợ.
#[derive(Debug, Clone)]
pub enum AiProvider {
    OpenAI {
        api_key: String,
        model: String,
        base_url: String,
    },
    #[allow(dead_code)]
    AwsBedrock {
        region: String,
        model_id: String,
    },
}

impl AiConfig {
    /// Load AI config từ environment variables.
    /// Trả về config với `enabled = false` nếu AI_ENABLED != "true".
    pub fn from_env() -> Self {
        let enabled = std::env::var("AI_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .eq_ignore_ascii_case("true");

        let provider_str = std::env::var("AI_PROVIDER")
            .unwrap_or_else(|_| "openai".to_string());

        let provider = match provider_str.as_str() {
            "bedrock" => AiProvider::AwsBedrock {
                region: std::env::var("AI_BEDROCK_REGION")
                    .unwrap_or_else(|_| "us-east-1".to_string()),
                model_id: std::env::var("AI_BEDROCK_MODEL_ID")
                    .unwrap_or_else(|_| "anthropic.claude-3-haiku-20240307-v1:0".to_string()),
            },
            _ => AiProvider::OpenAI {
                api_key: std::env::var("AI_OPENAI_API_KEY").unwrap_or_default(),
                model: std::env::var("AI_OPENAI_MODEL")
                    .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
                base_url: std::env::var("AI_OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            },
        };

        let timeout_secs = std::env::var("AI_ANALYSIS_TIMEOUT_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .unwrap_or(30)
            .min(120);

        let max_tokens = std::env::var("AI_MAX_TOKENS")
            .unwrap_or_else(|_| "2000".to_string())
            .parse::<u32>()
            .unwrap_or(2000)
            .min(4000);

        let rate_limit_per_user = std::env::var("AI_RATE_LIMIT_PER_USER")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u32>()
            .unwrap_or(5);

        Self {
            enabled,
            provider,
            timeout_secs,
            max_tokens,
            rate_limit_per_user,
        }
    }
}

// ── AI Rate Limiter (per user) ─────────────────────────────────────────

/// Rate limiter per user cho AI analysis endpoint.
/// Sliding window: max N requests per minute per user.
#[derive(Clone)]
pub struct AiRateLimiter {
    inner: Arc<Mutex<HashMap<Uuid, Vec<Instant>>>>,
    max_requests: u32,
    window: Duration,
}

impl AiRateLimiter {
    /// Tạo mới với max requests per user per window.
    pub fn new(max_requests: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window: Duration::from_secs(60),
        }
    }

    /// Kiểm tra và ghi nhận request. Trả `true` nếu được phép.
    pub async fn check_and_record(&self, user_id: Uuid) -> bool {
        let mut map = self.inner.lock().await;
        let now = Instant::now();

        let timestamps = map.entry(user_id).or_default();
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        if timestamps.len() >= self.max_requests as usize {
            return false;
        }

        timestamps.push(now);
        true
    }
}

// ── AI Analysis Cache (per request) ────────────────────────────────────

/// Cache kết quả AI analysis per request.
/// Mỗi entry có TTL — tự expire sau thời gian cấu hình.
/// Key = (request_id, sql_content_hash) — nếu SQL thay đổi thì cache miss.
#[derive(Clone)]
pub struct AiCache {
    inner: Arc<Mutex<HashMap<String, CacheEntry>>>,
    ttl: Duration,
}

/// Một entry trong AI analysis cache, lưu response kèm thời điểm tạo.
struct CacheEntry {
    /// Kết quả phân tích AI đã cache.
    response: AiAnalysisResponse,
    /// Thời điểm entry được tạo — dùng để tính TTL expiry.
    created_at: Instant,
}

impl AiCache {
    /// Tạo cache mới với TTL (default 10 phút).
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// Cache key = request_id + hash(sql_content).
    fn make_key(request_id: Uuid, sql_content: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        sql_content.hash(&mut hasher);
        format!("{}:{}", request_id, hasher.finish())
    }

    /// Lấy từ cache nếu còn valid.
    pub async fn get(&self, request_id: Uuid, sql_content: &str) -> Option<AiAnalysisResponse> {
        let key = Self::make_key(request_id, sql_content);
        let mut map = self.inner.lock().await;

        if let Some(entry) = map.get(&key) {
            if entry.created_at.elapsed() < self.ttl {
                return Some(entry.response.clone());
            }
            // Expired — remove
            map.remove(&key);
        }
        None
    }

    /// Lưu vào cache.
    pub async fn set(&self, request_id: Uuid, sql_content: &str, response: AiAnalysisResponse) {
        let key = Self::make_key(request_id, sql_content);
        let mut map = self.inner.lock().await;

        // Cleanup entries cũ nếu cache quá lớn (> 500 entries)
        if map.len() > 500 {
            let now = Instant::now();
            map.retain(|_, entry| now.duration_since(entry.created_at) < self.ttl);
        }

        map.insert(key, CacheEntry {
            response,
            created_at: Instant::now(),
        });
    }
}
