# Best Practices: Phân quyền truy vấn dữ liệu (Data Query Permission)

## Tổng quan tính năng

Cho phép team AppOps truy vấn dữ liệu từ database production với cơ chế kiểm soát chặt chẽ:
- **Maker** tạo Data Query Request (chọn columns, build SQL) cho mình và nhiều người khác run
- **DBA** cấu hình approval matrix (rule phê duyệt) theo table/schema
- **Approvers** (ITO Lead / Compliance / ITS) phê duyệt theo ma trận đã cấu hình
- Quyền được phân theo **column, table, schema** cụ thể
- Mỗi truy vấn được **audit log** đầy đủ
- Mỗi request gắn với **ticket trên ServiceDesk**

### Core Concepts

| Concept | Mô tả |
|---------|--------|
| Data Query Request | Câu SQL đã được approve, nhiều user có thể run |
| Approval Matrix | DBA cấu hình rule phê duyệt cho từng table/schema |
| Assigned Users | Những người được Maker assign — có thể run query sau approve |
| Hybrid Mode | Basic (UI builder) + Advanced (free-form SQL) |

---

## 1. Kiến trúc tổng thể

### 1.1 Flow chính

```
Maker tạo Query Request (Basic/Advanced mode)
  + Chọn database, schema, table, columns
  + Build SQL (auto-generate hoặc viết tay)
  + Assign users được phép run
  + Gắn ServiceDesk ticket
    │
    ▼
Hệ thống lookup Approval Matrix theo (database, schema, table)
    │
    ▼
Multi-level Sequential Approval (ITO → Compliance → ITS)
    │
    ▼ (All levels approved)
Query APPROVED → Executable bởi Maker + Assigned Users
    │
    ▼
Mỗi lần run → Audit log (who, when, rows returned)
```

### 1.2 Phân biệt với hệ thống Maker-Checker hiện tại

| | SQL Execution (hiện tại) | Data Query (mới) |
|---|---|---|
| Mục đích | Thực thi DML/DDL trên production | Truy vấn đọc dữ liệu (SELECT only) |
| Loại SQL | INSERT, UPDATE, DELETE, DDL | SELECT only |
| Ai run | Hệ thống tự execute sau approve | User tự run (nhiều lần) |
| Shared | Không (1 request = 1 maker) | Có (maker assign nhiều users) |
| Approval | Checker (1 level) | Ma trận linh hoạt (multi-level) |
| Kết quả | rows_affected | Dataset trả về cho user |

---

## 2. Hybrid Query Mode

### 2.1 Basic Mode — UI Query Builder

User chọn columns từ giao diện, hệ thống auto-generate SQL:

```
┌─────────────────────────────────────────────────────┐
│ Mode: [Basic ▼]                                     │
│                                                     │
│ Database: [PROD_CORE ▼]                             │
│ Schema:   [customer ▼]                              │
│ Table:    [transactions ▼]                          │
│                                                     │
│ Columns:                                            │
│   ☑ id  ☑ amount  ☑ status  ☑ created_at           │
│   ☐ customer_ssn  ☐ internal_notes                  │
│                                                     │
│ Filters:                                            │
│   created_at  [>=]  [2026-01-01]          [+ Add]   │
│   status      [=]   [COMPLETED ▼]                   │
│   amount      [>]   [1000000]                       │
│                                                     │
│ Limit: [1000]                                       │
│                                                     │
│ Generated SQL (read-only):                          │
│ ┌─────────────────────────────────────────────────┐ │
│ │ SELECT id, amount, status, created_at           │ │
│ │ FROM customer.transactions                      │ │
│ │ WHERE created_at >= '2026-01-01'                │ │
│ │   AND status = 'COMPLETED'                      │ │
│ │   AND amount > 1000000                          │ │
│ │ LIMIT 1000                                      │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│ Assign Users: [User A ×] [User B ×] [+ Add]        │
│ Ticket: [INC-12345]                                 │
│ Expires: [2026-09-01]                               │
│                                                     │
│ [Submit for Approval]                               │
└─────────────────────────────────────────────────────┘
```

### 2.2 Advanced Mode — Free-form SQL

User viết SQL tự do, hệ thống parse + validate:

