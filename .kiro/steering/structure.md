# Cấu trúc dự án

```
dbms/
├── backend/                        # Rust backend
│   ├── src/
│   │   ├── main.rs                 # Entry point, dual logging (stdout + file), startup
│   │   ├── config.rs               # Load cấu hình từ env (LOG_DIR, OIDC session params)
│   │   ├── error.rs                # AppError enum + IntoResponse (4xx/5xx)
│   │   ├── state.rs                # AppState { db, config, notification_tx, oidc_client, oidc_sessions }
│   │   │
│   │   ├── api/                    # HTTP handlers (axum routes) — MỎNG
│   │   │   ├── mod.rs              # build_router(), CORS, security headers, body limit
│   │   │   ├── auth.rs             # OIDC login/callback, logout, me, session cleanup
│   │   │   ├── requests.rs         # Maker: create, list, detail, update, submit, cancel, clone
│   │   │   ├── approvals.rs        # Checker: list_pending, detail, approve, reject + SQL execution
│   │   │   ├── audit.rs            # GET /requests/:id/audit (access control)
│   │   │   ├── databases.rs        # Admin: CRUD, test, test-new, credentials + Maker: my-permissions
│   │   │   ├── users.rs            # Admin: list, detail, update, permissions CRUD
│   │   │   ├── dashboard.rs        # Role-aware stats (Admin/Checker/Maker)
│   │   │   ├── health.rs           # GET /health (DB pool check)
│   │   │   └── ip_extractor.rs     # extract_client_ip (X-Forwarded-For priority)
│   │   │
│   │   ├── executor/               # Thực thi SQL trên database đích
│   │   │   ├── mod.rs              # DatabaseExecutor trait, execute_sql(), test_db_connection()
│   │   │   ├── encryption.rs       # AES-256-GCM encrypt/decrypt credentials
│   │   │   ├── postgres.rs         # PgExecutor (sslmode=require, transaction)
│   │   │   ├── mysql.rs            # MySqlExecutor (ssl-mode=REQUIRED, transaction)
│   │   │   ├── oracle.rs           # OracleExecutor (TCPS, spawn_blocking)
│   │   │   ├── mongodb.rs          # MongoExecutor (tls=true, JSON commands, session TX)
│   │   │   └── sql_analyzer.rs     # DDL detection (Warning/Danger severity)
│   │   │
│   │   ├── auth/                   # Authentication & Authorization
│   │   │   ├── mod.rs
│   │   │   ├── oidc.rs             # OpenID Connect flow (PKCE + CSRF + at_hash)
│   │   │   ├── jwt.rs              # Claims, generate/verify (HS256, 8h, issuer)
│   │   │   ├── middleware.rs       # require_auth (Bearer token → Claims extension)
│   │   │   ├── extractors.rs       # RequireAuth, RequireAdmin, RequireMaker, RequireChecker
│   │   │   └── rate_limit.rs       # 10 req/min per IP on /auth/login
│   │   │
│   │   ├── models/                 # Domain types (no I/O, no side effects)
│   │   │   ├── mod.rs
│   │   │   ├── user.rs             # User, Role, UserDbPermission
│   │   │   ├── request.rs          # SqlRequest, RequestStatus + state machine
│   │   │   ├── database.rs         # DatabaseConnection, DbType, response DTOs
│   │   │   ├── approval.rs         # AuditLog, AuditAction (incl. admin actions)
│   │   │   ├── notification.rs     # Notification, NotificationType
│   │   │   └── request_approval.rs # RequestApproval (multi-level tracking)
│   │   │
│   │   ├── db/                     # Database access layer (PostgreSQL nội bộ)
│   │   │   ├── mod.rs
│   │   │   ├── users.rs            # upsert_from_oidc, get_roles, first-user bootstrap
│   │   │   ├── requests.rs         # CRUD, state transitions, multi-level approval
│   │   │   ├── audit.rs            # insert, get_by_request_id, insert_admin_action
│   │   │   ├── databases.rs        # CRUD, list_by_user_permission, update_credentials
│   │   │   └── permissions.rs      # check/grant/revoke/list permissions
│   │   │
│   │   └── ws/                     # WebSocket handlers
│   │       └── mod.rs              # ws_handler, role-based notification filtering
│   │
│   ├── migrations/                 # 8 SQL migration files
│   └── Cargo.toml
│
├── frontend/                       # Svelte 4 + Vite 5 frontend
│   ├── src/
│   │   ├── main.js                 # Svelte app mount
│   │   ├── app.css                 # Tailwind + DaisyUI
│   │   ├── App.svelte              # Root: svelte-spa-router, auth guard, WS connect
│   │   │
│   │   ├── api/                    # HTTP client wrappers
│   │   │   ├── index.js            # apiFetch() + grouped API modules
│   │   │   └── endpoints.js        # All endpoint path constants
│   │   │
│   │   ├── stores/                 # Svelte stores (global state)
│   │   │   ├── auth.js             # authStore { token, user, login(), logout() }
│   │   │   └── notifications.js    # WebSocket store, auto-reconnect
│   │   │
│   │   ├── components/             # Reusable UI components
│   │   │   ├── Navbar.svelte       # Logo, theme toggle, user dropdown
│   │   │   ├── Sidebar.svelte      # Role-based navigation
│   │   │   ├── SqlEditor.svelte    # CodeMirror 6 (SQL + JSON)
│   │   │   ├── StatusBadge.svelte  # Status color badge
│   │   │   └── Toast.svelte        # WS notification toasts
│   │   │
│   │   └── pages/                  # Page-level views
│   │       ├── Login.svelte            # SSO redirect
│   │       ├── AuthCallback.svelte     # OIDC callback handler
│   │       ├── Dashboard.svelte        # Role-aware stats + quick links
│   │       ├── maker/
│   │       │   ├── NewRequest.svelte   # Form (SQL/JSON auto-detect)
│   │       │   ├── MyRequests.svelte   # Paginated list + actions
│   │       │   └── RequestDetail.svelte # Detail + edit + audit timeline + clone
│   │       ├── checker/
│   │       │   ├── PendingList.svelte  # Filterable + multi-level progress
│   │       │   └── ReviewDetail.svelte # SQL + DDL analysis + approve/reject
│   │       └── admin/
│   │           ├── Users.svelte        # Roles + permissions + activate/deactivate
│   │           └── Databases.svelte    # CRUD + test connection
│   │
│   ├── public/
│   ├── index.html
│   ├── vite.config.js
│   └── package.json
│
├── .kiro/
│   └── steering/
├── .env.example                    # Template biến môi trường (incl. LOG_DIR)
├── docker-compose.yml              # PostgreSQL + Keycloak + Backend + Frontend
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
