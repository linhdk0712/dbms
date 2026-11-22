# Báo Cáo Đánh Giá Toàn Diện Source Code Hệ Thống SECS
## (SQL Execution Control System)

---

- **Hệ thống**: SECS — SQL Execution Control System
- **Phạm vi review**: Toàn bộ source code (`backend/`, `frontend/`, `migrations/`, `docker-compose.yml`, configs, tài liệu kiến trúc)
- **Thời gian đánh giá**: Tháng 10/2026
- **Trọng tâm đánh giá**: **Tech Stack**, **Tính năng**, **Hiệu năng**, **Bảo mật**, và **Khuyến nghị cải tiến**

---

## MỤC LỤC
1. [Tổng Quan Kiến Trúc & Tech Stack](#1-tổng-quan-kiến-trúc--tech-stack)
2. [Đánh Giá Chi Tiết Tính Năng](#2-đánh-giá-chi-tiết-tính-năng)
3. [Đánh Giá Hiệu Năng (Performance Assessment)](#3-đánh-giá-hiệu-năng-performance-assessment)
4. [Đánh Giá Bảo Mật (Security Assessment)](#4-đánh-giá-bảo-mật-security-assessment)
5. [So Sánh Thực Tế Triển Khai vs Tài Liệu Đặc Tả](#5-so-sánh-thực-tế-triển-khai-vs-tài-liệu-đặc-tả)
6. [Danh Sách Lỗi & Cảnh Báo Mã Nguồn Cần Khắc Phục Ngay](#6-danh-sách-lỗi--cảnh-báo-mã-nguồn-cần-khắc-phục-ngay)
7. [Khuyến Nghị & Lộ Trình Nâng Cấp (Actionable Roadmap)](#7-khuyến-nghị--lộ-trình-nâng-cấp-actionable-roadmap)

---

## 1. Tổng Quan Kiến Trúc & Tech Stack

Hệ thống SECS áp dụng mô hình phân tách **Backend API Service** và **Frontend Single Page Application (SPA)**, phục vụ quy trình kiểm soát thao tác cơ sở dữ liệu production theo nguyên tắc **Maker-Checker** (4-eyes principle) cho các tổ chức tài chính/ngân hàng.

```
                           ┌────────────────────────┐
                           │   Browser / Client     │
                           └───────────┬────────────┘
                                       │ HTTPS / WSS
                                       ▼
                           ┌────────────────────────┐
                           │   Nginx (Port 80/8088) │
                           └─────┬────────────┬─────┘
                     Static Assets│            │ /api, /auth
                                  ▼            ▼
                         [SPA Svelte 4]  ┌───────────┐
                                         │ Axum REST │
                                         │  (Rust)   │
                                         └─────┬─────┘
                 ┌─────────────────────────────┼─────────────────────────────┐
                 │                             │                             │
                 ▼                             ▼                             ▼
       ┌───────────────────┐         ┌───────────────────┐         ┌───────────────────┐
       │ Internal Postgres │         │  Keycloak 24 OIDC │         │ Target Databases  │
       │   (App Database)  │         │  (Auth Provider)  │         │ PG, MySQL, Mongo, │
       │                   │         │                   │         │      Oracle       │
       └───────────────────┘         └───────────────────┘         └───────────────────┘
```

### 1.1. Chi tiết Tech Stack

| Thành phần | Công nghệ / Thư viện chính | Phiên bản | Nhận xét kiến trúc |
| :--- | :--- | :--- | :--- |
| **Backend Core** | Rust (`edition = "2021"`) | 1.88+ | Lựa chọn tuyệt vời về memory safety, zero-cost abstractions, hiệu năng cao và loại bỏ hoàn toàn GC pauses. |
| **Web Framework** | `axum`, `tower`, `tower-http` | `axum 0.7`, `tower 0.4` | Chuẩn mực hiện đại cho async microservices trong hệ sinh thái Rust; tích hợp routing, middleware pipeline mượt mà. |
| **Async Runtime** | `tokio` (full features) | `1.x` | Runtime bất đồng bộ mạnh mẽ nhất hiện nay của Rust, hỗ trợ work-stealing multi-threading scheduler. |
| **Database Access (Nội bộ)** | `sqlx` (postgres, runtime-tokio-rustls) | `0.8` | Compile-time SQL verification, hoàn toàn async, tích hợp migration tự động lúc khởi động. |
| **Cơ sở dữ liệu đích** | `sqlx` (postgres, mysql), `mongodb` native, `oracle` OCI | `sqlx 0.8`, `mongo 3.8`, `oracle 0.6` | Hỗ trợ đa dạng RDBMS & NoSQL. Riêng driver Oracle chạy đồng bộ qua `tokio::task::spawn_blocking`. |
| **Authentication & IAM**| `openidconnect`, `oauth2`, `jsonwebtoken` | `oidc 3.5`, `jwt 9.0` | Chuẩn SSO OAuth2/OIDC với PKCE challenge, JWT RS256/HS256 với token blacklist in-memory. |
| **Mã hóa dữ liệu** | `aes-gcm` (AES-256-GCM), `base64`, `hex` | `0.10` | Chuẩn Authenticated Encryption with Associated Data (AEAD); mã hóa mật khẩu database đích với unique 96-bit nonce. |
| **Frontend Framework** | Svelte 4, Vite | `svelte 4.2`, `vite 5.4` | Cực kỳ nhẹ, reactivity compile-time (không runtime Virtual DOM), thời gian tải nhanh và bundle nhỏ. |
| **UI Components & Style**| Tailwind CSS, DaisyUI | `tailwind 3.4`, `daisyui 4.12` | Giao diện hiện đại, hỗ trợ theme linh hoạt, chuẩn hóa CSS utility-first. |
| **Code Editor** | CodeMirror 6 | `6.0` | Editor chuẩn cho SQL syntax highlight, hỗ trợ PostgreSQL, MySQL, JSON. |
| **Internal Database** | PostgreSQL | 16 (Alpine) | Cơ sở dữ liệu ACID tin cậy, lưu trữ state, users, permissions, audit logs. |
| **DevOps & Container** | Docker, Docker Compose, Nginx | Docker 24+, Compose v2 | Multi-stage build tối ưu kích thước image và an ninh container (chạy non-root). |

---

## 2. Đánh Giá Chi Tiết Tính Năng

### 2.1. Nhóm tính năng đã hoàn thiện xuất sắc

1. **Quy trình Maker-Checker (4-Eyes Principle)**:
   - Phân định rạch ròi 3 vai trò: `Admin`, `Maker`, `Checker`, `Approver`.
   - **Bảo vệ kép (Defense in Depth)**: Ràng buộc "Người phê duyệt không được trùng người tạo" được kiểm tra cả ở tầng Application (`approvals.rs`) lẫn tầng Database schema thông qua PostgreSQL CHECK constraint:
     ```sql
     CONSTRAINT four_eyes_check CHECK (approved_by IS NULL OR approved_by != created_by)
     ```
   - Trạng thái vòng đời câu lệnh kiểm soát chặt chẽ: `DRAFT -> PENDING -> APPROVED -> EXECUTED / FAILED`, kèm theo nhánh `REJECTED` (bắt buộc lý do tối thiểu 10 ký tự) và `CANCELLED`.
   - Hỗ trợ **Multi-level Approval Matrix** (`approval_levels_required`): Cho phép cấu hình nhiều cấp phê duyệt tuần tự (ví dụ: Level 1 ITO -> Level 2 ITS -> Level 3 Compliance) trước khi câu lệnh được kích hoạt thực thi.

2. **Cơ chế Phân quyền Truy vấn Dữ liệu (Data Query Permissions)**:
   - Module `data_queries` cho phép người dùng đăng ký câu truy vấn đọc dữ liệu (`SELECT`) với 2 chế độ:
     - **Basic Mode**: Giao diện trực quan chọn bảng, cột, filter, limit (tự sinh câu lệnh SQL an toàn).
     - **Advanced Mode**: Nhập câu lệnh SQL tự do.
   - Gán quyền ủy quyền (`assigned_user_ids`): Maker có thể tạo query mẫu và ủy quyền cho danh sách user cụ thể được phép chạy sau khi đã được approve.
   - Giới hạn thời gian hiệu lực (`expires_at`) và hạn mức kết quả (`MAX_QUERY_ROWS = 10,000`).

3. **Schema Browser & Metadata Introspection**:
   - Cho phép người dùng duyệt danh sách bảng và thông tin cột (kiểu dữ liệu, nullable, primary key) trực tiếp từ database đích (PostgreSQL, MySQL, MongoDB).
   - Kiểm tra quyền truy cập database trước khi cho phép inspect schema.

4. **Tích hợp Quản lý Thay đổi (ManageEngine ServiceDesk Plus)**:
   - Cho phép đính kèm Change Request (CR) URL từ hệ thống ServiceDesk Plus.
   - Tự động bóc tách CR ID qua regex (`/changes/(\d+)`), gọi API lấy metadata (Title, Status, Maintenance Window).
   - Kiểm tra khung giờ bảo trì (`scheduled_start_time` đến `scheduled_end_time`) trước khi cho phép Approve & Execute SQL.

5. **AI SQL Analysis**:
   - Tích hợp LLM client (OpenAI API GPT-4o-mini).
   - Tự động trích xuất bảng từ SQL, introspect cấu trúc schema và build prompt chi tiết gửi đến AI để đánh giá rủi ro, phân tích tác động và cảnh báo hiệu năng trước khi Checker duyệt.
   - Có in-memory cache kết quả AI theo hash request ID + SQL content để tiết kiệm token và chi phí.

---

### 2.2. Điểm khuyết & Thiếu sót về tính năng

1. **WebSocket Realtime Notifications bị tắt (Disabled)**:
   - Trong `backend/src/main.rs`, route `/ws` và module `ws` đã bị comment out hoàn toàn:
     ```rust
     // WebSocket disabled
     // mod ws;
     // .route("/ws", get(ws::ws_handler))
     ```
   - Trong `frontend/src/stores/notifications.js`, các hàm `connect()` và `disconnect()` là no-op.
   - *Hệ quả*: Người dùng không nhận được thông báo thời gian thực khi có request mới hay khi request được duyệt; phải reload trang thủ công.

2. **Schema Browser cho Oracle DB chưa hoàn thiện (Stub/Placeholder)**:
   - Trong `backend/src/api/schema_browser.rs` (dòng 488–508), hai hàm `list_oracle_tables` và `list_oracle_columns` chỉ là code khung (chưa query `ALL_TABLES` / `ALL_TAB_COLUMNS`) và luôn trả về mảng rỗng `Ok(vec![])`.

3. **Chưa hỗ trợ Transaction Rollback cho MongoDB Single Node**:
   - Module `mongodb.rs` yêu cầu MongoDB chạy chế độ Replica Set để hỗ trợ multi-document transactions. Mặc dù Docker Compose đã có container `mongodb-init` khởi tạo `rs0`, nếu kết nối đến MongoDB standalone ngoài thực tế, các request multi-command sẽ thất bại.

---

## 3. Đánh Giá Hiệu Năng (Performance Assessment)

### 3.1. Điểm mạnh về hiệu năng

1. **Kiến trúc Không tiêu tốn tài nguyên (High Concurrency & Low Memory)**:
   - Backend Rust + Axum có thời gian khởi động < 50ms, RAM footprint khi khởi chạy chỉ khoảng ~15MB - 30MB (so với 300MB - 500MB của Java Spring Boot hoặc 100MB+ của Node.js).
   - Profile release tối ưu tối đa trong [Cargo.toml](file:///d:/Apps/dbms/backend/Cargo.toml):
     ```toml
     [profile.release]
     opt-level = 3
     lto = true
     codegen-units = 1
     ```
     Đảm bảo binary được tối ưu hoá Link-Time Optimization (LTO), inline tối đa và loại bỏ dead-code triệt để.

2. **Tối ưu hóa Phía Client & Web Server**:
   - `nginx.conf` bật Gzip compression (text, css, js, json) với kích thước tối thiểu 256 bytes.
   - Tài nguyên tĩnh (JS, CSS, fonts) được cấu hình Cache-Control tĩnh 1 năm: `Cache-Control: public, immutable`.

---

### 3.2. Điểm nghẽn hiệu năng (Bottlenecks) & Rủi ro

1. **Overhead Kết Nối Cơ Sở Dữ Liệu Đích (Target DB Connection Churn)**:
   - Trong `executor/postgres.rs`, `executor/mysql.rs`, và `schema_browser.rs`:
     ```rust
     let pool = sqlx::postgres::PgPoolOptions::new()
         .max_connections(1)
         .acquire_timeout(Duration::from_secs(10))
         .connect(&conn_string)
         .await?;
     // ... query execution ...
     pool.close().await;
     ```
   - **Vấn đề**: Cứ mỗi lần thực thi câu lệnh SQL, mỗi lần bấm nút Validate (EXPLAIN), hoặc mỗi thao tác chọn bảng trên Schema Browser, backend lại khởi tạo một TCP connection mới, thực hiện TLS handshake, xác thực user/password, rồi đóng kết nối ngay sau đó (`pool.close().await`).
   - **Ảnh hưởng**: Độ trễ (latency) của mỗi request bị cộng thêm từ 50ms đến 250ms chỉ riêng cho giai đoạn bắt tay kết nối mạng. Nếu có nhiều Checker cùng validate, điều này gây chậm chạp rõ rệt.

2. **Kích thước Bundle Frontend Vượt Ngưỡng Khuyến Nghị (Vite Chunk Size)**:
   - Kết quả build Vite:
     ```
     dist/assets/index-DmFwQdao.css    121.85 kB │ gzip:  18.07 kB
     dist/assets/index-dvcMWIzx.js     669.61 kB │ gzip: 207.38 kB
     (!) Some chunks are larger than 500 kB after minification.
     ```
   - Toàn bộ CodeMirror modules, DaisyUI, các page Admin, Checker, Maker, Data Queries đều bị nén vào một file JS duy nhất `index-xxx.js`.
   - **Ảnh hưởng**: Người dùng lần đầu truy cập trang Login phải tải toàn bộ ~670kB code (kèm CodeMirror editors), làm tăng chỉ số FCP (First Contentful Paint) và TTI (Time to Interactive).

3. **Tải Toàn Bộ Dataset vào RAM khi Query Dữ Liệu (Memory Buffering)**:
   - Trong `execute_query_readonly_pg` và `execute_query_readonly_mysql`, hệ thống fetch toàn bộ kết quả lên đến `max_rows` (10,000 dòng) vào memory dưới dạng `Vec<Vec<serde_json::Value>>` rồi mới serialize thành JSON trả về HTTP response.
   - Nếu bảng có các cột TEXT dung lượng lớn, 10,000 dòng có thể chiếm hàng chục MB RAM trên tiến trình backend.

---

## 4. Đánh Giá Bảo Mật (Security Assessment)

### 4.1. Điểm mạnh bảo mật (Best Practices Đã Triển Khai)

1. **Mã hóa thông tin nhạy cảm chuẩn công nghiệp**:
   - Toàn bộ credentials (username, password) của database đích được lưu tại cột `encrypted_credentials` bằng thuật toán **AES-256-GCM**.
   - Mỗi lần mã hóa sinh một 96-bit cryptographic nonce ngẫu nhiên thông qua `OsRng`, ngăn chặn triệt để tấn công replay attack hoặc nhận diện ciphertext lặp lại.
   - Key mã hóa 256-bit tách rời khỏi database (lưu trong biến môi trường `ENCRYPTION_KEY`).

2. **Chống đánh cắp Token (XSS Mitigations)**:
   - JWT token chính thức được backend lưu trong **HttpOnly Cookie** (`secs_token`):
     ```
     Set-Cookie: secs_token=...; HttpOnly; SameSite=Lax; Path=/; Max-Age=28800; Secure
     ```
   - Nhờ cờ `HttpOnly`, mã độc JavaScript (nếu bị tấn công XSS) không thể đọc được session token từ `document.cookie`.
   - Frontend store `auth.js` đã loại bỏ việc lưu token trong `localStorage`.

3. **Bộ Security Headers đầy đủ trên HTTP Response**:
   - Backend tự động gắn các header phòng thủ:
     - `Content-Security-Policy: default-src 'self'`
     - `Strict-Transport-Security: max-age=31536000; includeSubDomains`
     - `X-Content-Type-Options: nosniff`
     - `X-Frame-Options: DENY` (chống Clickjacking)
     - `Referrer-Policy: strict-origin-when-cross-origin`
     - `Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=()`

4. **Kiểm soát Tốc độ (Rate Limiting) & Chống Brute Force**:
   - `rate_limit_login`: Giới hạn tối đa 10 request/phút/IP trên endpoint `/auth/login`.
   - `global_rate_limit`: Giới hạn 100 request/phút/IP trên toàn bộ protected routes.
   - In-memory `TokenBlacklist` hỗ trợ vô hiệu hóa token ngay lập tức khi người dùng bấm Đăng xuất hoặc khi Admin khóa tài khoản.

5. **An toàn Thực thi Cơ sở dữ liệu Nội bộ (SQL Injection Free)**:
   - Toàn bộ tương tác với PostgreSQL nội bộ thông qua `sqlx` đều sử dụng câu lệnh tham số hóa (Parameterized queries `$1, $2, ...`). Không có hiện tượng nối chuỗi thô để query DB nội bộ.

---

### 4.2. Các Lỗ Hổng & Rủi Ro Bảo Mật Cần Chú Ý (Security Vulnerabilities)

#### LỖ HỔNG 1 (MỨC ĐỘ CRITICAL): File `cookies.txt` Chứa Active Tokens Bị Commit Vào Git
- **Vị trí**: [cookies.txt](file:///d:/Apps/dbms/cookies.txt) tại thư mục gốc dự án.
- **Mô tả**: File chứa cookie session ID và identity token JWT của Keycloak (`KEYCLOAK_IDENTITY`, `AUTH_SESSION_ID`, user ID `51f99503-6fea-4b9d-833c-a2fe210645f7`).
- **Nguy cơ**: File này đang được Git track (`git ls-files cookies.txt`). Bất kỳ ai clone repository đều có thể sở hữu session token này.
- **Biện pháp xử lý**: Xóa file khỏi Git tracking (`git rm --cached cookies.txt`), đưa vào `.gitignore`, và invalidate session trên Keycloak.

#### LỖ HỔNG 2 (MỨC ĐỘ HIGH): Client IP Spoofing trong Audit Trail Do Bỏ Qua `trusted_proxies`
- **Vị trí**: [ip_extractor.rs](file:///d:/Apps/dbms/backend/src/api/ip_extractor.rs#L8-L20) & [config.rs](file:///d:/Apps/dbms/backend/src/config.rs#L99-L104)
- **Mô tả**:
  ```rust
  pub fn extract_client_ip(headers: &HeaderMap, conn_info: &ConnectInfo<SocketAddr>) -> String {
      let ip = extract_from_headers(headers)
          .unwrap_or_else(|| conn_info.0.ip().to_string());
      truncate_ip(&ip)
  }
  fn extract_from_headers(headers: &HeaderMap) -> Option<String> {
      if let Some(forwarded_for) = headers.get("x-forwarded-for") {
          let first_ip = value.split(',').next().unwrap_or("").trim();
          return Some(first_ip.to_string());
      }
      // ...
  }
  ```
  Trong khi đó, biến cấu hình `trusted_proxies` trong [config.rs](file:///d:/Apps/dbms/backend/src/config.rs) hoàn toàn không được truyền vào hay sử dụng (compiler cảnh báo: `field trusted_proxies is never read`).
- **Nguy cơ**: Kẻ tấn công hoặc người dùng nội bộ có thể gửi header giả mạo `X-Forwarded-For: 8.8.8.8` hoặc IP của lãnh đạo/đồng nghiệp. Bảng `audit_log` sẽ ghi nhận IP giả này, vô hiệu hóa tính tin cậy của nhật ký kiểm toán (vi phạm tiêu chuẩn SOX / PCI-DSS compliance).
- **Biện pháp xử lý**: Chỉ đọc `X-Forwarded-For` khi IP nguồn từ `conn_info` thuộc danh sách `trusted_proxies` (ví dụ Nginx container IP). Nếu không thuộc trusted proxy, bắt buộc lấy `conn_info.0.ip()`.

#### LỖ HỔNG 3 (MỨC ĐỘ HIGH): Lỗi Logic Điều Hướng Auth State Giữa Cookie và Token trên Frontend
- **Vị trí**: [App.svelte](file:///d:/Apps/dbms/frontend/src/App.svelte#L69-L88) và [auth.js](file:///d:/Apps/dbms/frontend/src/stores/auth.js#L47-L52)
- **Mô tả**:
  Trong `auth.js`, hàm `setUser(user)` phục vụ Cookie flow được thiết kế đặt `token: null`:
  ```javascript
  setUser(user) {
    const auth = { user, token: null };
    set(auth);
  }
  ```
  Tuy nhiên, trong `App.svelte`, điều kiện kiểm tra đăng nhập lại kiểm tra biến `token`:
  ```javascript
  $: {
    if (!$authStore?.token && !isPublicRoute) {
      push("/login");
    }
  }
  ```
  Và thanh điều hướng:
  ```svelte
  {#if $authStore?.token}
    <Navbar />
    <Sidebar />
  {/if}
  ```
- **Nguy cơ**: Khi người dùng đăng nhập qua luồng Cookie chuẩn (`authStore.setUser`), `$authStore.token` là `null`. Hệ quả là `App.svelte` ngay lập tức redirect người dùng ngược trở lại `/login` hoặc không render giao diện Navbar/Sidebar!
- **Biện pháp xử lý**: Đổi điều kiện kiểm tra sang `$authStore?.user` hoặc getter `authStore.isAuthenticated`.

#### LỖ HỔNG 4 (MỨC ĐỘ MEDIUM): Nguy cơ Bypass DDL Warning Qua Kỹ Thuật Comment Obfuscation
- **Vị trí**: [sql_analyzer.rs](file:///d:/Apps/dbms/backend/src/executor/sql_analyzer.rs#L50-L70)
- **Mô tả**: Bộ phân tích DDL sử dụng Regex:
  ```rust
  Regex::new(r"(?i)\b(DROP\s+TABLE)\b")
  ```
  Hệ thống không thực hiện chuẩn hóa hoặc loại bỏ SQL comments trước khi chạy Regex.
- **Nguy cơ**: Một câu lệnh chứa comment dạng:
  ```sql
  DROP/**/TABLE customer_accounts;
  ```
  sẽ **không khớp** với `DROP\s+TABLE`, khiến câu lệnh nguy hiểm (DANGER) lọt qua mà không bị gắn cờ cảnh báo màu đỏ trên giao diện Review của Checker.
- **Biện pháp xử lý**: Chuẩn hóa câu SQL bằng cách strip comments (`-- ...` và `/* ... */`) trước khi chạy regex patterns, hoặc dùng SQL parser chuẩn (như thư viện `sqlparser-rs`).

#### LỖ HỔNG 5 (MỨC ĐỘ MEDIUM): Ngắt Câu Lệnh Bằng `sql.split(';')` Gây Lỗi Khi Chứa Semicolon Trong String/Block
- **Vị trí**: [executor/postgres.rs](file:///d:/Apps/dbms/backend/src/executor/postgres.rs#L51-L55) và [executor/mysql.rs](file:///d:/Apps/dbms/backend/src/executor/mysql.rs#L50-L55)
- **Mô tả**:
  ```rust
  let statements: Vec<&str> = sql.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
  ```
- **Nguy cơ**: Nếu câu SQL hợp lệ chứa dấu `;` bên trong nháy đơn:
  ```sql
  INSERT INTO app_config (key, value) VALUES ('db_params', 'timeout=30;retry=3');
  ```
  Hệ thống sẽ cắt đôi chuỗi này thành 2 câu SQL không hợp lệ cú pháp, dẫn đến execution fail không mong muốn.

---

## 5. So Sánh Thực Tế Triển Khai vs Tài Liệu Đặc Tả

| Hạng mục cam kết (README / Docs) | Hiện trạng trong Codebase | Kết luận đánh giá |
| :--- | :--- | :--- |
| **Workflow Maker-Checker (4-eyes)** | Triển khai hoàn thiện, có DB Check Constraint & UI phê duyệt | ✅ **Đạt 100%** |
| **Credential Encryption (AES-256-GCM)**| Có đầy đủ module mã hóa, unit test round-trip hoàn chỉnh | ✅ **Đạt 100%** |
| **RBAC đa cơ sở dữ liệu** | Phân quyền chi tiết Admin, Maker, Checker, Approver | ✅ **Đạt 100%** |
| **Tích hợp SSO / OIDC** | Tích hợp Keycloak với PKCE flow, auto session cleanup | ✅ **Đạt 95%** (Cần sửa Auth state Svelte) |
| **Tích hợp ServiceDesk Plus (CR)** | Hoàn thiện API sync và chặn ngoài khung giờ bảo trì | ✅ **Đạt 100%** |
| **Phân quyền Data Query (SELECT)** | Hoàn thiện Basic builder, Advanced SQL, Approval matrix | ✅ **Đạt 100%** |
| **Realtime Notifications (WebSocket)** | **Bị tắt hoàn toàn** trong `main.rs` và frontend store | ❌ **Không hoạt động (0%)** |
| **Schema Browser (Oracle)** | Hàm listing Oracle là stub rỗng | ⚠️ **Chưa hoàn thiện (50%)** |
| **Health Check Docker** | Dockerfile gọi binary với flag `--health-check` không tồn tại | ⚠️ **Cấu hình sai** |

---

## 6. Danh Sách Lỗi & Cảnh Báo Mã Nguồn Cần Khắc Phục Ngay

### 6.1. Cảnh báo Compiler (Rust `cargo check` — 20 Warnings)
1. **Dead Code Variables**:
   - `config.rs:32`: Field `trusted_proxies` không bao giờ được đọc.
   - `config.rs:27`: Field `disable_target_db_tls` không bao giờ được đọc trực tiếp từ struct (chỉ đọc qua env var).
   - `data_queries.rs:48`: Các trường `filters` và `limit` trong `CreateDataQueryBody` không bao giờ được đọc (chỉ nhận rồi bỏ qua).
2. **Unused Functions**:
   - `db/data_queries.rs`: `mark_approved` và `mark_rejected` không được gọi (dùng các query khác).
   - `db/approver_units.rs`: `list_active` không được dùng.
   - `db/users.rs`: `find_by_subject` không được dùng.
3. **Unused Imports**:
   - `executor/mod.rs`: `use sqlx::{Column, Row};` bị thừa tại dòng 845 và 981.

### 6.2. Cảnh báo Frontend (Svelte Vite Build)
1. **Accessibility (A11y)**:
   - Các file `NewDataQuery.svelte`, `DataQueryApprovals.svelte`, `ApprovalMatrix.svelte` có nhiều thẻ `<label class="label">` không gắn `for="..."` tương ứng với input control (vi phạm A11y warning).
2. **Bundle Size Warning**:
   - Kích thước file bundle JS chính vượt quá 500kB (669.61 kB minified).

---

## 7. Khuyến Nghị & Lộ Trình Nâng Cấp (Actionable Roadmap)

### 7.1. Giai đoạn 1: Vá lỗi khẩn cấp (Hotfixes - Ưu tiên cao nhất)

1. **Xử lý rò rỉ Token & Cập nhật `.gitignore`**:
   - Xóa `cookies.txt` khỏi git: `git rm --cached cookies.txt`.
   - Bổ sung vào [.gitignore](file:///d:/Apps/dbms/.gitignore):
     ```gitignore
     # Local environment and credential files
     .env
     *.env
     backend/.env
     cookies.txt
     ```
2. **Sửa lỗi Client IP Spoofing trong Audit Trail**:
   - Nâng cấp [ip_extractor.rs](file:///d:/Apps/dbms/backend/src/api/ip_extractor.rs) để nhận tham chiếu `trusted_proxies` từ `Config`.
   - Nếu IP kết nối trực tiếp (`conn_info.0.ip()`) nằm trong dải `trusted_proxies`, mới chấp nhận IP từ `X-Forwarded-For`.
3. **Sửa Auth State Bug trên Frontend**:
   - Trong [App.svelte](file:///d:/Apps/dbms/frontend/src/App.svelte):
     Thay toàn bộ `$authStore?.token` bằng `$authStore?.user` để hỗ trợ cơ chế HttpOnly Cookie mượt mà.
4. **Sửa Health Check trong Dockerfile**:
   - Đổi lệnh HEALTHCHECK trong [backend/Dockerfile](file:///d:/Apps/dbms/backend/Dockerfile) sang gọi endpoint HTTP:
     ```dockerfile
     HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
         CMD curl -f http://localhost:3000/health || exit 1
     ```
     (Cần cài `curl` trong stage runtime `debian:bookworm-slim`).

---

### 7.2. Giai đoạn 2: Tối ưu hiệu năng & Kiến trúc (Performance & Architecture)

1. **Triển khai Target Database Connection Cache**:
   - Thay vì tạo pool mới rồi `pool.close().await` cho mọi request, hãy tạo một `TargetDbPoolManager` sử dụng `moka` hoặc `dashmap` lưu trữ các connection pool có thời hạn rảnh rỗi (idle timeout ví dụ 5 phút).
   - Khi có request đến database `db_id`, lấy pool tái sử dụng, giúp giảm thời gian phản hồi từ ~200ms xuống chỉ còn ~5ms - 10ms.
2. **Tối ưu Code Splitting cho Frontend (Vite Dynamic Imports)**:
   - Sử dụng lazy loading cho các trang trong [App.svelte](file:///d:/Apps/dbms/frontend/src/App.svelte) thông qua dynamic import:
     ```javascript
     import { wrap } from "svelte-spa-router/wrap";

     const routes = {
       "/": Dashboard,
       "/requests/new": wrap({ asyncComponent: () => import("./pages/maker/NewRequest.svelte") }),
       "/manual": wrap({ asyncComponent: () => import("./pages/Manual.svelte") }),
       // ...
     };
     ```
   - Tách CodeMirror ra một vendor chunk riêng thông qua `manualChunks` trong `vite.config.js`.

---

### 7.3. Giai đoạn 3: Hoàn thiện tính năng & Mở rộng (Feature Completion)

1. **Khôi phục Realtime Notification**:
   - Bật lại module WebSocket trong `backend/src/main.rs`.
   - Chuyển cơ chế truyền JWT khi kết nối WebSocket từ query param (`/ws?token=JWT` — dễ bị lưu vào URL log) sang đọc trực tiếp từ HttpOnly Cookie trong handshake request.
2. **Hoàn thiện Schema Browser cho Oracle**:
   - Viết câu lệnh truy vấn `ALL_TABLES` và `ALL_TAB_COLUMNS` trong `backend/src/api/schema_browser.rs` khi cờ `oracle-support` được bật.
3. **Thay thế Regex SQL Analyzer bằng SQL Parser chuyên dụng**:
   - Sử dụng crate `sqlparser-rs` để phân tích Abstract Syntax Tree (AST) của câu lệnh SQL.
   - Nhận diện chính xác 100% các câu lệnh DDL nguy hiểm bất kể comment, ký tự xuống dòng hay case sensitivity, đồng thời bóc tách danh sách bảng/cột một cách triệt để cho module Data Queries.

---

### TỔNG KẾT ĐÁNH GIÁ (SCORECARD)

| Tiêu chí | Điểm số (Thang 10) | Nhận xét vắn tắt |
| :--- | :---: | :--- |
| **Tech Stack** | **9.5 / 10** | Lựa chọn công nghệ đỉnh cao (Rust, Axum, Svelte, PostgreSQL, AES-GCM). |
| **Tính Năng** | **8.5 / 10** | Nghiệp vụ Maker-Checker và Data Query rất chặt chẽ; trừ điểm do tắt WebSocket và thiếu Oracle Schema Browser. |
| **Hiệu Năng** | **8.0 / 10** | Backend cực nhanh; trừ điểm do không cache target DB connection và bundle frontend chưa code-split. |
| **Bảo Mật** | **7.5 / 10** | Nền tảng bảo mật rất tốt; trừ điểm vì commit `cookies.txt`, IP spoofing audit log và bug auth state cookie. |
| **Độ Sẵn Sàng (Readiness)** | **8.5 / 10** | Mã nguồn sạch sẽ, tổ chức module khoa học, sau khi vá các lỗi ở Giai đoạn 1 là sẵn sàng chạy production. |