```
┌─────────────────────────────────────────────────────┐
│ Mode: [Advanced ▼]                                  │
│                                                     │
│ Database: [PROD_CORE ▼]                             │
│                                                     │
│ SQL Editor:                                         │
│ ┌─────────────────────────────────────────────────┐ │
│ │ SELECT t.id, t.amount, t.status, c.name         │ │
│ │ FROM customer.transactions t                    │ │
│ │ JOIN customer.customer_info c ON t.cust_id=c.id │ │
│ │ WHERE t.created_at >= '2026-01-01'              │ │
│ │   AND t.amount > 1000000                        │ │
│ │ ORDER BY t.amount DESC                          │ │
│ │ LIMIT 500                                       │ │
│ └─────────────────────────────────────────────────┘ │
│                                                     │
│ Validation Results:                                 │
│ ✓ Read-only (SELECT only)                           │
│ ✓ Tables: transactions, customer_info               │
│ ✓ Columns: id, amount, status, name, cust_id       │
│ ⚠ Approval required: ITO + Compliance (2 tables)   │
│                                                     │
│ Assign Users: [User A ×] [User B ×] [+ Add]        │
│ Ticket: [INC-12345]                                 │
│                                                     │
│ [Submit for Approval]                               │
└─────────────────────────────────────────────────────┘
```

### 2.3 SQL Generation (Basic Mode)

```rust
/// Generate SQL từ UI builder selections
fn generate_query_sql(
    schema: &str,
    table: &str,
    columns: &[String],
    filters: &[FilterCondition],
    limit: i64,
) -> String {
    let cols = columns.join(", ");
    let mut sql = format!("SELECT {} FROM {}.{}", cols, schema, table);

    if !filters.is_empty() {
        let conditions: Vec<String> = filters.iter()
            .enumerate()
            .map(|(i, f)| format!("{} {} ${}", f.column, f.operator, i + 1))
            .collect();
        sql.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
    }

    sql.push_str(&format!(" LIMIT {}", limit.min(MAX_QUERY_ROWS)));
    sql
}

struct FilterCondition {
    column: String,
    operator: FilterOp,  // =, !=, >, <, >=, <=, LIKE, IN, BETWEEN
    value: String,
}
```

---

## 3. Ma trận Phê duyệt linh hoạt (Approval Matrix)

### 3.1 Khái niệm

DBA cấu hình **approval rules** khi setup cho từng table/schema. Mỗi rule xác định:
- Cần bao nhiêu cấp phê duyệt
- Đơn vị nào phê duyệt ở mỗi cấp
- Thứ tự tuần tự (level N phải approved trước khi gửi level N+1)

### 3.2 Đơn vị phê duyệt

| Đơn vị | Vai trò | Khi nào cần |
|--------|---------|-------------|
| ITO (Team Lead) | Phê duyệt nội bộ team vận hành | Mọi query đều cần (level 1) |
| ITS (IT Security) | Kiểm tra an toàn thông tin | Dữ liệu nhạy cảm, audit trail, credentials |
| Compliance | Đảm bảo tuân thủ quy định | Dữ liệu khách hàng, tài chính, PII |

### 3.3 Ví dụ cấu hình Approval Matrix

```
┌─────────────────────────────────────────────────────────────────┐
│ Database: PROD_CORE                                             │
├─────────────────┬───────────────────────────────────────────────┤
│ Table/Schema    │ Approval Chain                                │
├─────────────────┼───────────────────────────────────────────────┤
│ *.internal_logs │ Level 1: ITO only                             │
│ *.system_config │ Level 1: ITO only                             │
│ customer.*      │ Level 1: ITO → Level 2: Compliance            │
│ customer.cust_  │                                               │
│   _balance      │ Level 1: ITO → Level 2: Compliance → L3: ITS │
│ financial.*     │ Level 1: ITO → Level 2: ITS                   │
│ financial.      │                                               │
│   settlements   │ Level 1: ITO → Level 2: Compliance → L3: ITS │
│ audit.*         │ Level 1: ITO → Level 2: ITS                   │
└─────────────────┴───────────────────────────────────────────────┘
```

### 3.4 Rule Resolution (Most Specific Wins)

