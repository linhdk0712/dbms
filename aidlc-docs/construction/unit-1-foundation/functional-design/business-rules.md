# Business Rules — Unit 1: Foundation

## BR-001: 4-Eyes Principle
- Maker không được approve/reject request của chính mình
- Enforce tại 2 layers:
  1. Application layer: check `approved_by != created_by` trước khi update
  2. DB constraint: `CHECK (approved_by IS NULL OR approved_by != created_by)`
- Nếu vi phạm: `AppError::FourEyesViolation` → HTTP 400

## BR-002: Immutable SQL Content After Submit
- Sau khi `PENDING`, `sql_content` và `database_id` không được sửa
- Maker muốn sửa phải Cancel → Clone → tạo request mới
- Nếu vi phạm: `AppError::ImmutableRequest` → HTTP 400

## BR-003: State Transition Validation
- Mọi thay đổi trạng thái phải đi qua `can_transition_to()`
- Invalid transition: `AppError::InvalidStateTransition { from, to }` → HTTP 400

## BR-004: Rejection Requires Reason
- Khi REJECTED, `rejection_reason` bắt buộc, min 10 ký tự
- Validate bởi `validator` crate trước khi xử lý

## BR-005: Audit Log Mandatory
- Mọi thay đổi trạng thái phải ghi AuditLog TRONG cùng transaction
- Nếu ghi audit fail → rollback toàn bộ transaction
- Không có audit log = không có state change

## BR-006: Permission Check trước khi tạo Request
- Maker chỉ tạo request trên DatabaseConnection mà họ có quyền
- Check qua `user_db_permissions` table
- Không có quyền: `AppError::DatabasePermissionDenied` → HTTP 403

## BR-007: SQL Execution trong Transaction
- Multi-statement SQL phải wrap trong transaction
- Một statement lỗi → rollback toàn bộ
- Ghi `rows_affected` khi thành công, `execution_error` khi thất bại

## BR-008: Timeout Enforcement
- Mỗi SQL execution có timeout theo `DatabaseConnection.timeout_secs`
- Mặc định: 30s, tối đa: 300s
- Quá timeout: status → FAILED, ghi audit log với action FAILED

## BR-009: Credential Encryption
- `DatabaseConnection.encrypted_credentials` luôn phải encrypt trước khi lưu
- Không bao giờ log hoặc trả về credentials trong API response
- Decrypt chỉ khi cần execute SQL, không lưu decrypted form vào memory lâu dài

## BR-010: Soft Delete Users
- User không bị DELETE khỏi DB, chỉ set `is_active = false`
- Giữ referential integrity với AuditLog (actor_id)
- User inactive không thể login (JWT verify kiểm tra `is_active`)

## PBT Properties — Unit 1 Foundation (PBT-01)

| Component | Property Category | Property |
|-----------|------------------|---------|
| `RequestStatus::can_transition_to` | Invariant | Không có transition nào tạo thành cycle (đồ thị DAG) |
| `RequestStatus::can_transition_to` | Invariant | `EXECUTED` và `FAILED` là terminal states — không transition đi đâu |
| Credential encrypt/decrypt | Round-trip | `decrypt(encrypt(plaintext, key), key) = plaintext` |
| `AuditAction` serialization | Round-trip | `deserialize(serialize(action)) = action` |
| `RequestStatus` serialization | Round-trip | `deserialize(serialize(status)) = status` |
| Claims encode/decode | Round-trip | `decode(encode(claims, secret), secret) = claims` (khi chưa hết hạn) |
