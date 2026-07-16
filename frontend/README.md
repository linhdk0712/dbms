# SECS Frontend

Giao diện web cho SQL Execution Control System — xây dựng bằng Svelte + Vite với DaisyUI.

## Tech Stack

| Thành phần | Công nghệ |
|-----------|-----------|
| Framework | Svelte 4 |
| Build Tool | Vite 5 |
| CSS | Tailwind CSS 3 + DaisyUI 4 |
| Code Editor | CodeMirror 6 |
| Router | svelte-spa-router |
| Package Manager | pnpm |

## Cấu trúc thư mục

```
src/
├── App.svelte              # Root component + routing
├── app.css                 # Global styles (Tailwind/DaisyUI)
├── api/                    # HTTP client wrappers
│   ├── index.js            # Base fetch config, token attach
│   └── endpoints.js        # API endpoint definitions
├── stores/                 # Svelte stores (global state)
│   ├── auth.js             # User session, role
│   └── notifications.js    # WebSocket notifications
├── components/             # Reusable UI components
│   ├── Navbar.svelte       # Navigation bar
│   ├── Sidebar.svelte      # Sidebar menu
│   ├── SqlEditor.svelte    # CodeMirror SQL editor
│   ├── JsonViewer.svelte   # JSON result viewer
│   ├── StatusBadge.svelte  # Badge trạng thái request
│   └── ...
└── pages/                  # Page-level views
    ├── Login.svelte        # Trang đăng nhập / SSO redirect
    ├── Dashboard.svelte    # Tổng hợp theo role
    ├── maker/              # Screens cho Maker role
    ├── checker/            # Screens cho Checker role
    └── admin/              # Screens cho Admin role
```

## Yêu cầu

- Node.js 22+
- pnpm 9+

## Cài đặt & Chạy

### 1. Cài dependencies

```bash
pnpm install
```

### 2. Chạy development server

```bash
pnpm dev
```

Mở browser tại http://localhost:5173

### 3. Build production

```bash
pnpm build
```

Output nằm trong thư mục `dist/`.

## Commands

```bash
pnpm install    # Cài dependencies
pnpm dev        # Dev server (http://localhost:5173)
pnpm build      # Build production
pnpm preview    # Preview bản build
pnpm lint       # Check format với Prettier
pnpm format     # Auto-format với Prettier
```

## Docker

```bash
docker build -t secs-frontend .
docker run -p 8088:80 secs-frontend
```

Container sử dụng Nginx để:
- Serve static files (với caching 1 năm cho hashed assets)
- Reverse proxy `/api/`, `/auth/`, `/ws` đến backend
- SPA fallback routing (mọi route trả về `index.html`)

## Quy ước code

- Không gọi API trực tiếp trong component — phải qua `src/api/`
- Không chứa logic phân quyền — chỉ hiển thị theo data từ API
- Mỗi component trong file `.svelte` riêng
- Prettier trước commit: `pnpm lint`
- Không hardcode URL — dùng constants hoặc env vars

## Cấu hình API

Frontend kết nối backend qua:
- **Development**: Vite proxy (cấu hình trong `vite.config.js`)
- **Production (Docker)**: Nginx reverse proxy (xem `nginx.conf`)

Không cần cấu hình URL backend riêng — routing được xử lý tự động.
