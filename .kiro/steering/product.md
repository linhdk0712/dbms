# Product Overview

**SQL Execution Control System (SECS)** — Hệ thống quản lý và kiểm soát thực thi câu lệnh SQL trên môi trường production cho các công ty tài chính.

## Vấn đề cần giải quyết

Nhân viên vận hành thường phải thao tác trực tiếp với database (Oracle, MongoDB, v.v.) bằng các lệnh DML (INSERT, UPDATE, DELETE) trên môi trường PROD, dẫn đến rủi ro cao về mất mát hoặc sai lệch dữ liệu.

## Tính năng cốt lõi

### Workflow Maker - Checker
- **Maker**: Nhập câu lệnh SQL cần thực thi, gửi yêu cầu để Checker review
- **Checker**: Review, comment, approve hoặc reject yêu cầu
- Hệ thống tự động thực thi câu lệnh sau khi được approve
- Hỗ trợ nhiều cấp approve nếu cần

### Quản lý kết nối Database
- Quản lý nhiều database, schema, database user
- Hỗ trợ Oracle, PostgreSQL, MySQL, MongoDB
- Phân quyền theo từng database/schema cho từng user

### Xác thực & Phân quyền
- Đăng nhập qua SSO của công ty (OAuth2/OIDC)
- Phân role: Admin, Maker, Checker
- Mỗi user có thể có nhiều role trên nhiều database khác nhau

### Dashboard & Báo cáo
- Màn hình tổng hợp cho Maker: danh sách request đã gửi, trạng thái
- Màn hình tổng hợp cho Checker: danh sách request chờ duyệt, lịch sử
- Audit log đầy đủ: ai gửi, ai duyệt, khi nào, kết quả thực thi

## Đối tượng sử dụng

- **Admin**: Quản lý hệ thống, user, database connections
- **Maker**: Nhân viên vận hành — soạn và gửi câu lệnh SQL
- **Checker**: Trưởng nhóm/quản lý — review và phê duyệt câu lệnh

## Môi trường triển khai

Ứng dụng web nội bộ, triển khai trên hạ tầng công ty, tích hợp với SSO hiện có.