```
Priority: table-level rule > schema-level rule > database-level rule > default

Ví dụ query access "customer.customer_balance":
1. Tìm rule cho (PROD_CORE, customer, customer_balance) → Found! → Dùng rule này
2. Nếu không tìm thấy → fallback (PROD_CORE, customer, *) 
3. Nếu không tìm thấy → fallback (PROD_CORE, *, *)
4. Nếu không tìm thấy → Default: ITO only
```

### 3.5 Multi-table Query → Merge Rules

Khi query truy cập **nhiều tables** (JOIN), hệ thống merge approval chains:

```rust
/// Merge approval rules khi query access nhiều tables
/// Lấy UNION của tất cả approval levels (strictest wins)
fn merge_approval_rules(rules: &[ApprovalRule]) -> Vec<ApprovalLevel> {
    // Ví dụ:
    // Table A cần: [ITO]
    // Table B cần: [ITO, COMPLIANCE]
    // → Merged: [ITO, COMPLIANCE] (strictest)
    //
    // Table A cần: [ITO, ITS]
    // Table B cần: [ITO, COMPLIANCE]
    // → Merged: [ITO, COMPLIANCE, ITS] (union of all unique levels)
    
    let mut all_units: Vec<ApproverUnit> = vec![];
    for rule in rules {
        for level in &rule.approval_levels {
            if !all_units.contains(&level.approver_unit) {
                all_units.push(level.approver_unit.clone());
            }
        }
    }
    // Sắp xếp theo priority: ITO → COMPLIANCE → ITS
    all_units.sort_by_key(|u| match u {
        ApproverUnit::Ito => 0,
        ApproverUnit::Compliance => 1,
        ApproverUnit::Its => 2,
    });
    all_units.into_iter()
        .enumerate()
        .map(|(i, unit)| ApprovalLevel { level_order: (i + 1) as i32, approver_unit: unit, required: true })
        .collect()
}
```

### 3.6 Approval Flow Diagram

```
Maker submit Data Query Request
    │
    ▼
System resolves approval rules cho mỗi table trong query
    │
    ▼
Merge rules → Final approval chain
    │
    ▼
╔═══════════════════════════════════════════════════════╗
║ Level 1: ITO Team Lead                               ║
║   → Approve: tiến Level 2                            ║
║   → Reject: REJECTED (notify Maker)                  ║
╠═══════════════════════════════════════════════════════╣
║ Level 2: Compliance (nếu rule yêu cầu)              ║
║   → Approve: tiến Level 3 (hoặc APPROVED)            ║
║   → Reject: REJECTED (notify Maker + L1 approver)    ║
╠═══════════════════════════════════════════════════════╣
║ Level 3: ITS (nếu rule yêu cầu)                     ║
║   → Approve: APPROVED ✓                              ║
║   → Reject: REJECTED (notify all previous)           ║
╚═══════════════════════════════════════════════════════╝
    │
    ▼ (APPROVED)
Query executable bởi: Maker + tất cả Assigned Users
(Run thoải mái, không giới hạn số lần, trong thời hạn)
```

---

## 4. Shared Query Execution (Multi-user Run)

### 4.1 Khái niệm

- Maker tạo query request và **assign nhiều users** có thể run
- Assigned users **không cần** permission riêng trên DB — được assign = đủ quyền run
- **Chỉ run nguyên câu query** đã được approve — không sửa đổi
- **Run thoải mái** trong thời hạn (không giới hạn số lần)

### 4.2 Ai được assign?

- Bất kỳ user active nào trong hệ thống
- Maker tự động có quyền run (không cần assign chính mình)
- Maker chỉ assign users khi tạo request (status = DRAFT)
- Sau khi submit → không thể thêm/bớt assigned users

### 4.3 Execution Rules

| Rule | Mô tả |
|------|--------|
| Immutable SQL | Sau APPROVED, sql_query không thể sửa |
| No modification | User run nguyên câu SQL, không thêm WHERE/LIMIT |
| Unlimited runs | Trong thời hạn, run bao nhiêu lần cũng được |
| Expiration | Khi hết hạn, KHÔNG ai run được (kể cả Maker) |
| Audit per run | Mỗi lần run đều ghi log riêng |

### 4.4 Authorization Check

