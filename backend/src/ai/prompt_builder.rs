//! Prompt Builder — xây dựng prompt tối ưu cho LLM.
//!
//! Bao gồm:
//! - System prompt (role, output format)
//! - Schema context (tables, columns, indexes)
//! - SQL content (đã sanitize)
//! - Analysis dimensions (performance, security, best practices, cost)

use crate::models::database::DbType;
use super::schema_introspector::SchemaContext;

/// Tạo system prompt định nghĩa role và output format cho AI.
///
/// Prompt yêu cầu LLM trả về JSON với cấu trúc cố định bao gồm:
/// performance, security, best_practices, cost_estimation, risk_score,
/// suggested_sql, và summary (tiếng Việt).
///
/// # Returns
///
/// Chuỗi system prompt sẵn sàng gửi tới LLM API.
pub fn build_system_prompt() -> String {
    r#"You are an expert database administrator and SQL analyst. Your role is to analyze SQL statements and provide actionable recommendations.

You MUST respond with a valid JSON object following this exact structure:
{
  "performance": [
    {"severity": "high|medium|low", "category": "string", "message": "string", "suggestion": "string"}
  ],
  "security": [
    {"severity": "critical|high|medium|low", "issue_type": "string", "message": "string", "remediation": "string"}
  ],
  "best_practices": [
    {"severity": "high|medium|low", "category": "string", "message": "string", "suggestion": "string"}
  ],
  "cost_estimation": {
    "estimated_rows": "string or null",
    "lock_impact": "minimal|moderate|heavy",
    "replication_note": "string or null",
    "estimated_duration": "string or null"
  },
  "risk_score": 1-10,
  "suggested_sql": "optimized SQL string or null",
  "summary": "brief one-paragraph summary in Vietnamese"
}

Analysis Categories:
- Performance: missing_index, full_scan, cartesian_join, select_star, subquery_optimization, n_plus_one, unnecessary_sort
- Security: missing_where_clause, potential_injection, excessive_privilege, data_exposure, unparameterized_delete, mass_update
- Best Practices: explicit_columns, transaction_usage, naming_convention, null_handling, data_type_mismatch

Rules:
- Always write summary in Vietnamese
- Be specific: reference actual table/column names from the schema
- If schema info is unavailable, base analysis on SQL patterns only
- Severity levels: critical > high > medium > low
- risk_score: 1 = trivial safe query, 10 = extremely dangerous (mass delete without WHERE)
- suggested_sql should be null if no improvement is needed
- Do NOT include markdown formatting in the JSON response"#.to_string()
}

/// Tạo user prompt chứa SQL cần phân tích kèm schema context.
///
/// Prompt bao gồm:
/// - Loại database đích
/// - Thông tin schema (tables, columns, indexes) nếu có
/// - Câu SQL đã được sanitize (string literals thay bằng `'***'`)
/// - Hướng dẫn phân tích theo 4 chiều (performance, security, best practices, cost)
///
/// # Arguments
///
/// * `sql` — Câu SQL gốc cần phân tích (sẽ được sanitize trước khi đưa vào prompt)
/// * `schema` — Thông tin schema của database đích (tables, columns, indexes)
/// * `db_type` — Loại database (PostgreSQL, MySQL, Oracle, MongoDB)
///
/// # Returns
///
/// Chuỗi user prompt sẵn sàng gửi tới LLM API.
pub fn build_user_prompt(sql: &str, schema: &SchemaContext, db_type: &DbType) -> String {
    let mut prompt = String::with_capacity(4096);

    // Database type
    prompt.push_str(&format!("## Database Type\n{}\n\n", db_type));

    // Schema context
    if !schema.tables.is_empty() {
        prompt.push_str("## Schema Context\n");
        for table in &schema.tables {
            prompt.push_str(&format!("### Table: `{}`", table.name));
            if let Some(count) = table.row_count_estimate {
                prompt.push_str(&format!(" (~{} rows)", format_row_count(count)));
            }
            prompt.push('\n');

            prompt.push_str("| Column | Type | Nullable | PK |\n");
            prompt.push_str("|--------|------|----------|----|\n");
            for col in &table.columns {
                prompt.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    col.name,
                    col.data_type,
                    if col.is_nullable { "YES" } else { "NO" },
                    if col.is_primary_key { "✓" } else { "" }
                ));
            }
            prompt.push('\n');
        }

        if !schema.indexes.is_empty() {
            prompt.push_str("### Indexes\n");
            for idx in &schema.indexes {
                prompt.push_str(&format!(
                    "- `{}` on `{}` ({}) — columns: [{}]\n",
                    idx.name,
                    idx.table_name,
                    if idx.is_unique { "UNIQUE" } else { "NON-UNIQUE" },
                    idx.columns.join(", ")
                ));
            }
            prompt.push('\n');
        }
    } else {
        prompt.push_str("## Schema Context\nNo schema information available. Analyze based on SQL patterns only.\n\n");
    }

    // SQL content (sanitized)
    let sanitized_sql = sanitize_sql_literals(sql);
    prompt.push_str("## SQL to Analyze\n```sql\n");
    prompt.push_str(&sanitized_sql);
    prompt.push_str("\n```\n\n");

    prompt.push_str("## Instructions\nAnalyze the above SQL for:\n");
    prompt.push_str("1. Performance issues (missing indexes, full scans, inefficient patterns)\n");
    prompt.push_str("2. Security risks (missing WHERE on DELETE/UPDATE, data exposure, injection patterns)\n");
    prompt.push_str("3. Best practices violations (SELECT *, naming, transaction usage)\n");
    prompt.push_str("4. Cost estimation (rows affected, lock impact, execution duration)\n\n");
    prompt.push_str("Provide your analysis as a JSON object following the specified schema.");

    prompt
}

