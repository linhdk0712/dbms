//! SQL Analyzer — phát hiện DDL statements nguy hiểm.
//!
//! BR-005 (domain.md): "Các câu lệnh DDL (DROP, TRUNCATE, ALTER) cần flag
//! cảnh báo đặc biệt khi Checker review"
//!
//! Module này cung cấp:
//! - Phát hiện DDL keywords trong SQL
//! - Phân loại mức độ nguy hiểm (WARNING / DANGER)
//! - Response struct để trả về cho frontend (Checker review UI)

use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

/// Mức độ nguy hiểm của DDL statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DdlSeverity {
    /// ALTER, CREATE — thay đổi schema nhưng không mất data
    Warning,
    /// DROP, TRUNCATE — có thể gây mất dữ liệu không phục hồi
    Danger,
}

/// Một DDL warning cụ thể phát hiện trong SQL.
#[derive(Debug, Clone, Serialize)]
pub struct DdlWarning {
    /// Keyword DDL được phát hiện (VD: "DROP TABLE")
    pub keyword: String,
    /// Mức độ nguy hiểm
    pub severity: DdlSeverity,
    /// Mô tả ngắn cho Checker
    pub message: String,
    /// Vị trí (index) trong chuỗi SQL gốc
    pub position: usize,
}

/// Kết quả phân tích SQL.
#[derive(Debug, Clone, Serialize)]
pub struct SqlAnalysis {
    /// Danh sách DDL warnings (rỗng nếu SQL chỉ có DML)
    pub ddl_warnings: Vec<DdlWarning>,
    /// SQL có chứa DDL statements nguy hiểm không
    pub contains_ddl: bool,
    /// Có statement nào mức DANGER không (DROP/TRUNCATE)
    pub has_danger: bool,
}

/// Các pattern DDL cần detect (case-insensitive).
static DDL_PATTERNS: LazyLock<Vec<DdlPattern>> = LazyLock::new(|| {
    vec![
        DdlPattern {
            regex: Regex::new(r"(?i)\b(DROP\s+TABLE)\b").unwrap(),
            severity: DdlSeverity::Danger,
            message: "DROP TABLE sẽ xóa toàn bộ bảng và dữ liệu — không thể phục hồi".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(DROP\s+INDEX)\b").unwrap(),
            severity: DdlSeverity::Danger,
            message: "DROP INDEX sẽ xóa index — có thể ảnh hưởng performance".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(DROP\s+DATABASE)\b").unwrap(),
            severity: DdlSeverity::Danger,
            message: "DROP DATABASE sẽ xóa toàn bộ database — CỰC KỲ NGUY HIỂM".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(DROP\s+SCHEMA)\b").unwrap(),
            severity: DdlSeverity::Danger,
            message: "DROP SCHEMA sẽ xóa toàn bộ schema và các objects bên trong".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(DROP\s+VIEW)\b").unwrap(),
            severity: DdlSeverity::Warning,
            message: "DROP VIEW sẽ xóa view — có thể ảnh hưởng các queries phụ thuộc".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(DROP\s+SEQUENCE)\b").unwrap(),
            severity: DdlSeverity::Warning,
            message: "DROP SEQUENCE sẽ xóa sequence — có thể ảnh hưởng auto-increment".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(DROP\s+\w+)\b").unwrap(),
            severity: DdlSeverity::Warning,
            message: "DROP statement detected — verify target object carefully".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(TRUNCATE\s+TABLE?)\b").unwrap(),
            severity: DdlSeverity::Danger,
            message: "TRUNCATE xóa toàn bộ dữ liệu trong bảng — không thể rollback".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(ALTER\s+TABLE)\b").unwrap(),
            severity: DdlSeverity::Warning,
            message: "ALTER TABLE thay đổi cấu trúc bảng — có thể gây downtime hoặc lock".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(CREATE\s+TABLE)\b").unwrap(),
            severity: DdlSeverity::Warning,
            message: "CREATE TABLE — tạo bảng mới, verify schema design".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(CREATE\s+INDEX)\b").unwrap(),
            severity: DdlSeverity::Warning,
            message: "CREATE INDEX có thể gây lock bảng khi tạo trên bảng lớn".to_string(),
        },
        DdlPattern {
            regex: Regex::new(r"(?i)\b(RENAME\s+TABLE)\b").unwrap(),
            severity: DdlSeverity::Warning,
            message: "RENAME TABLE — các queries tham chiếu tên cũ sẽ bị lỗi".to_string(),
        },
    ]
});

struct DdlPattern {
    regex: Regex,
    severity: DdlSeverity,
    message: String,
}