```rust
/// Kiểm tra user có quyền execute query request này không
pub async fn can_execute_query(
    pool: &PgPool,
    user_id: Uuid,
    query_request_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM data_query_requests dqr
            WHERE dqr.id = $2
              AND dqr.status = 'APPROVED'
              AND (dqr.expires_at IS NULL OR dqr.expires_at > NOW())
              AND (
                  dqr.created_by = $1
                  OR EXISTS (
                      SELECT 1 FROM data_query_assigned_users dqau
                      WHERE dqau.query_request_id = dqr.id
                        AND dqau.user_id = $1
                  )
              )
        )
        "#
    )
    .bind(user_id)
    .bind(query_request_id)
    .fetch_one(pool)
    .await?;

    Ok(result)
}
```

### 4.5 Execution Flow

```
User click "Run" trên approved query
    │
    ▼
┌────────────────────────────────────────────────┐
│ 1. user ∈ (creator OR assigned_users)?         │
│ 2. status = 'APPROVED'?                        │
│ 3. expires_at chưa qua (hoặc NULL)?           │
└──────────────────┬─────────────────────────────┘
                   │ All pass ✓
                   ▼
┌────────────────────────────────────────────────┐
│ 4. Lấy sql_query nguyên bản từ request        │
│ 5. Execute trên target database (read-only)    │
│ 6. Log vào data_query_executions               │
│ 7. Return dataset cho user                     │
└────────────────────────────────────────────────┘
```

---

## 5. SQL Parsing & Validation (sqlparser-rs)

### 5.1 Validation Pipeline

```
User SQL Input (Basic hoặc Advanced mode)
    │
    ▼
┌─────────────────────────────┐
│ 1. Parse SQL (AST)          │ ← sqlparser-rs
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ 2. Read-only check          │ ← Chỉ cho phép SELECT statements
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ 3. Extract all tables       │ ← visit_relations() (incl. JOINs, subqueries)
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ 4. Extract all columns      │ ← Walk SelectItem nodes
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ 5. Resolve approval matrix  │ ← Lookup rules cho mỗi table
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ 6. Enforce row LIMIT        │ ← Max 10,000 rows
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ 7. Store validated SQL      │ ← Immutable sau approve
└─────────────────────────────┘
```

### 5.2 Implementation (sqlparser-rs + Visitor pattern)

```rust
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::{visit_relations, Expr, ObjectName, Statement};
use core::ops::ControlFlow;

/// Parse SQL và validate read-only
pub fn validate_data_query(sql: &str) -> Result<QueryValidation, QueryValidationError> {
    let dialect = GenericDialect {};
    let stmts = Parser::parse_sql(&dialect, sql)
        .map_err(|e| QueryValidationError::ParseError(e.to_string()))?;

    // 1. Chỉ cho phép SELECT
    for stmt in &stmts {
        match stmt {
            Statement::Query(_) => {},
            _ => return Err(QueryValidationError::NotReadOnly(
                "Only SELECT statements are allowed".into()
            )),
        }
    }

    // 2. Extract tables (bao gồm JOINs, subqueries, CTEs)
    let mut tables = vec![];
    visit_relations(&stmts, |rel: &ObjectName| {
        tables.push(rel.to_string());
        ControlFlow::<()>::Continue(())
    });

    // 3. Extract columns từ SELECT clause
    let columns = extract_columns_from_stmts(&stmts);

    Ok(QueryValidation {
        tables_accessed: tables,
        columns_accessed: columns,
        has_wildcard: check_has_wildcard(&stmts),
        has_limit: check_has_limit(&stmts),
    })
}

struct QueryValidation {
    tables_accessed: Vec<String>,
    columns_accessed: Vec<String>,
    has_wildcard: bool,   // SELECT * detected
    has_limit: bool,      // LIMIT clause present
}
```

### 5.3 Row Limit Enforcement

```rust
const MAX_QUERY_ROWS: i64 = 10_000;

/// Đảm bảo query có LIMIT và không vượt quá max
fn enforce_row_limit(sql: &str) -> Result<String, QueryValidationError> {
    let dialect = GenericDialect {};
    let mut stmts = Parser::parse_sql(&dialect, sql)?;
    
    // Nếu chưa có LIMIT → thêm
    // Nếu LIMIT > MAX → giảm xuống MAX
    // Trả về SQL đã modified
    Ok(stmts[0].to_string())
}
```

---

## 6. Database Schema

### 6.1 Core Tables