/// Sanitize SQL string literals — thay giá trị thực bằng '***'.
///
/// Mục đích: không gửi dữ liệu nhạy cảm (PII, secrets) ra AI API.
/// Giữ nguyên structure SQL và column references.
///
/// # Examples
/// ```
/// assert_eq!(
///     sanitize_sql_literals("SELECT * FROM users WHERE name = 'John Doe'"),
///     "SELECT * FROM users WHERE name = '***'"
/// );
/// ```
pub fn sanitize_sql_literals(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\'' {
            // Tìm closing quote (handle escaped quotes '')
            result.push('\'');
            i += 1;
            let mut found_end = false;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if i + 1 < chars.len() && chars[i + 1] == '\'' {
                        // Escaped quote — skip
                        i += 2;
                    } else {
                        // End of string literal
                        found_end = true;
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            result.push_str("***");
            if found_end {
                result.push('\'');
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Format số lượng rows thành chuỗi human-readable.
///
/// - >= 1,000,000 → hiển thị dạng "X.XM" (triệu)
/// - >= 1,000 → hiển thị dạng "X.XK" (nghìn)
/// - < 1,000 → hiển thị nguyên số
///
/// # Arguments
///
/// * `count` — Số lượng rows ước tính.
///
/// # Returns
///
/// Chuỗi formatted (ví dụ: "5.5M", "42.0K", "500").
fn format_row_count(count: i64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_simple_string() {
        let sql = "SELECT * FROM users WHERE name = 'John'";
        let result = sanitize_sql_literals(sql);
        assert_eq!(result, "SELECT * FROM users WHERE name = '***'");
    }

    #[test]
    fn test_sanitize_multiple_strings() {
        let sql = "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')";
        let result = sanitize_sql_literals(sql);
        assert_eq!(result, "INSERT INTO users (name, email) VALUES ('***', '***')");
    }

    #[test]
    fn test_sanitize_escaped_quotes() {
        let sql = "SELECT * FROM users WHERE name = 'O''Brien'";
        let result = sanitize_sql_literals(sql);
        assert_eq!(result, "SELECT * FROM users WHERE name = '***'");
    }

    #[test]
    fn test_sanitize_no_strings() {
        let sql = "SELECT id, count FROM users WHERE id = 123";
        let result = sanitize_sql_literals(sql);
        assert_eq!(result, sql); // No change
    }

    #[test]
    fn test_sanitize_empty() {
        assert_eq!(sanitize_sql_literals(""), "");
    }

    #[test]
    fn test_format_row_count_millions() {
        assert_eq!(format_row_count(5_500_000), "5.5M");
    }

    #[test]
    fn test_format_row_count_thousands() {
        assert_eq!(format_row_count(42_000), "42.0K");
    }

    #[test]
    fn test_format_row_count_small() {
        assert_eq!(format_row_count(500), "500");
    }

    #[test]
    fn test_build_user_prompt_has_sql() {
        let schema = SchemaContext { tables: vec![], indexes: vec![] };
        let prompt = build_user_prompt("SELECT 1", &schema, &DbType::PostgreSQL);
        assert!(prompt.contains("SELECT 1"));
        assert!(prompt.contains("postgresql"));
    }

    #[test]
    fn test_build_user_prompt_with_schema() {
        use super::super::schema_introspector::{TableInfo, ColumnInfo, IndexInfo};

        let schema = SchemaContext {
            tables: vec![TableInfo {
                name: "users".to_string(),
                columns: vec![
                    ColumnInfo { name: "id".to_string(), data_type: "uuid".to_string(), is_nullable: false, is_primary_key: true },
                    ColumnInfo { name: "email".to_string(), data_type: "varchar".to_string(), is_nullable: false, is_primary_key: false },
                ],
                row_count_estimate: Some(50000),
            }],
            indexes: vec![IndexInfo {
                name: "idx_users_email".to_string(),
                table_name: "users".to_string(),
                columns: vec!["email".to_string()],
                is_unique: true,
            }],
        };

        let prompt = build_user_prompt("SELECT * FROM users WHERE email = 'test@test.com'", &schema, &DbType::PostgreSQL);
        assert!(prompt.contains("users"));
        assert!(prompt.contains("email"));
        assert!(prompt.contains("idx_users_email"));
        assert!(prompt.contains("50.0K rows"));
        // Verify SQL is sanitized
        assert!(prompt.contains("'***'"));
        assert!(!prompt.contains("test@test.com"));
    }
}