/// Phân tích SQL để phát hiện DDL statements.
/// Trả về `SqlAnalysis` với danh sách warnings.
///
/// # Arguments
/// * `sql` - Nội dung SQL hoặc JSON commands (MongoDB sẽ trả empty)
///
/// # Examples
/// ```
/// let analysis = analyze_sql("DROP TABLE users; INSERT INTO logs VALUES (1);");
/// assert!(analysis.contains_ddl);
/// assert!(analysis.has_danger);
/// ```
pub fn analyze_sql(sql: &str) -> SqlAnalysis {
    let mut warnings: Vec<DdlWarning> = Vec::new();
    let mut seen_positions: Vec<usize> = Vec::new();

    for pattern in DDL_PATTERNS.iter() {
        for mat in pattern.regex.find_iter(sql) {
            // Tránh duplicate warnings cho cùng vị trí
            // (vì "DROP TABLE" match cả pattern "DROP TABLE" và generic "DROP \w+")
            let pos = mat.start();
            if seen_positions.iter().any(|&p| p == pos) {
                continue;
            }
            seen_positions.push(pos);

            warnings.push(DdlWarning {
                keyword: mat.as_str().to_uppercase(),
                severity: pattern.severity.clone(),
                message: pattern.message.clone(),
                position: pos,
            });
        }
    }

    // Sắp xếp theo vị trí xuất hiện
    warnings.sort_by_key(|w| w.position);

    let contains_ddl = !warnings.is_empty();
    let has_danger = warnings.iter().any(|w| w.severity == DdlSeverity::Danger);

    SqlAnalysis {
        ddl_warnings: warnings,
        contains_ddl,
        has_danger,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_ddl_in_pure_dml() {
        let analysis = analyze_sql("INSERT INTO users (name) VALUES ('test')");
        assert!(!analysis.contains_ddl);
        assert!(!analysis.has_danger);
        assert!(analysis.ddl_warnings.is_empty());
    }

    #[test]
    fn test_detect_drop_table() {
        let analysis = analyze_sql("DROP TABLE users");
        assert!(analysis.contains_ddl);
        assert!(analysis.has_danger);
        assert_eq!(analysis.ddl_warnings[0].severity, DdlSeverity::Danger);
    }

    #[test]
    fn test_detect_truncate() {
        let analysis = analyze_sql("TRUNCATE TABLE orders");
        assert!(analysis.contains_ddl);
        assert!(analysis.has_danger);
    }

    #[test]
    fn test_detect_alter_table() {
        let analysis = analyze_sql("ALTER TABLE users ADD COLUMN age INT");
        assert!(analysis.contains_ddl);
        assert!(!analysis.has_danger); // ALTER is WARNING, not DANGER
        assert_eq!(analysis.ddl_warnings[0].severity, DdlSeverity::Warning);
    }

    #[test]
    fn test_case_insensitive() {
        let analysis = analyze_sql("drop table USERS");
        assert!(analysis.contains_ddl);
        assert!(analysis.has_danger);
    }

    #[test]
    fn test_mixed_dml_and_ddl() {
        let sql = "INSERT INTO logs VALUES (1); DROP TABLE temp_data; UPDATE users SET active = true";
        let analysis = analyze_sql(sql);
        assert!(analysis.contains_ddl);
        assert!(analysis.has_danger);
    }

    #[test]
    fn test_create_index_is_warning() {
        let analysis = analyze_sql("CREATE INDEX idx_users_email ON users(email)");
        assert!(analysis.contains_ddl);
        assert!(!analysis.has_danger);
        assert_eq!(analysis.ddl_warnings[0].severity, DdlSeverity::Warning);
    }

    #[test]
    fn test_drop_database_is_danger() {
        let analysis = analyze_sql("DROP DATABASE production");
        assert!(analysis.has_danger);
    }

    #[test]
    fn test_multiple_ddl_statements() {
        let sql = "DROP TABLE a; ALTER TABLE b ADD COLUMN c INT; TRUNCATE TABLE d";
        let analysis = analyze_sql(sql);
        assert!(analysis.contains_ddl);
        assert!(analysis.has_danger);
        // Should have at least 3 distinct warnings
        assert!(analysis.ddl_warnings.len() >= 3);
    }

    #[test]
    fn test_select_with_drop_in_string_literal() {
        // "DROP" inside a string literal — phân tích đơn giản vẫn sẽ detect
        // Đây là trade-off: false positive tốt hơn false negative cho security
        let analysis = analyze_sql("SELECT 'DROP TABLE users' AS example");
        // Sẽ detect — đây là expected behavior (better safe than sorry)
        assert!(analysis.contains_ddl);
    }

    #[test]
    fn test_mongodb_json_no_ddl() {
        let json = r#"{"insert": "users", "documents": [{"name": "test"}]}"#;
        let analysis = analyze_sql(json);
        assert!(!analysis.contains_ddl);
    }

    #[test]
    fn test_warnings_sorted_by_position() {
        let sql = "ALTER TABLE a ADD x INT; DROP TABLE b";
        let analysis = analyze_sql(sql);
        for i in 1..analysis.ddl_warnings.len() {
            assert!(analysis.ddl_warnings[i].position >= analysis.ddl_warnings[i - 1].position);
        }
    }
}