```sql
-- ============================================================
-- Table: data_query_requests
-- Trung tâm của flow — chứa câu SQL + metadata
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_requests (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title               TEXT NOT NULL,
    database_id         UUID NOT NULL REFERENCES database_connections(id),
    schema_name         TEXT NOT NULL,
    sql_query           TEXT NOT NULL,          -- Immutable sau APPROVED
    tables_accessed     TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    columns_accessed    TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    query_mode          TEXT NOT NULL DEFAULT 'BASIC'
                        CHECK (query_mode IN ('BASIC', 'ADVANCED')),
    ticket_id           TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'DRAFT'
                        CHECK (status IN (
                            'DRAFT','PENDING_APPROVAL','APPROVED',
                            'REJECTED','EXPIRED','REVOKED'
                        )),
    created_by          UUID NOT NULL REFERENCES users(id),
    current_approval_level INT NOT NULL DEFAULT 0,
    expires_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at         TIMESTAMPTZ,
    revoked_at          TIMESTAMPTZ,
    revoke_reason       TEXT
);
```

```sql
-- ============================================================
-- Table: data_query_assigned_users
-- Users được Maker assign để run query
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_assigned_users (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_request_id    UUID NOT NULL REFERENCES data_query_requests(id) ON DELETE CASCADE,
    user_id             UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    assigned_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(query_request_id, user_id)
);

-- ============================================================
-- Table: data_query_approval_rules (DBA cấu hình)
-- Ma trận phê duyệt theo database/schema/table
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_approval_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    database_id     UUID NOT NULL REFERENCES database_connections(id) ON DELETE CASCADE,
    schema_name     TEXT,                  -- NULL = áp dụng toàn DB
    table_name      TEXT,                  -- NULL = áp dụng toàn schema
    created_by      UUID NOT NULL REFERENCES users(id),
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(database_id, COALESCE(schema_name,'*'), COALESCE(table_name,'*'))
);

-- Các level trong approval chain
CREATE TABLE IF NOT EXISTS data_query_approval_levels (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id         UUID NOT NULL REFERENCES data_query_approval_rules(id) ON DELETE CASCADE,
    level_order     INT NOT NULL,          -- 1, 2, 3...
    approver_unit   TEXT NOT NULL
                    CHECK (approver_unit IN ('ITO', 'ITS', 'COMPLIANCE')),
    required        BOOLEAN NOT NULL DEFAULT TRUE,

    UNIQUE(rule_id, level_order)
);

-- ============================================================
-- Table: data_query_approval_tracking
-- Track status phê duyệt cho từng request × level
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_approval_tracking (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_request_id    UUID NOT NULL REFERENCES data_query_requests(id) ON DELETE CASCADE,
    level_order         INT NOT NULL,
    approver_unit       TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'PENDING'
                        CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED', 'SKIPPED')),
    approved_by         UUID REFERENCES users(id),
    comment             TEXT,
    acted_at            TIMESTAMPTZ,

    UNIQUE(query_request_id, level_order)
);
```

```sql
-- ============================================================
-- Table: data_query_executions
-- Audit log mỗi lần user run query
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_executions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_request_id    UUID NOT NULL REFERENCES data_query_requests(id),
    executed_by         UUID NOT NULL REFERENCES users(id),
    rows_returned       BIGINT NOT NULL DEFAULT 0,
    execution_time_ms   BIGINT NOT NULL DEFAULT 0,
    client_ip           TEXT,
    status              TEXT NOT NULL DEFAULT 'SUCCESS'
                        CHECK (status IN ('SUCCESS','FAILED','TIMEOUT')),
    error_message       TEXT,
    executed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Indexes
-- ============================================================
CREATE INDEX idx_dqr_status ON data_query_requests(status)
    WHERE status IN ('APPROVED', 'PENDING_APPROVAL');
CREATE INDEX idx_dqr_created_by ON data_query_requests(created_by, created_at DESC);
CREATE INDEX idx_dqr_ticket ON data_query_requests(ticket_id);
CREATE INDEX idx_dqr_expires ON data_query_requests(expires_at)
    WHERE expires_at IS NOT NULL AND status = 'APPROVED';

CREATE INDEX idx_dqau_user ON data_query_assigned_users(user_id);
CREATE INDEX idx_dqau_request ON data_query_assigned_users(query_request_id);

CREATE INDEX idx_dqar_lookup ON data_query_approval_rules(database_id, schema_name, table_name)
    WHERE is_active = TRUE;

CREATE INDEX idx_dqat_pending ON data_query_approval_tracking(approver_unit, status)
    WHERE status = 'PENDING';
CREATE INDEX idx_dqat_request ON data_query_approval_tracking(query_request_id);

CREATE INDEX idx_dqe_request ON data_query_executions(query_request_id, executed_at DESC);
CREATE INDEX idx_dqe_user ON data_query_executions(executed_by, executed_at DESC);
CREATE INDEX idx_dqe_date ON data_query_executions(executed_at DESC);
```

