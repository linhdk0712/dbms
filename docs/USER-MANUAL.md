# Hướng Dẫn Sử Dụng Hệ Thống SECS
## SQL Execution Control System — Tài liệu người dùng

**Phiên bản**: 1.0  
**Cập nhật**: 2026-08-08

---

## Mục lục

1. [Giới thiệu](#1-giới-thiệu)
2. [Đăng nhập & Xác thực](#2-đăng-nhập--xác-thực)
3. [Giao diện tổng quan](#3-giao-diện-tổng-quan)
4. [Dashboard](#4-dashboard)
5. [Hướng dẫn cho Maker](#5-hướng-dẫn-cho-maker)
6. [Hướng dẫn cho Checker](#6-hướng-dẫn-cho-checker)
7. [Data Query — Truy vấn dữ liệu](#7-data-query--truy-vấn-dữ-liệu)
8. [Hướng dẫn cho Admin](#8-hướng-dẫn-cho-admin)
9. [Change Requests — Tích hợp ITSM](#9-change-requests--tích-hợp-itsm)
10. [Thông báo realtime](#10-thông-báo-realtime)
11. [Phân tích SQL bằng AI](#11-phân-tích-sql-bằng-ai)
12. [Các quy tắc nghiệp vụ quan trọng](#12-các-quy-tắc-nghiệp-vụ-quan-trọng)
13. [FAQ — Câu hỏi thường gặp](#13-faq--câu-hỏi-thường-gặp)

---

## 1. Giới thiệu

### SECS là gì?

SECS (SQL Execution Control System) là hệ thống quản lý và kiểm soát việc thực thi
câu lệnh SQL trên môi trường production. Hệ thống áp dụng quy trình **Maker-Checker**
(4-eyes principle) để đảm bảo mọi thao tác trên database đều được review và phê duyệt
trước khi thực thi.

### Đối tượng sử dụng

| Role | Mô tả | Quyền hạn chính |
|------|--------|-----------------|
| **Admin** | Quản trị viên hệ thống | Quản lý users, databases, cấu hình hệ thống |
| **Maker** | Nhân viên vận hành | Soạn và gửi SQL request |
| **Checker** | Quản lý/trưởng nhóm | Review và phê duyệt SQL request |
| **Approver** | Người phê duyệt Data Query | Quản lý approval rules, phê duyệt data queries |

> **Lưu ý**: Một user có thể có nhiều role đồng thời (ví dụ: vừa là Maker vừa là Checker).

### Database hỗ trợ

- **PostgreSQL** — Hỗ trợ đầy đủ (execute + schema browser)
- **MySQL** — Hỗ trợ đầy đủ (execute + schema browser)
- **Oracle** — Hỗ trợ execute, schema browser đang phát triển
- **MongoDB** — Hỗ trợ execute JSON commands + schema browser (list collections)

---

## 2. Đăng nhập & Xác thực

### 2.1 Đăng nhập qua SSO

SECS sử dụng **Single Sign-On (SSO)** qua giao thức OAuth2/OpenID Connect, tích hợp
với hệ thống xác thực nội bộ của công ty (Keycloak, Azure AD, Okta, v.v.).

**Các bước đăng nhập:**

1. Truy cập URL hệ thống SECS (ví dụ: `https://secs.company.internal`)
2. Nhấn nút **"Đăng nhập bằng SSO"**
3. Hệ thống redirect đến trang đăng nhập SSO của công ty
4. Nhập username/password (hoặc dùng MFA nếu có)
5. Sau khi xác thực thành công, tự động quay lại SECS với session đã đăng nhập

### 2.2 Phiên làm việc (Session)

- Phiên làm việc có thời hạn tối đa **8 giờ**
- Sau khi hết hạn, cần đăng nhập lại
- Token được lưu trong trình duyệt — đóng tab không bị mất session
- Nhấn **"Đăng xuất"** ở thanh navigation trên cùng để kết thúc phiên

### 2.3 Tài khoản lần đầu

- Khi đăng nhập SSO lần đầu, hệ thống tự động tạo tài khoản trong SECS
- User đầu tiên đăng nhập vào hệ thống sẽ được gán role **Admin** (bootstrap)
- Các user sau cần được Admin gán role phù hợp

### 2.4 Rate Limiting

- Endpoint login được giới hạn **10 request/phút** cho mỗi địa chỉ IP
- Nếu vượt quá, hệ thống trả về HTTP 429 (Too Many Requests)
- Chờ 1 phút trước khi thử lại

---

## 3. Giao diện tổng quan

### 3.1 Layout chính

Sau khi đăng nhập, giao diện gồm 3 phần:

```
┌──────────────────────────────────────────────┐
│            NAVBAR (thanh trên)                │
├──────────┬───────────────────────────────────┤
│          │                                   │
│ SIDEBAR  │         NỘI DUNG CHÍNH           │
│ (menu    │                                   │
│  bên     │                                   │
│  trái)   │                                   │
│          │                                   │
└──────────┴───────────────────────────────────┘
```

### 3.2 Thanh Navigation (Navbar)

- **Logo SECS** — Nhấn để về Dashboard
- **Theme toggle** — Chuyển đổi giao diện sáng/tối
- **User dropdown** — Hiển thị tên, email, roles, nút đăng xuất

### 3.3 Sidebar (Menu bên trái)

Menu hiển thị theo role của user:

| Mục menu | Hiển thị cho | Chức năng |
|----------|-------------|-----------|
| Dashboard | Tất cả | Trang tổng quan |
| Tạo request mới | Maker | Form tạo SQL request |
| Requests của tôi | Maker | Danh sách request đã tạo |
| Change Requests | Maker | Quản lý CR từ ManageEngine |
| Chờ duyệt | Checker | Danh sách request chờ approve |
| Tạo Query mới | Tất cả | Form tạo data query |
| Data Queries | Tất cả | Danh sách data queries |
| Phê duyệt Query | Tất cả | Danh sách query chờ duyệt |
| Databases | Admin | Quản lý kết nối database |
| Users | Admin | Quản lý người dùng |
| Approval Matrix | Admin/Approver | Cấu hình quy tắc phê duyệt |
| Approver Units | Admin/Approver | Quản lý đơn vị phê duyệt |

---

## 4. Dashboard

Dashboard hiển thị thống kê tùy theo role:

### 4.1 Dashboard cho Admin

Hiển thị thống kê toàn hệ thống:
- Tổng số requests
- Requests đang chờ duyệt (Pending)
- Requests đã duyệt (Approved)
- Requests đã thực thi (Executed)
- Requests thực thi lỗi (Failed)
- Requests bị từ chối (Rejected)
- Requests đã hủy (Cancelled)

### 4.2 Dashboard cho Checker

Hiển thị thống kê liên quan đến hoạt động review:
- Số request đang chờ review
- Tổng số đã review
- Số đã approve
- Số đã reject

### 4.3 Dashboard cho Maker

Hiển thị thống kê requests của bản thân:
- Tổng requests đã tạo
- Requests đang pending
- Requests đã approved
- Requests đã executed
- Requests bị rejected
- Requests bị failed
- Requests đã cancelled

---

## 5. Hướng dẫn cho Maker

### 5.1 Tạo SQL Request mới

**Đường dẫn**: Sidebar → "Tạo request mới"

**Các bước thực hiện:**

1. **Chọn Database đích**
   - Dropdown hiển thị danh sách databases mà bạn có quyền
   - Nếu danh sách trống → liên hệ Admin để được cấp quyền

2. **Nhập mô tả (Description)**
   - Mô tả ngắn gọn mục đích của SQL request (1-500 ký tự)
   - Ví dụ: "Cập nhật email cho khách hàng ID 12345"

3. **Nhập nội dung SQL**
   - Sử dụng SQL Editor tích hợp (có syntax highlighting)
   - Hỗ trợ SQL (PostgreSQL, MySQL, Oracle) và JSON commands (MongoDB)
   - Giới hạn: 1-50,000 ký tự
   - Ví dụ:
     ```sql
     UPDATE customers SET email = 'new@email.com' WHERE id = 12345;
     ```

4. **Liên kết Change Request (tùy chọn)**
   - Dán URL từ ManageEngine ServiceDesk Plus
   - Hệ thống tự động parse CR number và fetch metadata
   - Ví dụ URL: `https://sdp.company.internal/app/itom/ChangeDetails.do?CHANGEID=12345`

5. **Chọn hành động:**
   - **Lưu nháp (Draft)**: Lưu để chỉnh sửa sau
   - **Gửi duyệt (Submit)**: Gửi ngay cho Checker review

### 5.2 Quản lý Requests

**Đường dẫn**: Sidebar → "Requests của tôi"

Trang hiển thị danh sách requests đã tạo với:
- Phân trang (20 items/trang)
- Bộ lọc theo trạng thái (Draft, Pending, Approved, Executed, Failed, Rejected, Cancelled)
- Thông tin: ID, mô tả, database, trạng thái, ngày tạo

### 5.3 Chi tiết Request

Nhấn vào một request để xem chi tiết:
- Nội dung SQL (hiển thị trong editor read-only)
- Thông tin database đích
- Trạng thái hiện tại (badge màu)
- Lý do reject (nếu bị từ chối)
- Kết quả thực thi (rows affected hoặc error message)
- Audit timeline — lịch sử mọi hành động trên request này

### 5.4 Các thao tác trên Request

| Thao tác | Trạng thái cho phép | Mô tả |
|----------|---------------------|--------|
| **Sửa** | DRAFT | Chỉnh sửa mô tả và nội dung SQL |
| **Submit** | DRAFT | Gửi cho Checker review → trạng thái chuyển sang PENDING |
| **Cancel** | PENDING | Hủy request đang chờ duyệt → trạng thái CANCELLED |
| **Clone** | Bất kỳ | Tạo bản sao dưới dạng DRAFT mới |
| **AI Analysis** | DRAFT, PENDING | Phân tích SQL bằng AI (xem mục 11) |

### 5.5 Vòng đời của một Request

```
  ┌─────────┐     Submit      ┌─────────┐     Approve     ┌──────────┐
  │  DRAFT  │────────────────>│ PENDING │───────────────>│ APPROVED │
  └─────────┘                 └─────────┘                 └──────────┘
       │                           │                           │
       │ (Edit)                    │                           │ (Auto-execute)
       │                           │                           ▼
       │                           │ Cancel              ┌──────────┐
       │                           ├────────────────────>│CANCELLED │
       │                           │                     └──────────┘
       │                           │ Reject
       │                           ├────────────────────>┌──────────┐
       │                           │                     │ REJECTED │
       │                           │                     └──────────┘
       │                           │
       │                           │               ┌──────────┐
       │                           │               │ EXECUTED │ (thành công)
       │                           │               └──────────┘
       │                           │               ┌──────────┐
       │                           │               │  FAILED  │ (lỗi)
       │                           │               └──────────┘
```

### 5.6 Quy tắc quan trọng cho Maker

- **Không thể tự approve request của mình** — dù có role Checker
- **Không thể sửa SQL sau khi submit** — phải Cancel rồi tạo mới (hoặc Clone)
- **Chỉ thấy request của mình** — không xem được request của người khác
- **Phải có quyền trên database** — Admin cấp quyền trước khi tạo request

---

## 6. Hướng dẫn cho Checker

### 6.1 Xem danh sách chờ duyệt

**Đường dẫn**: Sidebar → "Chờ duyệt"

Trang hiển thị:
- Danh sách requests có trạng thái PENDING
- **Không hiển thị request do chính mình tạo** (4-eyes principle)
- Thông tin: mô tả, database đích, người tạo, ngày submit, tiến trình approval

### 6.2 Review chi tiết Request

Nhấn vào một request để xem chi tiết:
- **Nội dung SQL đầy đủ** trong editor có syntax highlighting
- **Database connection info** (loại DB, host, database name, schema)
- **DDL Analysis** — cảnh báo tự động nếu SQL chứa:
  - `DROP` — Mức độ: **Danger** ⚠️
  - `TRUNCATE` — Mức độ: **Danger** ⚠️
  - `ALTER` — Mức độ: **Warning** ⚡
  - `CREATE` — Mức độ: **Warning** ⚡
- **Tiến trình approval** (bao nhiêu level đã approve / tổng cần)

### 6.3 Validate SQL

Trước khi approve, Checker có thể nhấn **"Validate"** để:
- Hệ thống chạy `EXPLAIN` trên database đích
- Kiểm tra SQL có hợp lệ không (syntax, table/column tồn tại)
- Xem execution plan (estimated cost, rows affected)
- **Không thực thi SQL** — chỉ kiểm tra

### 6.4 Approve Request

1. Nhấn nút **"Approve"**
2. Hệ thống kiểm tra:
   - Bạn không phải người tạo request (4-eyes)
   - Bạn chưa approve request này trước đó (no duplicate)
   - Request đang ở trạng thái PENDING
3. Approval được ghi nhận
4. Nếu đủ số level approval cần thiết → **SQL tự động thực thi**
5. Kết quả thực thi:
   - **Thành công**: Trạng thái → EXECUTED, hiển thị rows affected
   - **Lỗi**: Trạng thái → FAILED, hiển thị error message

### 6.5 Reject Request

1. Nhấn nút **"Reject"**
2. Nhập lý do từ chối (bắt buộc, tối thiểu 10 ký tự)
3. Request chuyển sang trạng thái REJECTED
4. Maker nhận thông báo realtime

### 6.6 Multi-Level Approval

Một request có thể yêu cầu nhiều cấp phê duyệt (ví dụ: DDL commands cần 2 Checker approve):
- Hiển thị tiến trình: "1/2 approvals" hoặc "2/2 approvals"
- Mỗi Checker chỉ approve 1 lần cho mỗi request
- SQL chỉ thực thi khi đủ tất cả approval levels

### 6.7 Quy tắc quan trọng cho Checker

- **Không thấy request mình tạo** trong danh sách chờ duyệt
- **Không approve 2 lần** cho cùng một request (HTTP 409 Conflict)
- **Đọc kỹ DDL warnings** — DROP/TRUNCATE có thể gây mất dữ liệu
- **Dùng Validate trước Approve** — đặc biệt với SQL phức tạp

---

## 7. Data Query — Truy vấn dữ liệu

Data Query là tính năng cho phép thực thi **câu lệnh SELECT** (read-only) với quy trình
phê duyệt riêng, phù hợp cho việc trích xuất dữ liệu phục vụ báo cáo, kiểm tra.

### 7.1 Tạo Data Query

**Đường dẫn**: Sidebar → "Tạo Query mới"

**Các bước:**

1. **Nhập tiêu đề** (3-500 ký tự)
   - Mô tả mục đích truy vấn
   - Ví dụ: "Báo cáo doanh thu tháng 7/2026"

2. **Chọn Database & Schema**
   - Chọn database đích
   - Nhập tên schema (ví dụ: `public`, `dbo`)

3. **Chọn chế độ query:**

   **Basic Mode (Giao diện trực quan):**
   - Chọn table từ schema browser
   - Tick columns cần lấy
   - Thêm điều kiện lọc (filters)
   - Hệ thống tự generate SQL

   **Advanced Mode (SQL tự do):**
   - Viết SQL SELECT trực tiếp trong editor
   - Giới hạn: 5-50,000 ký tự

4. **Nhập Ticket ID** (1-100 ký tự)
   - Mã ticket/task liên quan (JIRA, ServiceNow, etc.)
   - Ví dụ: "TASK-1234"

5. **Gán users được phép thực thi (tùy chọn)**
   - Ngoài bản thân, có thể ủy quyền cho users khác thực thi query
   - Chọn từ danh sách users trong hệ thống

6. **Ngày hết hạn (tùy chọn)**
   - Sau ngày này, query không thể thực thi được nữa
   - Khuyến nghị: set expiry cho các queries nhạy cảm

7. **Submit** — Gửi cho approver phê duyệt

### 7.2 Quy trình phê duyệt Data Query

```
DRAFT → PENDING_APPROVAL → APPROVED → (Execute nhiều lần)
                 ↓
             REJECTED
```

**Approval Matrix** quyết định cần bao nhiêu cấp duyệt:
- Cấu hình theo database + schema + table
- Ví dụ: table `customers` cần 2 cấp (DBA Team → Data Owner Team)
- Ví dụ: table `logs` chỉ cần 1 cấp (DBA Team)

### 7.3 Phê duyệt Data Query (cho Approver)

**Đường dẫn**: Sidebar → "Phê duyệt Query"

Hiển thị danh sách queries đang chờ phê duyệt dựa trên **Approver Unit** của bạn:
- Chỉ thấy queries ở level mà unit của bạn cần approve
- Xem chi tiết: SQL query, tables accessed, ticket ID, người tạo

**Approve:**
- Nhấn "Approve" + comment (tùy chọn)
- Nếu đủ tất cả levels → query chuyển sang APPROVED

**Reject:**
- Nhấn "Reject" + lý do (bắt buộc, tối thiểu 10 ký tự)
- Query chuyển sang REJECTED

### 7.4 Thực thi Data Query

Sau khi query được APPROVED:

1. Mở chi tiết query
2. Nhấn **"Execute"**
3. Hệ thống thực thi SELECT trên database đích
4. Kết quả hiển thị dưới dạng bảng:
   - Tên columns
   - Dữ liệu rows (tối đa 10,000 rows)
   - Thời gian thực thi
   - Cảnh báo nếu kết quả bị cắt (truncated)
5. Mỗi lần thực thi được ghi nhận trong lịch sử

**Ai được thực thi:**
- Người tạo query
- Users được gán (assigned users)
- Admin

**Lưu ý:**
- Query có thể thực thi nhiều lần (cho đến khi hết hạn)
- Mỗi lần thực thi đều được audit (ai, khi nào, bao nhiêu rows)
- Timeout áp dụng theo cấu hình database

### 7.5 Xem lịch sử thực thi

**Đường dẫn**: Chi tiết query → tab "Executions"

Hiển thị:
- Thời gian thực thi
- Người thực thi
- Số rows trả về
- Duration (ms)
- Trạng thái (success/failed)

---

## 8. Hướng dẫn cho Admin

### 8.1 Quản lý Database Connections

**Đường dẫn**: Sidebar → "Databases"

#### Tạo Database Connection mới

1. Nhấn **"Thêm Database"**
2. Điền thông tin:
   - **Tên**: Tên hiển thị (ví dụ: "PROD - Oracle CRM")
   - **Loại DB**: PostgreSQL / MySQL / Oracle / MongoDB
   - **Host**: Hostname hoặc IP (ví dụ: `db.company.internal`)
   - **Port**: Port kết nối (ví dụ: 5432, 3306, 1521, 27017)
   - **Database Name**: Tên database
   - **Schema** (tùy chọn): Schema mặc định
   - **Username**: Tài khoản DB
   - **Password**: Mật khẩu DB
   - **Timeout**: Thời gian chờ tối đa khi thực thi (1-300 giây, mặc định 30)

3. Nhấn **"Test Connection"** để kiểm tra trước khi lưu
4. Nhấn **"Tạo"** nếu test thành công

> **Bảo mật**: Credentials được mã hóa AES-256-GCM trước khi lưu vào database hệ thống.
> Không ai có thể xem lại password sau khi tạo — chỉ có thể thay đổi.

#### Quản lý Connections

- **Xem danh sách**: Tất cả connections với thông tin (tên, loại, host, status)
- **Test Connection**: Kiểm tra kết nối đang hoạt động
- **Sửa thông tin**: Host, port, database name, schema, timeout, active/inactive
- **Đổi Credentials**: Cập nhật username/password (ví dụ: khi rotate password)
- **Vô hiệu hóa**: Set `is_active = false` — không cho phép tạo request mới lên DB này

#### Lưu ý quan trọng

- Credentials **không** được log ra console hay trả về trong API response
- Kết nối đến database đích yêu cầu **TLS/SSL** (sslmode=require)
- Mỗi thao tác đổi credentials được ghi audit log

### 8.2 Quản lý Users

**Đường dẫn**: Sidebar → "Users"

#### Danh sách Users

Hiển thị tất cả users đã đăng nhập SSO vào hệ thống:
- Tên hiển thị
- Email
- Roles hiện tại
- Trạng thái (Active/Inactive)
- Ngày tạo

#### Gán Role

1. Nhấn vào user cần quản lý
2. Tick/untick các roles: Admin, Maker, Checker, Approver
3. Nhấn **"Lưu"**

#### Cấp quyền Database

Mỗi user cần được cấp quyền explicit trên từng database:

1. Mở trang chi tiết user
2. Phần **"Database Permissions"**
3. Nhấn **"Thêm quyền"**
4. Chọn database từ dropdown
5. Nhấn **"Cấp quyền"**

**Thu hồi quyền:**
- Nhấn icon xóa bên cạnh permission
- Requests đang PENDING **không bị ảnh hưởng** khi thu hồi quyền

#### Vô hiệu hóa User

- Set user thành **Inactive**
- User inactive không thể đăng nhập
- Không xóa vĩnh viễn (giữ lại cho audit trail)

### 8.3 Quản lý Approver Units

**Đường dẫn**: Sidebar → "Approver Units"

Approver Units là các nhóm phê duyệt (ví dụ: "DBA Team", "Security Team", "Data Owner").

#### Tạo Approver Unit

1. Nhấn **"Tạo Unit"**
2. Nhập tên (1-50 ký tự) và mô tả (tùy chọn)
3. Nhấn **"Tạo"**

#### Gán User vào Unit

1. Vào trang quản lý Users
2. Mở chi tiết user
3. Phần **"Approver Units"**
4. Chọn unit và nhấn **"Gán"**

> Một user có thể thuộc nhiều Approver Units.

#### Sửa / Xóa Unit

- **Sửa**: Đổi tên, mô tả, active/inactive
- **Xóa**: Xóa unit (chỉ khi chưa có rule nào sử dụng)

### 8.4 Cấu hình Approval Matrix

**Đường dẫn**: Sidebar → "Approval Matrix"

Approval Matrix định nghĩa quy trình phê duyệt cho Data Queries:

#### Tạo Rule mới

1. Nhấn **"Tạo Rule"**
2. **Chọn Database** — rule áp dụng cho database nào
3. **Schema** (tùy chọn) — nếu trống, áp dụng cho mọi schema trong DB
4. **Table** (tùy chọn) — nếu trống, áp dụng cho mọi table trong schema
5. **Thêm Approval Levels:**
   - Level 1: Chọn Approver Unit (ví dụ: "DBA Team")
   - Level 2: Chọn Approver Unit (ví dụ: "Data Owner Team")
   - Có thể thêm nhiều levels
6. Nhấn **"Tạo"**

#### Logic matching rule

Khi user submit một data query, hệ thống tìm rule phù hợp nhất theo thứ tự ưu tiên:
1. Exact match: database + schema + table
2. Schema match: database + schema (table = null)
3. Database match: database (schema = null, table = null)
4. Nếu không tìm thấy rule → từ chối submit

#### Ví dụ cấu hình

| Database | Schema | Table | Levels |
|----------|--------|-------|--------|
| PROD-CRM | public | customers | DBA Team → Data Owner → Security |
| PROD-CRM | public | * | DBA Team → Data Owner |
| PROD-CRM | * | * | DBA Team |
| DEV-DB | * | * | DBA Team (1 level) |

---

## 9. Change Requests — Tích hợp ITSM

### 9.1 Giới thiệu

SECS tích hợp với **ManageEngine ServiceDesk Plus** để liên kết SQL requests với
Change Requests (CR) trong hệ thống ITSM. Điều này giúp:
- Truy vết: SQL request nào thuộc change nào
- Kiểm soát: Chỉ thực thi trong maintenance window
- Tuân thủ: Audit trail đầy đủ giữa ITSM và DB changes

### 9.2 Tạo Change Request

**Đường dẫn**: Sidebar → "Change Requests"

1. Nhấn **"Thêm CR"**
2. Dán URL từ ManageEngine (ví dụ: `https://sdp.company.internal/...?CHANGEID=12345`)
3. Hệ thống tự động:
   - Parse CR number từ URL
   - Gọi ManageEngine API lấy metadata (title, status, maintenance window)
4. CR được lưu với thông tin:
   - CR Number
   - Tiêu đề
   - Trạng thái (Approved, In Progress, Closed, v.v.)
   - Maintenance Window (ngày bắt đầu — ngày kết thúc)

### 9.3 Liên kết SQL Request với CR

Khi tạo SQL Request mới:
- Dán URL ManageEngine vào trường **"Change Request URL"**
- Hệ thống tự động liên kết request với CR tương ứng
- Nếu CR chưa tồn tại trong SECS → tự động tạo mới

### 9.4 Maintenance Window

Khi một SQL request được liên kết với CR có maintenance window:
- Hệ thống kiểm tra thời gian hiện tại có nằm trong window không
- **Trong window**: Thực thi bình thường
- **Ngoài window**: Cảnh báo hoặc từ chối thực thi (tùy cấu hình)

### 9.5 Đồng bộ CR

- Nhấn **"Sync"** trên một CR để cập nhật metadata mới nhất từ ManageEngine
- Cập nhật: title, status, scheduled_start_time, scheduled_end_time

### 9.6 Xóa CR

- Chỉ Admin mới có quyền xóa CR
- Khi xóa, các SQL requests liên kết sẽ tự động gỡ liên kết (set CR = null)

---

## 10. Thông báo Realtime

### 10.1 Cơ chế hoạt động

SECS sử dụng **WebSocket** để push thông báo realtime đến trình duyệt.
Khi đăng nhập, hệ thống tự động kết nối WebSocket — không cần cấu hình gì thêm.

### 10.2 Các loại thông báo

| Loại | Nhận bởi | Mô tả |
|------|----------|--------|
| New Request | Checker | Có request mới cần review |
| Request Approved | Maker (owner) | Request của bạn được approve |
| Request Rejected | Maker (owner) | Request bị reject (kèm lý do) |
| Request Executed | Maker (owner) | SQL đã thực thi thành công |
| Request Failed | Maker (owner) | SQL thực thi lỗi |
| Partial Approval | Maker (owner) | Request được approve 1 level (chưa đủ) |

### 10.3 Quy tắc filtering theo role

- **Admin**: Nhận tất cả thông báo
- **Maker**: Chỉ nhận thông báo liên quan request của mình
- **Checker**: Nhận thông báo request mới + request mình đã review trước đó

### 10.4 Hiển thị

- Thông báo hiển thị dưới dạng **Toast** (popup nhỏ góc phải) 
- Tự động ẩn sau vài giây
- Khi mất kết nối WebSocket → tự động reconnect

---

## 11. Phân tích SQL bằng AI

### 11.1 Giới thiệu

SECS tích hợp AI (Large Language Model) để phân tích SQL trước khi thực thi,
giúp phát hiện sớm các vấn đề về performance, security, và logic.

### 11.2 Cách sử dụng

1. Mở chi tiết SQL Request (trạng thái DRAFT hoặc PENDING)
2. Nhấn nút **"🤖 AI Analysis"**
3. Chờ vài giây — hệ thống:
   - Trích xuất tên tables từ SQL
   - Kết nối database đích để lấy schema (columns, indexes)
   - Gửi SQL + schema context cho AI phân tích
4. Xem kết quả phân tích

### 11.3 Kết quả phân tích

AI trả về đánh giá chi tiết:

- **Risk Score** (1-10): Mức độ rủi ro tổng thể
  - 1-3: Thấp (✅ Safe)
  - 4-6: Trung bình (⚡ Cần review kỹ)
  - 7-10: Cao (⚠️ Cẩn thận)

- **Performance Issues**: Các vấn đề hiệu năng tiềm ẩn
  - Full table scan (thiếu index)
  - Query quá phức tạp
  - JOIN nhiều bảng lớn

- **Security Issues**: Vấn đề bảo mật
  - Thao tác trên dữ liệu nhạy cảm
  - Missing WHERE clause (UPDATE/DELETE toàn bộ)
  - Privilege escalation risk

- **Suggestions**: Gợi ý cải thiện
  - Thêm WHERE clause
  - Sử dụng transaction
  - Backup trước khi thực thi

### 11.4 Giới hạn

- AI analysis có thể bị **tắt** bởi Admin (qua cấu hình)
- Rate limit: giới hạn số lần phân tích mỗi user trong một khoảng thời gian
- Kết quả được **cache** — cùng SQL sẽ không gọi AI lần thứ 2
- Nếu không introspect được schema → vẫn phân tích nhưng ít chính xác hơn

### 11.5 Ai có thể sử dụng?

- **Maker**: Phân tích request của chính mình (DRAFT hoặc PENDING)
- **Checker**: Phân tích request đang PENDING (đang review)

---

## 12. Các quy tắc nghiệp vụ quan trọng

### 12.1 Nguyên tắc 4-Eyes (Four-Eyes Principle)

> Người tạo request KHÔNG THỂ tự approve request đó.

- Dù user có cả role Maker và Checker
- Hệ thống kiểm tra `created_by ≠ approved_by`
- Vi phạm → HTTP 403 (FourEyesViolation)

### 12.2 Immutable Request

> Sau khi submit (PENDING), nội dung SQL không thể chỉnh sửa.

- Chỉ DRAFT mới cho phép edit
- Muốn thay đổi: Cancel → Clone → Sửa → Submit lại

### 12.3 Audit Trail bắt buộc

> Mọi hành động trên request đều được ghi log.

Mỗi bản ghi audit chứa:
- Hành động (CREATED, SUBMITTED, APPROVED, REJECTED, EXECUTED, FAILED, CANCELLED)
- Người thực hiện
- Thời gian chính xác
- Chi tiết (lý do reject, error message, rows affected)
- Địa chỉ IP

### 12.4 DDL Warning

> Câu lệnh DDL (DROP, TRUNCATE, ALTER, CREATE) được flag đặc biệt.

- Checker thấy cảnh báo rõ ràng khi review
- DDL thường yêu cầu nhiều approval levels hơn
- Maker nên ghi rõ lý do trong description khi submit DDL

### 12.5 Thực thi nguyên tử (Atomic Execution)

> Mọi SQL execution được wrap trong transaction.

- Nếu SQL gồm nhiều statements → tất cả thành công hoặc tất cả rollback
- Timeout áp dụng — quá thời gian → rollback + ghi log FAILED
- Connection TLS bắt buộc

### 12.6 Phân quyền Database

> Không có quyền global — phải cấp explicit từng database.

- Admin cấp quyền user → database
- Maker chỉ thấy databases mình có quyền
- Thu hồi quyền → không ảnh hưởng requests đang PENDING

---

## 13. FAQ — Câu hỏi thường gặp

### Q: Tôi không thấy database nào khi tạo request?
**A**: Admin chưa cấp quyền cho bạn trên database nào. Liên hệ Admin để được cấp quyền
(Admin → Users → [Tên bạn] → Database Permissions → Thêm quyền).

### Q: Request bị reject, làm sao sửa lại?
**A**: Request đã submit không thể sửa. Sử dụng nút **"Clone"** để tạo bản sao DRAFT,
chỉnh sửa SQL theo feedback từ Checker, rồi submit lại.

### Q: Tôi có cả role Maker và Checker, sao không approve được request của mình?
**A**: Đây là thiết kế cố ý — **4-eyes principle**. Mọi SQL request phải được người khác
review để đảm bảo tính kiểm soát chéo. Nhờ một Checker khác approve.

### Q: SQL thực thi bị timeout, phải làm sao?
**A**: Timeout được cấu hình theo từng database (1-300 giây).
- Kiểm tra SQL có quá phức tạp không (dùng AI Analysis)
- Chia nhỏ thành nhiều request riêng biệt
- Liên hệ Admin tăng timeout cho database (nếu cần thiết)

### Q: Data Query được approve nhưng tôi không thể execute?
**A**: Kiểm tra:
- Query có hết hạn (expires_at) chưa?
- Bạn có phải người tạo hoặc assigned user không?
- Database đích có đang active không?

### Q: Checker approve rồi nhưng SQL chưa thực thi?
**A**: Request yêu cầu **multi-level approval**. Kiểm tra tiến trình approval
(ví dụ: "1/2 approvals"). Cần thêm Checker khác approve.

### Q: Tôi là Checker nhưng không thấy request trong danh sách chờ duyệt?
**A**: Có thể vì:
- Request do chính bạn tạo (4-eyes — tự động ẩn)
- Request đã bị Cancel hoặc đã đủ approval
- Bạn chưa có role Checker (kiểm tra trong menu User ở Navbar)

### Q: ManageEngine CR không sync được metadata?
**A**: Kiểm tra:
- ManageEngine integration có được bật trong cấu hình server không?
- URL có đúng format không?
- Network từ SECS server đến ManageEngine có thông không?
- API credentials của ManageEngine còn hợp lệ không?

### Q: WebSocket thông báo không hoạt động?
**A**: Thử:
- Refresh trang (F5) — hệ thống tự reconnect
- Kiểm tra browser có block WebSocket không (proxy, firewall)
- Kiểm tra token còn hạn (nếu hết hạn → đăng nhập lại)

### Q: Schema browser không hiển thị tables?
**A**: Kiểm tra:
- Bạn có quyền trên database đó không?
- Database connection có đang active không?
- Network từ SECS server đến database đích có thông không?
- Schema name có đúng không? (PostgreSQL: `public`, MySQL: tên database)

### Q: Data Query approval — không ai approve được query của tôi?
**A**: Kiểm tra:
- Approval Matrix có rule cho database/schema/table bạn query không?
- Các Approver Units trong rule có user nào được gán không?
- Liên hệ Admin/Approver để kiểm tra cấu hình

---

## Phụ lục A: Trạng thái Request & Màu sắc

| Trạng thái | Badge | Ý nghĩa |
|------------|-------|----------|
| DRAFT | 🔵 Xám | Nháp — chưa gửi, có thể sửa |
| PENDING | 🟡 Vàng | Đang chờ Checker review |
| APPROVED | 🟢 Xanh lá | Đã duyệt (trước khi thực thi) |
| EXECUTED | ✅ Xanh đậm | Thực thi thành công |
| FAILED | 🔴 Đỏ | Thực thi lỗi |
| REJECTED | ⛔ Cam | Bị từ chối bởi Checker |
| CANCELLED | ⚪ Xám nhạt | Maker tự hủy |

---

## Phụ lục B: Keyboard Shortcuts trong SQL Editor

| Phím tắt | Chức năng |
|----------|-----------|
| `Ctrl+Enter` | Submit form / Execute |
| `Ctrl+S` | Lưu draft |
| `Ctrl+/` | Toggle comment |
| `Ctrl+Shift+F` | Format SQL |
| `Ctrl+Space` | Autocomplete |

---

## Phụ lục C: API Rate Limits

| Endpoint | Giới hạn | Scope |
|----------|----------|-------|
| `/auth/login` | 10 req/phút | Per IP |
| `/api/v1/requests/:id/analyze` | Configurable | Per User |
| Các endpoint khác | Không giới hạn | — |

---

## Phụ lục D: Liên hệ hỗ trợ

- **Vấn đề về quyền (role, permission)**: Liên hệ Admin hệ thống
- **Vấn đề kỹ thuật (bug, lỗi)**: Tạo ticket trên hệ thống nội bộ
- **Yêu cầu tính năng mới**: Gửi email cho team phát triển

---

*Tài liệu này được tạo tự động từ source code của SECS v1.0.*
*Cập nhật lần cuối: 2026-08-08*
