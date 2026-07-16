---
inclusion: auto
description: "SQLx — Reference cho Database Layer."
---

# SQLx — Reference cho Database Layer

Nguồn: https://docs.rs/sqlx/latest/sqlx/

## Dependency (Cargo.toml)

```toml
[dependencies]
sqlx = { version = "0.8", features = [
    "postgres",
    "mysql",
    "runtime-tokio-rustls",
    "migrate",
    "chrono",
    "uuid",
] }
```

---

## 1. Khởi tạo Connection Pool

```rust
use sqlx::PgPool;

// Trong main.rs hoặc config.rs
let pool = PgPool::connect(&database_url).await?;

// Với cấu hình chi tiết
use sqlx::postgres::PgPoolOptions;
let pool = PgPoolOptions::new()
    .max_connections(10)
    .min_connections(2)
    .acquire_timeout(std::time::Duration::from_secs(5))
    .connect(&database_url)
    .await?;
```

---

## 2. Query thông thường (runtime-checked)

```rust
use sqlx::PgPool;

// Execute (INSERT/UPDATE/DELETE — không cần rows trả về)
let result = sqlx::query("DELETE FROM sql_requests WHERE id = $1")
    .bind(request_id)
    .execute(&pool)
    .await?;
println!("Rows affected: {}", result.rows_affected());

// Fetch one
let row = sqlx::query("SELECT id, title FROM sql_requests WHERE id = $1")
    .bind(id)
    .fetch_one(&pool)
    .await?;

// Fetch optional
let maybe = sqlx::query("SELECT * FROM users WHERE email = $1")
    .bind(email)
    .fetch_optional(&pool)
    .await?;

// Fetch all
let rows = sqlx::query("SELECT * FROM sql_requests WHERE maker_id = $1")
    .bind(user_id)
    .fetch_all(&pool)
    .await?;
```

---

## 3. Mapping vào Struct với `FromRow`

```rust
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct SqlRequest {
    pub id: Uuid,
    pub title: String,
    pub sql_content: String,
    pub status: String,
    pub maker_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// Query trả về struct
let request: SqlRequest = sqlx::query_as(
    "SELECT id, title, sql_content, status, maker_id, created_at
     FROM sql_requests WHERE id = $1"
)
.bind(id)
.fetch_one(&pool)
.await?;

// Query trả về Vec<struct>
let requests: Vec<SqlRequest> = sqlx::query_as(
    "SELECT * FROM sql_requests WHERE maker_id = $1 ORDER BY created_at DESC"
)
.bind(user_id)
.fetch_all(&pool)
.await?;

// Flatten nested struct
#[derive(FromRow)]
struct UserWithRole {
    id: Uuid,
    username: String,
    #[sqlx(flatten)]
    role_info: RoleInfo,
}

// Skip field không có trong query
#[derive(FromRow)]
struct UserSummary {
    id: Uuid,
    username: String,
    #[sqlx(skip)]
    permissions: Vec<String>, // sẽ dùng Default (empty vec)
}
```

---

## 4. Compile-time Checked Query với `query!` macro

> **Yêu cầu**: Phải có `DATABASE_URL` trong `.env` khi compile

```rust
// query! macro — type-safe tại compile time
let account = sqlx::query!(
    "SELECT id, username, email FROM users WHERE id = $1",
    user_id
)
.fetch_one(&pool)
.await?;

println!("{}: {}", account.id, account.username);

// query_as! — map vào struct với type checking
let request = sqlx::query_as!(
    SqlRequest,
    "SELECT * FROM sql_requests WHERE id = $1",
    id
)
.fetch_optional(&pool)
.await?;
```

---

## 5. Transaction

```rust
use sqlx::PgPool;

async fn approve_and_execute(pool: &PgPool, request_id: Uuid, checker_id: Uuid) -> anyhow::Result<()> {
    // Bắt đầu transaction
    let mut tx = pool.begin().await?;

    // Cập nhật status request
    sqlx::query(
        "UPDATE sql_requests SET status = 'APPROVED', checker_id = $1, approved_at = NOW()
         WHERE id = $2"
    )
    .bind(checker_id)
    .bind(request_id)
    .execute(&mut *tx)
    .await?;

    // Ghi audit log
    sqlx::query(
        "INSERT INTO audit_log (request_id, action, actor_id, timestamp)
         VALUES ($1, 'APPROVED', $2, NOW())"
    )
    .bind(request_id)
    .bind(checker_id)
    .execute(&mut *tx)
    .await?;

    // Commit — rollback tự động nếu không gọi commit() trước khi drop
    tx.commit().await?;
    Ok(())
}
```

**Lưu ý**: Nếu `tx` bị drop mà không gọi `commit()`, SQLx tự động `ROLLBACK`.

---

## 6. Migration

```bash
# Cài sqlx-cli
cargo install sqlx-cli --features postgres

# Tạo migration mới
sqlx migrate add create_users_table
sqlx migrate add create_sql_requests_table

# Chạy migration
sqlx migrate run

# Revert migration gần nhất
sqlx migrate revert
```

Cấu trúc file migration:
```
backend/migrations/
├── 20240101000001_create_users.sql
├── 20240101000002_create_database_connections.sql
├── 20240101000003_create_sql_requests.sql
└── 20240101000004_create_audit_log.sql
```

Chạy migration khi start app:
```rust
// Trong main.rs
sqlx::migrate!("./migrations")
    .run(&pool)
    .await
    .expect("Failed to run database migrations");
```

---

## 7. Pattern cho db/ layer trong dự án SECS

```rust
// backend/src/db/requests.rs

use sqlx::PgPool;
use uuid::Uuid;
use crate::models::request::{SqlRequest, RequestStatus};
use crate::error::AppError;

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<SqlRequest>, AppError> {
    let request = sqlx::query_as!(
        SqlRequest,
        "SELECT * FROM sql_requests WHERE id = $1",
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(request)
}

pub async fn list_by_maker(pool: &PgPool, maker_id: Uuid) -> Result<Vec<SqlRequest>, AppError> {
    let requests = sqlx::query_as!(
        SqlRequest,
        "SELECT * FROM sql_requests WHERE maker_id = $1 ORDER BY created_at DESC",
        maker_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(requests)
}

pub async fn update_status(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: Uuid,
    status: RequestStatus,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE sql_requests SET status = $1, updated_at = NOW() WHERE id = $2",
        status as RequestStatus,
        id
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
```

---

## Quy tắc áp dụng trong dự án SECS

- `db/` chỉ dùng `PgPool` cho PostgreSQL nội bộ — không nhầm với `executor/` (database đích)
- Luôn dùng parameterized query (`.bind()`), không bao giờ interpolate string SQL
- Các thao tác liên quan đến state transitions (approve/reject/execute) phải wrap trong transaction
- Audit log phải ghi trong cùng transaction với action chính — đảm bảo tính nhất quán
- Dùng `query_as!` macro khi có thể để có compile-time type checking
- Không log SQL statement chứa dữ liệu nhạy cảm