---

## 7. Axum Middleware & API Design

### 7.1 Permission Guard Extractor

```rust
use axum::{
    extract::{FromRequestParts, Path, State},
    http::{request::Parts, StatusCode},
};

/// Extractor: kiểm tra user có quyền run một query request cụ thể
pub struct CanRunQuery {
    pub query_request: DataQueryRequest,
}

impl<S> FromRequestParts<S> for CanRunQuery
where
    S: Send + Sync + AsRef<AppState>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let claims = extract_jwt_claims(parts)?;
        let Path(request_id) = Path::<Uuid>::from_request_parts(parts, state).await?;
        let pool = &state.as_ref().db;

        // 1. Load query request
        let request = db::data_queries::get_by_id(pool, request_id).await?
            .ok_or(AppError::NotFound)?;

        // 2. Check status + expiration
        if request.status != "APPROVED" {
            return Err(AppError::Forbidden("Query not approved".into()));
        }
        if let Some(exp) = request.expires_at {
            if exp < Utc::now() {
                return Err(AppError::Forbidden("Query expired".into()));
            }
        }

        // 3. Check user is creator OR assigned
        if request.created_by != claims.user_id {
            let assigned = db::data_queries::is_user_assigned(
                pool, request_id, claims.user_id
            ).await?;
            if !assigned {
                return Err(AppError::Forbidden("Not assigned to this query".into()));
            }
        }

        Ok(CanRunQuery { query_request: request })
    }
}
```

### 7.2 API Endpoints

| Method | Path | Description | Role |
|--------|------|-------------|------|
| **Query Requests** | | | |
| POST | `/api/data-queries` | Maker tạo query request | Any authenticated |
| GET | `/api/data-queries` | List requests (filter by status, user) | Any authenticated |
| GET | `/api/data-queries/:id` | Chi tiết request | Creator/Assigned/Approver |
| PUT | `/api/data-queries/:id` | Update draft | Creator (DRAFT only) |
| POST | `/api/data-queries/:id/submit` | Submit for approval | Creator (DRAFT only) |
| POST | `/api/data-queries/:id/execute` | Run approved query | Creator/Assigned |
| **Approvals** | | | |
| GET | `/api/data-query-approvals/pending` | List pending (by unit) | Approvers |
| POST | `/api/data-queries/:id/approve` | Approve current level | Approver (matching unit) |
| POST | `/api/data-queries/:id/reject` | Reject | Approver (matching unit) |
| **Approval Matrix (DBA)** | | | |
| GET | `/api/data-query-rules` | List approval rules | Admin/DBA |
| POST | `/api/data-query-rules` | Create rule | Admin/DBA |
| PUT | `/api/data-query-rules/:id` | Update rule | Admin/DBA |
| DELETE | `/api/data-query-rules/:id` | Delete rule | Admin/DBA |
| **Audit** | | | |
| GET | `/api/data-query-executions` | Execution audit log | Admin/DBA |
| GET | `/api/data-queries/:id/executions` | Executions of a query | Creator/Assigned/Admin |

### 7.3 Request/Response Examples

**Create Query Request (Basic mode):**
```json
POST /api/data-queries
{
    "title": "Check pending transactions Q3",
    "database_id": "uuid-here",
    "schema_name": "customer",
    "query_mode": "BASIC",
    "table_name": "transactions",
    "columns": ["id", "amount", "status", "created_at"],
    "filters": [
        {"column": "created_at", "operator": ">=", "value": "2026-07-01"},
        {"column": "status", "operator": "=", "value": "PENDING"}
    ],
    "limit": 1000,
    "assigned_user_ids": ["user-uuid-1", "user-uuid-2"],
    "ticket_id": "INC-12345",
    "expires_at": "2026-09-01T00:00:00Z"
}
```

