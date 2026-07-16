# Nguyên tắc phát triển — SECS

Áp dụng cho **mọi** tác vụ trong dự án này, bất kể quy mô lớn hay nhỏ.
Kiro phải tuân thủ những nguyên tắc này trong suốt vòng đời phát triển.

---

## 1. Trước khi bắt đầu bất kỳ việc gì

### 1.1 Làm rõ yêu cầu trước khi làm

**Không bắt đầu implement khi yêu cầu còn mơ hồ.**

Nếu một yêu cầu có bất kỳ điểm nào chưa rõ — dù nhỏ — phải hỏi lại trước.
Không được tự suy diễn, giả định, hoặc "làm trước hỏi sau".

Các dấu hiệu yêu cầu chưa rõ:
- Thiếu ngữ cảnh: "thêm tính năng filter" — filter theo gì? ở trang nào? cho role nào?
- Mâu thuẫn với tài liệu hiện có: yêu cầu mới xung đột với business rule đã định nghĩa
- Phạm vi không rõ: "cải thiện performance" — endpoint nào? metric nào?
- Điều kiện biên chưa được nêu: "nếu approve thất bại thì sao?"

**Cách hỏi lại**: Nêu cụ thể phần nào chưa hiểu, đưa ra assumption đang có,
hỏi để confirm — không hỏi chung chung.

> Ví dụ tốt: "Tôi hiểu yêu cầu là filter danh sách request theo database.
> Có 2 điểm cần làm rõ: (1) Filter này áp dụng cho cả Maker và Checker không,
> hay chỉ một role? (2) Nếu user không chọn filter thì mặc định hiển thị
> tất cả hay chỉ DB mình có quyền?"

### 1.2 Nghiên cứu tài liệu trước khi implement

Trước khi viết bất kỳ dòng code nào cho một tính năng mới hoặc chỉnh sửa
tính năng cũ, phải đọc lại:

1. **`domain.md`** — Business rules liên quan đến tính năng đó
2. **`system-design.md`** — Functional/NFR requirements, API design, UX design
3. **`implementation-references.md`** — Code patterns và best practices cho thư viện liên quan
4. **`structure.md`** — Vị trí đặt code mới trong project structure
5. **`tech.md`** — Constraints về tech stack, security rules

Mục tiêu: hiểu đủ context trước khi quyết định approach. Đừng code rồi mới
phát hiện vi phạm business rule hay sai kiến trúc.

---

## 2. Nguyên tắc thiết kế code

### 2.1 SOLID Principles

**S — Single Responsibility Principle**
Mỗi module/struct/function chỉ làm một việc và chỉ có một lý do để thay đổi.

```rust
// ❌ Sai: handler làm cả business logic + DB + notification
async fn approve_handler(/* ... */) {
    // validate 4-eyes
    // update DB
    // send email
    // push WS notification
    // log audit
}

// ✅ Đúng: handler mỏng, delegate cho các layer chuyên biệt
async fn approve_handler(/* ... */) -> Result<impl IntoResponse> {
    db::requests::approve_request(&state.db, &state.notification_tx, id, user_id).await?;
    Ok(StatusCode::OK)
}
```

**O — Open/Closed Principle**
Code mở để mở rộng, đóng để sửa đổi. Dùng trait thay vì if/match cứng nhắc.

```rust
// ✅ DatabaseExecutor trait cho phép thêm DB mới mà không sửa code hiện có
pub trait DatabaseExecutor: Send + Sync {
    async fn execute(&self, sql: &str, timeout_secs: u64) -> Result<ExecutionResult, ExecutorError>;
}
// Thêm OracleExecutor mới → implement trait, không sửa code hiện tại
```

**L — Liskov Substitution Principle**
Mọi impl của `DatabaseExecutor` phải hoạt động đúng khi được dùng thay thế
cho nhau — không có impl nào throw lỗi với input hợp lệ mà impl khác chấp nhận.

**I — Interface Segregation Principle**
Trait nhỏ và focused hơn là trait lớn chứa nhiều method không liên quan.

