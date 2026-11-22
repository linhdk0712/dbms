# Presentation — SECS (SQL Execution Control System)

**Mục đích**: Trình bày với ban lãnh đạo về hệ thống kiểm soát thực thi SQL trên production  
**Số slide**: 3-4 slides  
**Đối tượng**: Ban lãnh đạo công ty tài chính

---

## SLIDE 1: Vấn đề & Rủi ro hiện tại

**Tiêu đề**: "Rủi ro khi thao tác trực tiếp trên Database Production"

**Nội dung chính:**

- **Thực trạng**: Nhân viên vận hành đang trực tiếp chạy SQL (INSERT, UPDATE, DELETE) trên môi trường production — không có kiểm soát, không có phê duyệt
- **Rủi ro nghiêm trọng**:
  - Một lệnh `UPDATE accounts SET balance = 0 WHERE 1=1` chạy nhầm → thiệt hại không thể phục hồi
  - Không truy vết được ai làm gì, khi nào, tại sao
  - Không có cơ chế "second pair of eyes" trước khi thực thi
- **Vi phạm quy định**:
  - **SOX** (Sarbanes-Oxley): yêu cầu audit trail đầy đủ, tối thiểu 7 năm, và phân tách người thực hiện với người phê duyệt
  - **DORA** (EU, hiệu lực 2025): tổ chức tài chính phải chứng minh kiểm soát mọi thao tác trên production
  - Kiểm toán nội bộ không có dữ liệu để đánh giá

**Điểm nhấn (visual)**: Biểu tượng cảnh báo đỏ, con số thiệt hại tiềm tàng nếu xảy ra sự cố

---

## SLIDE 2: Giải pháp — Hệ thống SECS

**Tiêu đề**: "SECS — Kiểm soát SQL trên Production theo chuẩn Maker-Checker"

**Mô tả ngắn**: Ứng dụng web nội bộ cho phép kiểm soát toàn bộ vòng đời thực thi SQL trên database production.

**Workflow cốt lõi:**

```
Maker soạn SQL → Hệ thống phân tích → Checker review → Approve → Tự động thực thi
                                                        ↓
                                                     Reject (kèm lý do)
```

**3 trụ cột chính:**

| # | Trụ cột | Mô tả |
|---|---------|-------|
| 1 | **Maker-Checker (4-eyes)** | Người tạo SQL ≠ người phê duyệt — bắt buộc |
| 2 | **Audit trail đầy đủ** | Mọi hành động được ghi nhận: ai, khi nào, lý do, kết quả — lưu tối thiểu 7 năm |
| 3 | **Phân tích SQL tự động** | Phát hiện DDL nguy hiểm (DROP, TRUNCATE), cảnh báo UPDATE/DELETE không có WHERE |

**Hỗ trợ đa database**: Oracle, PostgreSQL, MySQL, MongoDB

---

## SLIDE 3: Tính năng & Lợi ích theo Role

**Tiêu đề**: "Giá trị cho từng đối tượng sử dụng"

| Đối tượng | Tính năng chính | Lợi ích |
|-----------|----------------|---------|
| **Maker** (vận hành) | Soạn SQL, chọn DB đích, submit, theo dõi trạng thái, nhận thông báo realtime | Không cần access trực tiếp DB production — giảm rủi ro cá nhân |
| **Checker** (quản lý) | Review SQL + phân tích an toàn, approve/reject, xem audit | Ra quyết định có thông tin — hệ thống cảnh báo pattern nguy hiểm |
| **Admin** | Quản lý users, DB connections, phân quyền theo từng DB, dashboard tổng quan | Kiểm soát toàn bộ, dễ cấp/thu quyền |
| **Kiểm toán viên** | Export audit log, filter theo tiêu chí, dữ liệu immutable | Đáp ứng SOX/DORA compliance ngay lập tức |

**Bảo mật nổi bật:**

- Đăng nhập qua SSO công ty (không có mật khẩu riêng)
- Credentials database được mã hóa AES-256-GCM
- Phân quyền chi tiết theo từng database cho từng user
- Tất cả kết nối đến DB đích bắt buộc qua TLS

---

## SLIDE 4: Kiến trúc kỹ thuật & Kế hoạch triển khai

**Tiêu đề**: "Kiến trúc nhẹ, hiệu quả, dễ triển khai nội bộ"

**Tech stack:**

| Layer | Công nghệ | Lý do chọn |
|-------|-----------|------------|
| Backend | **Rust + Axum** | Performance cao, memory-safe, phù hợp fintech |
| Frontend | **Svelte + DaisyUI** | Nhẹ, nhanh, UX hiện đại |
| Database nội bộ | **PostgreSQL** | Ổn định, miễn phí, hỗ trợ enterprise |
| Auth | **OIDC/SSO** | Tích hợp trực tiếp với SSO hiện có của công ty |

**Chỉ số hiệu năng mục tiêu:**

- API response < 300ms (p95)
- Uptime 99.5%
- RTO < 30 phút, RPO < 1 giờ
- Hỗ trợ 100+ concurrent users

**Kế hoạch triển khai:**

| Giai đoạn | Nội dung | Thời gian |
|-----------|----------|-----------|
| Phase 1 | Foundation + Auth + Core workflow + Frontend | ~4 tuần (1 dev) |
| Phase 2 (tương lai) | Multi-level approval, Scheduled execution, Template SQL, Email notification | Theo nhu cầu |

**Triển khai**: Docker container, chạy trên hạ tầng nội bộ, không phụ thuộc cloud vendor

---

## Gợi ý khi trình bày

**Mở đầu**: Bắt đầu bằng câu chuyện rủi ro thực tế (slide 1) — tạo urgency

**Giọng điệu**: Tập trung vào giá trị kinh doanh, compliance, và giảm rủi ro — không đi sâu kỹ thuật

**Câu kết**: "Hệ thống này không phải lựa chọn — đây là yêu cầu bắt buộc để đáp ứng SOX/DORA và bảo vệ dữ liệu khách hàng."

**Q&A dự phòng:**

- "Tốn bao nhiêu?" → 1 dev × 4 tuần, không license fee (toàn bộ open-source stack)
- "Ảnh hưởng workflow hiện tại?" → Maker chỉ cần thêm 2 phút để submit thay vì chạy trực tiếp
- "Nếu Checker nghỉ phép?" → Có thể assign nhiều Checker, hệ thống hỗ trợ multi-level approval (phase 2)
- "Bảo mật như thế nào?" → SSO công ty, mã hóa credentials, audit immutable, không ai xóa được log