**Create Query Request (Advanced mode):**
```json
POST /api/data-queries
{
    "title": "Cross-table customer analysis",
    "database_id": "uuid-here",
    "schema_name": "customer",
    "query_mode": "ADVANCED",
    "sql": "SELECT t.id, t.amount, c.name FROM customer.transactions t JOIN customer.customer_info c ON t.cust_id = c.id WHERE t.amount > 1000000 LIMIT 500",
    "assigned_user_ids": ["user-uuid-1"],
    "ticket_id": "INC-12345",
    "expires_at": "2026-09-01T00:00:00Z"
}
```

**Execute Query Response:**
```json
{
    "columns": ["id", "amount", "status", "created_at"],
    "rows": [
        ["uuid-1", 1500000, "PENDING", "2026-07-15T10:30:00Z"],
        ["uuid-2", 2000000, "PENDING", "2026-07-16T14:20:00Z"]
    ],
    "rows_returned": 2,
    "execution_time_ms": 45,
    "truncated": false
}
```

---

## 8. Security Best Practices

### 8.1 Core Principles

- **SELECT ONLY**: Chỉ cho phép Statement::Query, reject mọi DML/DDL
- **Immutable after approve**: sql_query không thể sửa sau APPROVED
- **No param modification**: User run nguyên SQL, không inject thêm điều kiện
- **Audit everything**: Mỗi lần execute → log đầy đủ (who, when, rows, IP)
- **Time-bound**: Khuyến khích đặt expires_at cho mỗi request
- **Ticket mandatory**: Không có ServiceDesk ticket = không submit được

### 8.2 SQL Injection Prevention

```rust
// ✅ ĐÚNG: Parse SQL bằng AST, validate toàn bộ structure
let validation = validate_data_query(&user_sql)?;
// SQL đã được validate → store nguyên bản, execute nguyên bản

// ❌ SAI: Dùng regex hoặc string matching
let safe = !sql.contains("DROP"); // KHÔNG an toàn
```

### 8.3 Subquery & JOIN Protection

Sử dụng `visit_relations()` từ sqlparser-rs để extract TẤT CẢ tables:
- FROM clause tables
- JOIN tables  
- Subqueries trong WHERE
- CTEs (WITH clauses)
- UNION/INTERSECT components

Mỗi table đều phải pass approval matrix → query access nhiều sensitive tables sẽ cần nhiều cấp approve hơn.

### 8.4 Row Limit

- Hard limit: 10,000 rows (configurable)
- Nếu user không specify LIMIT → hệ thống tự thêm
- Nếu user specify LIMIT > max → giảm xuống max
- Response ghi rõ `truncated: true` nếu bị cắt

---

## 9. Tích hợp hệ thống hiện tại

### 9.1 Executor — thêm read-only method

```rust
pub trait DatabaseQueryExecutor: Send + Sync {
    /// Execute read-only query, trả về dataset
    fn execute_query(
        &self,
        sql: &str,
        credentials: &DbCredentials,
        host: &str,
        port: i32,
        database_name: &str,
        schema_name: Option<&str>,
        timeout: Duration,
        max_rows: i64,
    ) -> impl Future<Output = Result<QueryResult, ExecutorError>> + Send;
}

pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub rows_returned: i64,
    pub truncated: bool,
}
```

### 9.2 PostgreSQL implementation sketch

```rust
impl DatabaseQueryExecutor for PgExecutor {
    async fn execute_query(/* params */) -> Result<QueryResult, ExecutorError> {
        // 1. Connect với sslmode=require
        // 2. SET statement_timeout = timeout_ms
        // 3. SET default_transaction_read_only = on  ← QUAN TRỌNG
        // 4. Execute query
        // 5. Fetch up to max_rows + 1 (để detect truncation)
        // 6. Return QueryResult
    }
}
```

**Key**: Set `default_transaction_read_only = on` ở connection level để đảm bảo
không thể execute write operations ngay cả khi SQL validation bị bypass.

### 9.3 Notification Integration

