# Requirements Document — SQL Execution Control System (SECS)

**Version**: 1.0  
**Date**: 2026-07-11  
**Status**: Approved

---

## 1. Intent Analysis

Người dùng yêu cầu review lại toàn bộ tài liệu thiết kế đã có và đưa ra đánh giá về tính đầy đủ, nhất quán, và sẵn sàng để bắt đầu implement.

**Kết luận**: Tài liệu chất lượng cao, cần giải quyết một số inconsistency trước khi viết code.

---

## 2. Scope

Hệ thống SECS — web application nội bộ cho công ty tài chính, hỗ trợ workflow Maker-Checker để kiểm soát thực thi SQL trên môi trường production.

---

## 3. Functional Requirements (tham chiếu `system-design.md` §2)

Đã được định nghĩa đầy đủ trong `system-design.md`. Tài liệu này ghi nhận các điểm đã confirm và clarified:

### 3.1 Core Workflow
- Maker-Checker workflow với 4-eyes principle
- State machine: `DRAFT → PENDING → APPROVED → EXECUTED | REJECTED | FAILED | CANCELLED`
- Immutable request sau khi PENDING

### 3.2 Database Support (v1)
- PostgreSQL ✅ (đầy đủ)
- MySQL ✅ (đầy đủ)
- Oracle ✅ — **Quyết định**: implement đầy đủ với `oracle` crate (theo Q6=A)
- MongoDB ✅ (đầy đủ)

### 3.3 Features Phase 2 (không implement trong v1)
- Multi-level approval (nhiều cấp approve)
- Scheduled Execution
- Template Library
- Email Notification
- Dry Run / Explain

---

## 4. Non-Functional Requirements (tham chiếu `system-design.md` §3)

Đã được định nghĩa đầy đủ. Các mục tiêu chính:

| NFR | Target |
|-----|--------|
| API response time (p95) | < 300ms |
| Uptime | 99.5% |
| Code coverage | ≥ 70% business logic |
| JWT expiry | Tối đa 8 giờ |
| SQL execution timeout | Mặc định 30s, tối đa 300s |
| Audit log retention | Tối thiểu 7 năm (SOX) |

---

## 5. Quyết định kỹ thuật (Technical Decisions)

### 5.1 Datetime Library — **Chọn `chrono 0.4`**

**Quyết định**: Sử dụng `chrono 0.4` làm datetime library chuẩn cho toàn dự án.

**Lý do kỹ thuật**:
- Tất cả code examples trong `libs-sqlx.md`, `libs-tokio-serde.md` đã dùng `chrono::DateTime<Utc>`
- sqlx 0.8 hỗ trợ cả hai, nhưng `chrono` có ecosystem lớn hơn và quen thuộc hơn
- `time 0.3` nhẹ hơn nhưng API ít trực quan hơn cho developers mới
- Consistency quan trọng hơn: toàn bộ examples đã viết với `chrono`

**Action cần thực hiện**: Cập nhật `implementation-plan.md` — thay `time = { version = "0.3", features = ["serde"] }` bằng `chrono = { version = "0.4", features = ["serde"] }`. Và sqlx features dùng `"chrono"` thay `"time"`.

### 5.2 Claims Struct — **Chọn có `user_id: Uuid` riêng biệt**

**Quyết định**: Claims struct sẽ có cả `sub: String` (theo JWT standard) VÀ `user_id: Uuid` (type-safe).

**Lý do kỹ thuật**:
- Type safety: handlers nhận `Uuid` trực tiếp, không cần parse mỗi lần
- `sub` vẫn giữ để đúng JWT standard (issuer, audience claim validation)
- Tránh parse error tại runtime nếu `sub` không phải valid UUID
- Pattern phù hợp với `principles.md` — prefer explicit types