```rust
// ✅ Tách thành 2 trait rõ ràng
pub trait DatabaseExecutor { async fn execute(/*..*/); }
pub trait DatabaseProber   { async fn test_connection(/*..*/); }
```

**D — Dependency Inversion Principle**
Handler và service layer phụ thuộc vào abstraction (trait), không vào
concrete implementation.

```rust
// ✅ Handler nhận &dyn DatabaseExecutor, không phải PostgresExecutor cụ thể
async fn run_sql(executor: &dyn DatabaseExecutor, sql: &str) { /* ... */ }
```

### 2.2 DRY — Don't Repeat Yourself

Mọi đoạn logic xuất hiện hơn 1 lần phải được extract thành hàm/module tái sử dụng.

```rust
// ❌ Sai: check permission lặp lại ở nhiều handler
async fn approve_handler(/* .. */) {
    let perm = sqlx::query!("SELECT ... FROM user_db_permissions WHERE ...").fetch_optional(&pool).await?;
    if perm.is_none() { return Err(AppError::Forbidden); }
    // ...
}
async fn submit_handler(/* .. */) {
    let perm = sqlx::query!("SELECT ... FROM user_db_permissions WHERE ...").fetch_optional(&pool).await?;
    if perm.is_none() { return Err(AppError::Forbidden); }
    // ...
}

// ✅ Đúng: extract thành hàm trong db/users.rs
pub async fn check_db_permission(pool: &PgPool, user_id: Uuid, db_id: Uuid) -> Result<()> {
    // logic một chỗ duy nhất
}
```

**Áp dụng với frontend:**
```javascript
// ✅ apiFetch trong api/index.js — không gọi fetch trực tiếp trong component
// ✅ formatDateTime trong utils.js — không format date ở nhiều chỗ
// ✅ StatusBadge.svelte — không duplicate badge logic
```

### 2.3 KISS — Keep It Simple, Stupid

Luôn chọn giải pháp đơn giản nhất đáp ứng yêu cầu.
Không over-engineer, không thêm abstraction khi chưa cần.

- Không dùng macro phức tạp khi function đơn giản là đủ
- Không tạo trait khi chỉ có 1 implementation
- Không dùng generic khi concrete type là đủ

### 2.4 YAGNI — You Aren't Gonna Need It

Không implement tính năng vì "có thể sẽ cần sau này".
Chỉ implement những gì yêu cầu hiện tại đòi hỏi.

```rust
// ❌ Sai: thêm "để sau dùng" mà chưa có yêu cầu
pub struct SqlRequest {
    pub priority: Option<i32>,          // "biết đâu sau cần"
    pub tags: Vec<String>,              // "để filter sau"
    pub notify_email: Option<String>,   // "phase 2 mà"
}

// ✅ Đúng: chỉ có những field đang cần
pub struct SqlRequest {
    pub id: Uuid,
    pub sql_content: String,
    pub description: String,
    pub status: RequestStatus,
    pub database_id: Uuid,
    pub created_by: Uuid,
}
```

### 2.5 Separation of Concerns

Phân tách rõ ràng giữa các layer:

| Layer | Trách nhiệm | Không được làm |
|-------|-------------|----------------|
| `api/` handlers | Parse request, gọi db layer, trả response | Chứa business logic, gọi executor trực tiếp |
| `db/` | Query PostgreSQL nội bộ, enforce business rules | Gọi executor, biết về HTTP |
| `executor/` | Thực thi SQL trên database đích | Query PostgreSQL nội bộ, biết về HTTP |
| `models/` | Data structures, state machine | I/O, side effects |
| `auth/` | JWT, OIDC logic | DB queries, business rules |

---

## 3. Nguyên tắc viết code

### 3.1 Rust

- **Không `unwrap()` trong production code** — dùng `?` hoặc xử lý explicit
- **Không `clone()` không cần thiết** — hiểu ownership trước khi clone
- **Error handling rõ ràng**: `thiserror` cho domain errors, `anyhow` cho application errors
- **`cargo clippy -- -D warnings` phải pass** trước khi commit — không suppress warning
- **`cargo fmt`** trước mỗi commit
- **Test public API, không test internals** — test behavior, không test implementation