Reuse WebSocket notification hiện tại:
- Notify approvers (by unit) khi có request pending ở level của họ
- Notify Maker khi request được approve/reject
- Notify assigned users khi query trở thành executable
- Notify Maker + assigned users khi query sắp hết hạn (7 ngày trước)

---

## 10. Dependencies

| Crate | Mục đích | Version |
|-------|----------|---------|
| `sqlparser` | Parse SQL, extract tables/columns, validate read-only | 0.62+ |
| `sqlx` | Database queries, migrations | (hiện tại) |
| `axum` | HTTP framework, middleware, extractors | (hiện tại) |
| `serde` / `serde_json` | Serialization, query results | (hiện tại) |
| `chrono` | Timestamps, expiration | (hiện tại) |
| `uuid` | IDs | (hiện tại) |

---

## 11. Implementation Checklist

### Phase 1: Foundation
- [ ] Migration: `data_query_requests` table
- [ ] Migration: `data_query_assigned_users` table
- [ ] Migration: `data_query_approval_rules` + `data_query_approval_levels` tables
- [ ] Migration: `data_query_approval_tracking` table
- [ ] Migration: `data_query_executions` table
- [ ] Model: `DataQueryRequest`, `QueryRequestStatus`, `QueryMode` structs
- [ ] Model: `ApprovalRule`, `ApprovalLevel`, `ApproverUnit` structs
- [ ] Model: `ApprovalTracking`, `DataQueryExecution` structs
- [ ] Crate dependency: `sqlparser = "0.62"` with features = ["visitor"]

### Phase 2: SQL Validation
- [ ] Module `query_validator.rs`: parse, read-only check, extract tables/columns
- [ ] Row LIMIT enforcement
- [ ] SELECT * detection (reject in Basic mode, warn in Advanced)

### Phase 3: Approval Matrix
- [ ] DB Layer: CRUD cho approval rules (DBA)
- [ ] DB Layer: Rule resolution logic (most specific wins)
- [ ] DB Layer: Multi-table rule merging
- [ ] DB Layer: Approval tracking (create, approve, reject)
- [ ] API: Approval rule management endpoints (DBA)
- [ ] API: Pending approvals listing (by approver_unit)
- [ ] API: Approve/Reject endpoints

### Phase 4: Query Requests
- [ ] DB Layer: CRUD cho data_query_requests
- [ ] DB Layer: Assigned users management
- [ ] API: Create request (Basic + Advanced mode)
- [ ] API: Submit for approval (trigger approval chain)
- [ ] API: List/detail requests
- [ ] ServiceDesk ticket validation

### Phase 5: Execution
- [ ] Executor: `DatabaseQueryExecutor` trait + PgExecutor impl
- [ ] Executor: MySqlExecutor, OracleExecutor, MongoExecutor impl
- [ ] API: Execute query endpoint (with authorization check)
- [ ] Audit: Log mỗi execution

### Phase 6: Frontend
- [ ] Page: Data Query Builder (Basic mode UI)
- [ ] Page: Advanced SQL Editor (Advanced mode)
- [ ] Page: Approval Queue (per approver unit)
- [ ] Page: My Queries (maker + assigned)
- [ ] Page: Query Execution + Results viewer
- [ ] Page: Approval Matrix Config (DBA)
- [ ] Page: Execution Audit Log
- [ ] Component: Query Result Table (paginated, exportable)
- [ ] WebSocket: Notifications cho approval flow

---

## Nguồn tham khảo

- [PostgreSQL GRANT/REVOKE Column Privileges](https://www.postgresql.org/docs/current/ddl-priv.html)
- [PostgreSQL Row Level Security](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [PostgreSQL CREATE POLICY](https://www.postgresql.org/docs/current/sql-createpolicy.html)
- [Axum Middleware from_extractor](https://docs.rs/axum/latest/axum/middleware/fn.from_extractor.html)
- [Axum route_layer](https://docs.rs/axum/latest/axum/routing/struct.Router.html)
- [SQLx QueryBuilder](https://docs.rs/sqlx/latest/sqlx/struct.QueryBuilder.html)
- [SQLx Parameterized Queries](https://docs.rs/sqlx/latest/sqlx/fn.query.html)
- [sqlparser-rs Visitor Pattern](https://github.com/apache/datafusion-sqlparser-rs)

*Content was rephrased for compliance with licensing restrictions.*
