# AI SQL Analysis — Thiết kế tính năng

## Tổng quan

Tích hợp AI (LLM) để phân tích, tối ưu và đưa ra recommendations cho các câu lệnh SQL trong hệ thống SECS.

## Quyết định kiến trúc

| Yêu cầu | Quyết định |
|----------|-----------|
| Provider | External API (OpenAI hoặc AWS Bedrock) |
| Scope phân tích | Performance + Security (SQL injection patterns) + Best practices + Cost estimation |
| Trigger | Khi submit (Maker) và khi review (Checker) |
| Schema awareness | Có — introspect schema từ database đích rồi đưa vào prompt |
| Bảo mật | Công ty cho phép gửi SQL ra external API |

---

## Kiến trúc tổng thể

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend                                                    │
│  ┌─────────────┐         ┌──────────────────┐              │
│  │NewRequest   │─submit──▶│ AI Analysis Panel│              │
│  │(Maker)      │         │ (auto-triggered) │              │
│  └─────────────┘         └──────────────────┘              │
│  ┌─────────────┐         ┌──────────────────┐              │
│  │ReviewDetail │─review──▶│ AI Analysis Panel│              │
│  │(Checker)    │         │ (auto-triggered) │              │
│  └─────────────┘         └──────────────────┘              │
└──────────────────────────────┬──────────────────────────────┘
                               │ POST /api/v1/requests/:id/analyze
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Backend (Rust/Axum)                                         │
│                                                              │
│  api/analysis.rs ──▶ ai/mod.rs ──▶ ai/schema_introspector.rs│
│       │                  │              │                    │
│       │                  │              ▼                    │
│       │                  │    executor/ (connect DB đích,    │
│       │                  │    query information_schema)      │
│       │                  ▼                                   │
│       │           ai/llm_client.rs                           │
│       │              │                                       │
│       │              ▼                                       │
│       │     ┌─────────────────┐                             │
│       │     │ OpenAI API      │ (hoặc AWS Bedrock)          │
│       │     │ GPT-4o / Claude │                             │
│       │     └─────────────────┘                             │
│       ▼                                                      │
│  Trả về: AiAnalysisResponse                                 │
│  {recommendations, security_issues, optimizations, cost}     │
└─────────────────────────────────────────────────────────────┘
```

---

## Module mới trong Backend

```
backend/src/
├── ai/                          # Module mới
│   ├── mod.rs                   # AiAnalyzer, response types, orchestration
│   ├── llm_client.rs            # HTTP client gọi OpenAI/Bedrock API
│   ├── schema_introspector.rs   # Introspect schema từ DB đích
│   └── prompt_builder.rs        # Xây dựng prompt tối ưu cho LLM
```

---

## Chi tiết thiết kế

### 1. Schema Introspection (`ai/schema_introspector.rs`)

Trước khi gọi AI, lấy schema context từ database đích để AI có đủ thông tin đưa ra recommendation chính xác.

```rust
pub struct SchemaContext {
    pub tables: Vec<TableInfo>,    // tên bảng, columns, data types
    pub indexes: Vec<IndexInfo>,   // indexes trên các bảng liên quan
    pub constraints: Vec<String>,  // FK, PK, unique constraints
}

pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub row_count_estimate: Option<i64>,
}

pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
}

