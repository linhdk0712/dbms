# Cấu trúc dự án

```
dbms/
├── backend/                        # Rust backend
│   ├── src/
│   │   ├── main.rs                 # Entry point, khởi tạo server
│   │   ├── config.rs               # Load cấu hình từ env/file
│   │   ├── error.rs                # Custom error types (thiserror)
│   │   │
│   │   ├── api/                    # HTTP handlers (axum routes)
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs             # SSO callback, login, logout
│   │   │   ├── requests.rs         # CRUD cho SQL requests (Maker)
│   │   │   ├── approvals.rs        # Review, approve, reject (Checker)
│   │   │   ├── databases.rs        # Quản lý database connections
│   │   │   ├── users.rs            # Quản lý users và roles
│   │   │   └── dashboard.rs        # Dữ liệu tổng hợp cho dashboard
│   │   │
│   │   ├── executor/               # Thực thi SQL trên database đích
│   │   │   ├── mod.rs              # DatabaseExecutor trait
│   │   │   ├── postgres.rs         # PostgreSQL driver impl
│   │   │   ├── oracle.rs           # Oracle driver impl
│   │   │   ├── mysql.rs            # MySQL driver impl
│   │   │   └── mongodb.rs          # MongoDB driver impl
│   │   │
│   │   ├── auth/                   # SSO/OIDC logic
│   │   │   ├── mod.rs
│   │   │   ├── oidc.rs             # OpenID Connect flow
│   │   │   └── jwt.rs              # JWT tạo và xác thực
│   │   │
│   │   ├── models/                 # Data models / domain types
│   │   │   ├── mod.rs
│   │   │   ├── user.rs             # User, Role
│   │   │   ├── request.rs          # SqlRequest, RequestStatus
│   │   │   ├── approval.rs         # Approval, Comment
│   │   │   └── database.rs         # DatabaseConnection config
│   │   │
│   │   ├── db/                     # Database access layer (PostgreSQL nội bộ)
│   │   │   ├── mod.rs
│   │   │   ├── migrations/         # sqlx migration files
│   │   │   ├── users.rs
│   │   │   ├── requests.rs
│   │   │   └── audit.rs            # Audit log queries
│   │   │
│   │   └── ws/                     # WebSocket handlers
│   │       └── mod.rs              # Realtime notification
│   │
│   ├── tests/                      # Integration tests
│   └── Cargo.toml
│
├── frontend/                       # Svelte frontend
│   ├── src/
│   │   ├── App.svelte              # Root component + routing
│   │   ├── app.css                 # Global styles (Tailwind/DaisyUI)
│   │   │
│   │   ├── api/                    # HTTP client wrappers
│   │   │   ├── index.js            # Base fetch config, token attach
│   │   │   ├── requests.js         # API calls cho SQL requests
│   │   │   ├── approvals.js        # API calls cho approvals
│   │   │   └── databases.js        # API calls cho database management
│   │   │
│   │   ├── stores/                 # Svelte stores (global state)
│   │   │   ├── auth.js             # User session, role
│   │   │   └── notifications.js    # WebSocket notifications
│   │   │
│   │   ├── components/             # Reusable UI components
│   │   │   ├── SqlEditor.svelte    # Code editor cho SQL input
│   │   │   ├── StatusBadge.svelte  # Badge trạng thái request
│   │   │   ├── RequestCard.svelte  # Card hiển thị một request
│   │   │   └── Navbar.svelte
│   │   │
│   │   └── pages/                  # Page-level views
│   │       ├── Login.svelte        # Trang đăng nhập / SSO redirect
│   │       ├── Dashboard.svelte    # Tổng hợp theo role
│   │       ├── maker/
│   │       │   ├── NewRequest.svelte    # Tạo SQL request mới
│   │       │   └── MyRequests.svelte    # Danh sách request của Maker
│   │       ├── checker/
│   │       │   ├── PendingList.svelte   # Danh sách chờ duyệt
│   │       │   └── ReviewDetail.svelte  # Chi tiết review + approve
│   │       └── admin/
│   │           ├── Users.svelte         # Quản lý users
│   │           └── Databases.svelte     # Quản lý database connections
│   │
│   ├── public/
│   ├── index.html
│   ├── vite.config.js
│   └── package.json
│
├── .kiro/
│   └── steering/
├── .env.example                    # Template biến môi trường
├── docker-compose.yml              # PostgreSQL + app cho local dev
├── .gitignore
└── README.md
```

## Quy tắc tổ chức code

- **API handlers phải mỏng**: handler chỉ parse request, gọi service/db layer, trả response — không chứa business logic
- **`executor/` là interface duy nhất** để chạy SQL trên database đích — không gọi driver trực tiếp từ nơi khác
- **`db/` chỉ dùng cho PostgreSQL nội bộ** (lưu trạng thái ứng dụng), không nhầm với `executor/`
- **Frontend không chứa logic phân quyền** — chỉ hiển thị theo dữ liệu trả về từ backend
- **Mọi thao tác thực thi SQL** phải được ghi vào audit log trước khi thực thi
- **Credentials database đích** không được log ra console hoặc trả về trong API response