**Chuẩn hóa Claims struct**:
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,         // JWT standard: user_id dưới dạng string
    pub user_id: Uuid,       // type-safe version của sub, để dùng trong handlers
    pub email: String,
    pub roles: Vec<String>,
    pub iss: String,         // "secs-backend"
    pub iat: usize,
    pub exp: usize,          // max 8 giờ
}
```

---

## 6. Extension Configuration

| Extension | Status | Enforcement |
|-----------|--------|-------------|
| Security Baseline (15 rules) | ✅ ENABLED | Blocking |
| Resiliency Baseline (15 rules) | ✅ ENABLED | Blocking (directional) |
| Property-Based Testing (10 rules) | ✅ ENABLED | Blocking |

---

## 7. Gaps cần giải quyết trong tài liệu

| # | Tài liệu | Hành động | Priority |
|---|----------|-----------|----------|
| 1 | `domain.md` | Thêm `CANCELLED` vào state diagram | LOW |
| 2 | `system-design.md` | Thêm response schema cho `GET /api/v1/dashboard` | LOW |
| 3 | `implementation-plan.md` | Sửa `time` → `chrono`, bỏ `async-trait` (Rust ≥ 1.75 không cần) | HIGH |
| 4 | `libs-auth.md` / `impl-refs.md` | Chuẩn hóa Claims struct theo quyết định §5.2 | HIGH |

---

## 8. Security Compliance — Requirements Phase

Đánh giá các SECURITY rules tại giai đoạn Requirements Analysis:

| Rule | Status | Ghi chú |
|------|--------|---------|
| SECURITY-01 (Encryption) | ✅ Addressed | `system-design.md` §3.3: TLS bắt buộc, AES-256-GCM cho credentials |
| SECURITY-02 (Access Logging) | ✅ Addressed | Audit log đầy đủ trong requirements |
| SECURITY-03 (App Logging) | ✅ Addressed | `tracing` với structured JSON, no sensitive data |
| SECURITY-04 (HTTP Headers) | ⚠️ Not mentioned | Cần thêm vào NFR: CSP, HSTS, X-Frame-Options |
| SECURITY-05 (Input Validation) | ✅ Addressed | `validator` crate, `ValidatedJson` extractor |
| SECURITY-06 (Least Privilege) | ✅ Addressed | App DB user: INSERT/SELECT only trên audit_log |
| SECURITY-07 (Network Config) | N/A | Infrastructure-level, ngoài scope của app code |
| SECURITY-08 (App Access Control) | ✅ Addressed | JWT middleware, 4-eyes, RBAC per DB |
| SECURITY-09 (Hardening) | ✅ Addressed | No stack trace ra client, generic error messages |
| SECURITY-10 (Supply Chain) | ⚠️ Partially | Cần thêm: `cargo audit` vào CI/CD pipeline |
| SECURITY-11 (Secure Design) | ✅ Addressed | auth/ module riêng, rate limiting trên /auth/login |
| SECURITY-12 (Auth & Credentials) | ✅ Addressed | SSO/OIDC, JWT 8h, no local passwords |
| SECURITY-13 (Data Integrity) | ✅ Addressed | Immutable audit log, append-only |
| SECURITY-14 (Alerting) | ⚠️ Partially | Log retention 7 năm OK, nhưng chưa có alert config |
| SECURITY-15 (Fail Safe) | ✅ Addressed | `thiserror`, global error handler, fail closed |

**Blocking findings**: SECURITY-04 (HTTP security headers chưa được mention) và SECURITY-14 (alerting chưa define) cần được address trong NFR Design stage.

---

## 9. Resiliency Compliance — Requirements Phase

Các RESILIENCY rules cần thêm câu hỏi trong giai đoạn này:

- **RESILIENCY-02** (RTO/RPO): `system-design.md` §3.2 đã định nghĩa Uptime 99.5%, RTO 30 phút, RPO 1 giờ → **Aligns with Pilot Light strategy**
- **RESILIENCY-03** (Change Management): Cần xác nhận từ user
- **RESILIENCY-04** (CI/CD & Rollback): Cần xác nhận từ user

Các câu hỏi này sẽ được hỏi trong giai đoạn NFR Requirements.

---

## 10. PBT Compliance — Requirements Phase

- **PBT-09** (Framework Selection): Rust → sử dụng `proptest` crate ✅ (sẽ add vào Cargo.toml)
- **PBT-01** (Property Identification): Sẽ thực hiện trong Functional Design stage

---

## 11. Kết luận

Tài liệu đã sẵn sàng để bắt đầu workflow. Các vấn đề kỹ thuật chính đã được giải quyết. Cần update một số tài liệu nhỏ song song với quá trình implement.