pub struct IndexInfo {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
}
```

**Cách lấy schema theo db_type:**

| DB Type | Source |
|---------|--------|
| PostgreSQL | `information_schema.tables`, `information_schema.columns`, `pg_indexes`, `pg_stat_user_tables` |
| MySQL | `information_schema.TABLES`, `information_schema.COLUMNS`, `information_schema.STATISTICS` |
| Oracle | `ALL_TAB_COLUMNS`, `ALL_INDEXES`, `ALL_TABLES`, `ALL_IND_COLUMNS` |
| MongoDB | `listCollections` + sample document để infer schema |

**Scope introspection**: Chỉ introspect tables/collections được **reference trong SQL** (parse SQL → extract table names → query metadata). Không dump toàn bộ schema để tiết kiệm token và giảm noise.

---

### 2. Prompt Builder (`ai/prompt_builder.rs`)

Xây dựng structured prompt gồm các phần:

1. **System prompt**: Role (SQL expert DBA), output format (JSON structured)
2. **Database context**: Loại DB, version (nếu có)
3. **Schema context**: Tables, columns, indexes, constraints liên quan
4. **SQL content**: Câu lệnh cần phân tích
5. **Analysis dimensions**: Performance, Security, Best Practices, Cost estimation

```rust
pub fn build_analysis_prompt(
    sql: &str,
    schema: &SchemaContext,
    db_type: &DbType,
) -> String {
    format!(r#"
You are an expert DBA for {db_type}. Analyze the following SQL statement.

## Database Schema Context
{schema_text}

## SQL to Analyze
```sql
{sql}
```

## Required Analysis (return JSON)
1. **Performance**: Missing indexes, full table scans, cartesian joins, N+1 patterns, unnecessary SELECT *
2. **Security**: SQL injection risks, excessive privilege usage, data exposure risks
3. **Best Practices**: Coding standards, naming, transaction usage, error-prone patterns
4. **Cost Estimation**: Estimated rows affected, lock duration, replication impact

## Output Format
Return a single JSON object with the structure defined below...
"#)
}
```

**Sanitization trước khi gửi prompt:**
- Strip string literals (thay `'actual_value'` → `'***'`) để không leak data nhạy cảm
- Giữ nguyên structure và column references

---

### 3. LLM Client (`ai/llm_client.rs`)

```rust
pub enum LlmProvider {
    OpenAI { api_key: String, model: String, base_url: String },
    AwsBedrock { region: String, model_id: String },
}

pub struct LlmClient {
    provider: LlmProvider,
    http_client: reqwest::Client,
    timeout: Duration,
    max_tokens: u32,
}

impl LlmClient {
    pub async fn analyze(&self, prompt: &str) -> Result<String, AiError>;
}

#[derive(Debug, thiserror::Error)]
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
}
```

---

### 4. Response Models (`ai/mod.rs`)

```rust
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
    /// Summary ngắn gọn cho Checker
    pub summary: String,
}

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
```

---

### 5. API Endpoint (`api/analysis.rs`)

```rust
/// POST /api/v1/requests/:id/analyze
///
/// Trigger: Maker sau khi submit, Checker khi mở review detail.
/// Auth: Maker (chỉ request của mình) HOẶC Checker (request đang PENDING).
///
/// Flow:
/// 1. Lấy SQL + database_id từ request
/// 2. Kiểm tra quyền truy cập (owner hoặc checker)
/// 3. Lấy DB connection info → decrypt credentials
/// 4. Connect DB đích → introspect schema (tables referenced in SQL)
/// 5. Build prompt: schema + sanitized SQL + analysis dimensions
/// 6. Gọi LLM API → parse response JSON
/// 7. Trả AiAnalysisResponse
pub async fn analyze_sql(
    State(state): State<AppState>,
    claims: Claims,           // RequireAuth — cả Maker và Checker đều gọi được
    Path(id): Path<Uuid>,
) -> Result<Json<AiAnalysisResponse>, AppError>;
```

**Access control:**
- Maker: chỉ analyze request của mình (created_by == user_id)
- Checker: chỉ analyze request đang PENDING
- Admin: không cần endpoint này

---

### 6. Tích hợp vào Flow hiện tại

#### Submit flow (Maker):
```
create_request(submit=true) → 201 Created
                            → Frontend tự động gọi POST /analyze
                            → Hiển thị kết quả trong AI Analysis Panel
```

#### Review flow (Checker):
```
GET /approvals/:id → trả detail + sql_analysis (DDL warnings)
                   → Frontend tự động gọi POST /analyze
                   → Hiển thị AI recommendations cạnh DDL warnings
```

**AI analysis là advisory** — không block approve/reject workflow. Checker vẫn có thể approve ngay cả khi AI cảnh báo.

---

## Cấu hình (Config)

### Environment Variables (`.env`)

```env
# ─── AI Analysis ────────────────────────────
AI_ENABLED=true                              # Feature flag — tắt/bật toàn bộ
AI_PROVIDER=openai                           # "openai" | "bedrock"

# OpenAI
AI_OPENAI_API_KEY=sk-...                     # API key
AI_OPENAI_MODEL=gpt-4o                       # Model: gpt-4o, gpt-4o-mini
AI_OPENAI_BASE_URL=https://api.openai.com/v1 # Base URL (hỗ trợ proxy)

# AWS Bedrock (alternative)
AI_BEDROCK_REGION=us-east-1
AI_BEDROCK_MODEL_ID=anthropic.claude-3-haiku-20240307-v1:0