```rust
// ❌ Sai
let user = get_user(id).unwrap();

// ✅ Đúng
let user = get_user(id).await?.ok_or(AppError::NotFound)?;
```

### 3.2 Svelte / Frontend

- **Không gọi API trực tiếp trong component** — phải qua `src/api/`
- **Không chứa logic phân quyền trong frontend** — chỉ hiển thị theo data từ API
- **Mỗi component trong file `.svelte` riêng** — không inline component
- **Prettier trước commit** — `pnpm lint`
- **Không hardcode URL, string** — dùng constants hoặc env vars

### 3.3 SQL

- **Dùng `query!` / `query_as!` macro** — compile-time type checking, không dùng dynamic SQL
- **Parameterized queries** — không bao giờ string-concat SQL với user input
- **Không SELECT * trong production** — explicit column names để tránh breaking change
- **Transaction cho mọi thao tác multi-step** — all-or-nothing

---

## 4. Quy trình phát triển (SDLC)

### 4.1 Trước khi code

```
1. Đọc lại requirements liên quan trong steering docs
2. Làm rõ ambiguities — hỏi nếu chưa chắc
3. Thiết kế approach: data model thay đổi không? API mới không? UI thay đổi gì?
4. Xác định test cases cần viết
5. Ước tính phạm vi: có vi phạm architecture không?
```

### 4.2 Trong khi code

```
1. Implement theo thứ tự: model → db layer → executor (nếu cần) → api handler → frontend
2. Viết test song song, không để sau
3. Commit nhỏ, thường xuyên — mỗi commit là một unit of work hoàn chỉnh
4. Commit message rõ ràng: "feat: add cancel request endpoint" không phải "update code"
```

### 4.3 Trước khi hoàn thành

```
□ cargo clippy -- -D warnings  → 0 warnings
□ cargo fmt                    → không có diff
□ cargo test                   → tất cả pass
□ pnpm lint                    → frontend clean
□ Tự review lại code: có DRY violation không? có business rule nào bị bỏ sót không?
□ Edge cases: empty list, null values, permission denied, timeout — đã xử lý chưa?
□ Không có secret/credential trong code
```

### 4.4 Commit conventions

```
feat:     tính năng mới
fix:      sửa bug
refactor: cải thiện code không thay đổi behavior
test:     thêm/sửa test
docs:     cập nhật documentation
chore:    build, deps, config

Ví dụ:
feat: add approve request endpoint with 4-eyes enforcement
fix: prevent checker from seeing sql_content in list response
refactor: extract check_db_permission into shared db/users.rs
```

---

## 5. Nguyên tắc bảo mật (Security by Design)

- **Fail secure**: Khi có lỗi hoặc thiếu thông tin → từ chối, không cho phép
- **Least privilege**: App DB user chỉ có quyền tối thiểu cần thiết
- **Defense in depth**: Enforce business rules ở cả application layer VÀ DB constraint
- **Không trust input**: Validate và sanitize mọi input từ user, kể cả JWT claims
- **Audit everything**: Mọi thao tác thay đổi state phải có audit log
- **Secrets management**: Không hardcode, không log, không trả về trong API response

---

## 6. Nguyên tắc xử lý thay đổi

### Khi có yêu cầu thay đổi tính năng hiện có:
1. Xác định xem thay đổi có vi phạm business rules nào trong `domain.md` không
2. Xác định ảnh hưởng: DB schema thay đổi không? API contract thay đổi không? Frontend thay đổi gì?
3. Nếu thay đổi API contract → cần versioning (`/api/v2/`) để backward compat
4. Nếu thay đổi DB schema → viết migration mới, không sửa migration cũ

### Khi có conflict giữa yêu cầu mới và tài liệu hiện có:
- Báo conflict rõ ràng, không tự quyết định
- Ví dụ: "Yêu cầu mới cho phép Maker xem SQL của người khác, nhưng `domain.md` nêu
  Maker chỉ xem request của mình. Cần làm rõ trước khi implement."
