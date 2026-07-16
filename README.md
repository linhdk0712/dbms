# SECS — SQL Execution Control System

Hệ thống quản lý và kiểm soát thực thi câu lệnh SQL trên môi trường production, áp dụng workflow **Maker-Checker** (4-eyes principle) cho các công ty tài chính.

## Tổng quan

SECS giải quyết vấn đề nhân viên vận hành thao tác trực tiếp với database production mà không qua kiểm soát, dẫn đến rủi ro mất mát hoặc sai lệch dữ liệu.

### Tính năng chính

- **Workflow Maker-Checker**: Maker soạn SQL → Checker review & approve → Hệ thống tự động thực thi
- **Multi-database**: Hỗ trợ PostgreSQL, MySQL, MongoDB, Oracle
- **SSO Integration**: Đăng nhập qua OAuth2/OIDC (Keycloak hoặc IdP công ty)
- **RBAC**: Phân quyền Admin / Maker / Checker theo từng database
- **Audit Trail**: Ghi log đầy đủ mọi thao tác (ai, khi nào, kết quả)
- **Realtime Notifications**: WebSocket thông báo khi có request mới hoặc được approve
- **SQL Analysis**: Tự động phát hiện DDL nguy hiểm (DROP, TRUNCATE, ALTER)
- **Credential Encryption**: Mã hóa AES-256-GCM cho thông tin kết nối database đích

## Tech Stack

| Layer | Công nghệ |
|-------|-----------|
| Backend | Rust, Axum, Tokio, SQLx |
| Frontend | Svelte 4, Vite, Tailwind CSS, DaisyUI |
| Database | PostgreSQL 16 |
| Auth | Keycloak (OIDC), JWT |
| Containerization | Docker, Docker Compose |

## Quick Start (Docker Compose)

### Yêu cầu

- Docker 24+
- Docker Compose v2

### Chạy toàn bộ hệ thống

```bash
# Clone và chạy
docker compose up -d

# Kiểm tra services
docker compose ps
```

Services sẽ khởi động:

| Service | URL | Mô tả |
|---------|-----|--------|
| Frontend | http://localhost:8088 | Giao diện web |
| Backend API | http://localhost:3000 | REST API + WebSocket |
| Keycloak | http://localhost:8080 | OIDC Provider (admin/admin) |
| PostgreSQL | localhost:5432 | Database nội bộ |

### Dừng hệ thống

```bash
docker compose down          # Dừng, giữ data
docker compose down -v       # Dừng, xoá data
```

## Development Setup

### Yêu cầu

- Rust stable (1.88+)
- Node.js 22+
- pnpm 9+
- PostgreSQL 16+
- (Optional) Keycloak hoặc OIDC provider khác

### 1. Cấu hình environment

```bash
cp .env.example .env
# Chỉnh sửa .env phù hợp
```

### 2. Khởi động database & Keycloak

```bash
docker compose up postgres keycloak -d
```

### 3. Chạy backend

```bash
cd backend
sqlx migrate run
cargo run
```

Backend chạy tại http://localhost:3000

### 4. Chạy frontend

```bash
cd frontend
pnpm install
pnpm dev
```

Frontend chạy tại http://localhost:5173

## Cấu trúc dự án

```
dbms/
├── backend/                # Rust backend (Axum)
│   ├── src/                # Source code
│   ├── migrations/         # SQLx migrations
│   ├── Dockerfile          # Multi-stage build
│   └── Cargo.toml
├── frontend/               # Svelte frontend
│   ├── src/                # Source code
│   ├── Dockerfile          # Multi-stage build (Nginx)
│   ├── nginx.conf          # Nginx config (proxy + SPA)
│   └── package.json
├── docker-compose.yml      # Local dev environment
├── .env.example            # Template biến môi trường
└── README.md
```

## Roles & Permissions

| Role | Quyền |
|------|-------|
| **Admin** | Quản lý users, database connections, cấu hình hệ thống |
| **Maker** | Soạn và gửi SQL request. Không thể tự approve |
| **Checker** | Review, approve/reject request. Không thể approve request mình tạo |

Một user có thể có nhiều role trên nhiều database khác nhau.

## Workflow

```
DRAFT → PENDING → APPROVED → EXECUTED
                ↓         ↓
             REJECTED   FAILED
             
PENDING → CANCELLED (Maker hủy)
```

1. **Maker** tạo SQL request (DRAFT)
2. **Maker** submit request (→ PENDING)
3. **Checker** review và approve/reject
4. Nếu APPROVED → hệ thống tự động thực thi SQL trên database đích
5. Kết quả: EXECUTED (thành công) hoặc FAILED (lỗi)

## Biến môi trường

Xem [.env.example](.env.example) cho danh sách đầy đủ.

| Biến | Mô tả | Default |
|------|--------|---------|
| `DATABASE_URL` | PostgreSQL connection string | — |
| `JWT_SECRET` | JWT signing secret (min 32 chars) | — |
| `ENCRYPTION_KEY` | AES-256 key, 64 hex chars | — |
| `OIDC_ISSUER_URL` | OIDC provider URL | — |
| `OIDC_CLIENT_ID` | OIDC client ID | — |
| `OIDC_CLIENT_SECRET` | OIDC client secret | — |
| `FRONTEND_URL` | Frontend URL cho CORS | `http://localhost:5173` |
| `SQL_EXECUTION_TIMEOUT_SECS` | Timeout thực thi SQL | `30` |
| `PORT` | Backend server port | `3000` |

## Bảo mật

- Credentials database đích được encrypt bằng AES-256-GCM
- JWT token hết hạn sau 8 giờ
- 4-eyes principle: Maker ≠ Checker cho cùng một request
- Kết nối TLS tới database đích
- Rate limiting trên API
- Security headers (X-Frame-Options, X-Content-Type-Options, etc.)
- Non-root user trong Docker containers

## Chi tiết thêm

- [Backend README](./backend/README.md) — Chi tiết API, cấu trúc code, commands
- [Frontend README](./frontend/README.md) — Chi tiết UI, components, dev workflow
