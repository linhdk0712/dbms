# Domain Model & Business Rules

## Các khái niệm chính

### SqlRequest (Yêu cầu thực thi SQL)
Đơn vị trung tâm của hệ thống. Một request gồm:
- Câu lệnh SQL cần thực thi
- Database đích (connection + schema)
- Người tạo (Maker)
- Trạng thái hiện tại
- Lịch sử approve/reject

**Trạng thái (RequestStatus):**
```
DRAFT → PENDING → APPROVED → EXECUTED
                ↓         ↓
             REJECTED   FAILED (nếu thực thi lỗi)
             
PENDING → CANCELLED (Maker hủy)
```

### Role người dùng
- **Admin**: Quản lý users, database connections, cấu hình hệ thống
- **Maker**: Tạo và gửi SQL request. Không thể tự approve request của mình
- **Checker**: Review và approve/reject request. Không thể approve request mà mình tạo
- Một user có thể có nhiều role, nhưng **Maker và Checker của cùng một request phải là 2 người khác nhau** (4-eyes principle)

### DatabaseConnection
Thông tin kết nối đến một database đích:
- Loại DB: `oracle | postgresql | mysql | mongodb`
- Host, port, database name, schema
- Credentials (encrypt khi lưu)
- Danh sách user được phép tạo request trên DB này

## Business Rules quan trọng

1. **4-eyes principle**: Maker không được tự approve request của mình, dù có role Checker
2. **Immutable request**: Sau khi gửi (`PENDING`), Maker không được sửa nội dung SQL — phải hủy và tạo mới
3. **Audit trail bắt buộc**: Mọi thay đổi trạng thái phải ghi log với timestamp, user thực hiện, và lý do (nếu reject)
4. **Thực thi nguyên tử**: Nếu câu SQL gồm nhiều statement, cần wrap trong transaction — rollback toàn bộ nếu một statement lỗi
5. **Không thực thi DDL tự động**: Các câu lệnh DDL (DROP, TRUNCATE, ALTER) cần flag cảnh báo đặc biệt khi Checker review
6. **Timeout thực thi**: Mỗi câu SQL có timeout tối đa (cấu hình được), quá thời gian sẽ bị cancel và ghi log lỗi

## Audit Log

Mỗi bản ghi audit log gồm:
- `request_id` — liên kết đến SqlRequest
- `action` — `CREATED | SUBMITTED | APPROVED | REJECTED | EXECUTED | FAILED | CANCELLED`
- `actor_id` — user thực hiện hành động
- `timestamp`
- `detail` — lý do reject, error message khi FAILED, v.v.
- `execution_result` — số rows affected (nếu thực thi thành công)

## Phân quyền theo Database

Mỗi user được cấp quyền trên từng DatabaseConnection cụ thể:
- Không có quyền global — phải cấp explicit từng DB
- Admin có thể cấp/thu quyền bất kỳ lúc nào
- Khi quyền bị thu hồi, các request đang `PENDING` của user đó không bị ảnh hưởng