# Common
AI_ANALYSIS_TIMEOUT_SECS=30                  # Timeout cho AI request
AI_MAX_TOKENS=2000                           # Max tokens trong response
AI_RATE_LIMIT_PER_USER=5                     # Max requests/phút per user
```

### Config struct bổ sung (`config.rs`)

```rust
pub struct AiConfig {
    pub enabled: bool,
    pub provider: String,           // "openai" | "bedrock"
    pub openai_api_key: Option<String>,
    pub openai_model: String,
    pub openai_base_url: String,
    pub bedrock_region: Option<String>,
    pub bedrock_model_id: Option<String>,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub rate_limit_per_user: u32,
}
```

---

## Bảo mật

| Concern | Mitigation |
|---------|-----------|
| SQL chứa data nhạy cảm trong string literals | Strip string literals trước khi gửi AI (thay `'value'` → `'***'`) |
| Schema metadata leak | Chỉ gửi table name + column name + data type, không gửi actual data |
| API key storage | Lưu trong env var, không log, không trả trong API response |
| Rate limiting | Limit AI requests/phút per user (configurable, default 5) |
| Response validation | Parse AI response qua schema validation trước khi trả client |
| Audit | Log mỗi lần gọi AI analyze (request_id, user_id, timestamp) — không log prompt/response |
| Feature flag | `AI_ENABLED=false` để disable hoàn toàn khi cần |

---

## Frontend Integration

### AI Analysis Panel Component

Hiển thị trong cả `NewRequest.svelte` (sau submit) và `ReviewDetail.svelte`:

```
┌─────────────────────────────────────────────┐
│ 🤖 AI SQL Analysis                    [↻]   │
├─────────────────────────────────────────────┤
│ Risk Score: ██████░░░░ 6/10                  │
│                                              │
│ ⚠️ Performance (2 issues)                    │
│   ├─ 🔴 HIGH: Missing index on users.email  │
│   │   → CREATE INDEX idx_users_email ON...   │
│   └─ 🟡 MED: Sequential scan on orders      │
│       → Add WHERE clause with indexed col    │
│                                              │
│ 🔒 Security (1 issue)                       │
│   └─ 🔴 HIGH: DELETE without WHERE clause   │
│       → Add WHERE condition to limit scope   │
│                                              │
│ 💡 Best Practices (1 suggestion)            │
│   └─ 🟢 LOW: Use explicit column names      │
│       → Replace SELECT * with column list    │
│                                              │
│ 💰 Cost Estimation                           │
│   ├─ Rows affected: ~50,000                  │
│   ├─ Lock impact: Heavy (table-level)        │
│   └─ Duration: Moderate (1-10s)              │
│                                              │
│ 📝 Suggested SQL                      [Copy] │
│ ┌─────────────────────────────────────────┐ │
│ │ DELETE FROM orders                       │ │
│ │ WHERE status = 'expired'                 │ │
│ │   AND created_at < NOW() - INTERVAL...   │ │
│ └─────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

### API call từ Frontend

```javascript
// src/api/index.js — thêm function mới
export async function analyzeRequest(requestId) {
    return apiFetch(`/api/v1/requests/${requestId}/analyze`, {
        method: 'POST',
    });
}
```

---

## Dependencies mới

### Backend (Cargo.toml)

```toml
# reqwest đã có sẵn (dùng cho OIDC) — chỉ cần ensure features
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Nếu dùng AWS Bedrock (optional):
# aws-config = "1"
# aws-sdk-bedrockruntime = "1"
```

### Frontend

Không cần thêm dependency — dùng `apiFetch` hiện có.

---

## Roadmap triển khai

| Phase | Nội dung | Effort | Priority |
|-------|----------|--------|----------|
| 1 | `ai/` module + OpenAI client + basic prompt + response types | 2-3 ngày | High |
| 2 | Schema introspection (PostgreSQL + MySQL) | 1-2 ngày | High |
| 3 | API endpoint `POST /requests/:id/analyze` + auth/access control | 1 ngày | High |
| 4 | Frontend: AI Analysis Panel component + integration | 1-2 ngày | High |
| 5 | Oracle + MongoDB schema introspection | 1 ngày | Medium |
| 6 | SQL sanitization (strip string literals) | 0.5 ngày | High |
| 7 | Rate limiting + caching (cache kết quả per request) | 1 ngày | Medium |
| 8 | AWS Bedrock alternative provider | 1 ngày | Low |

**Tổng estimate: ~8-12 ngày**

---

## Các hướng mở rộng tương lai

1. **Cache analysis results**: Lưu kết quả AI vào DB, không cần gọi lại khi SQL không đổi
2. **Batch analysis**: Cho phép analyze nhiều statements cùng lúc
3. **Learning from history**: Dùng lịch sử execution (success/fail, rows_affected) để fine-tune recommendations
4. **Custom rules per database**: Mỗi DB connection có thể có custom analysis rules (VD: "bảng X không được DELETE")
5. **Self-hosted model**: Fallback sang Ollama/vLLM nếu external API unavailable
6. **Explain Plan integration**: Kết hợp EXPLAIN output (đã có trong `validate_sql`) với AI analysis để recommendation chính xác hơn
