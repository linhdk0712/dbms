# Code Summary — Unit 4: Frontend

**Unit**: unit-4-frontend  
**Ngày**: 2026-07-11  
**Trạng thái**: ✅ COMPLETED — vite build pass (3.24s)

---

## Tổng quan

Unit 4 triển khai toàn bộ web app Svelte với DaisyUI, bao gồm: routing (hash-based SPA), auth flow (SSO callback), tất cả pages cho Maker/Checker/Admin, real-time notification qua WebSocket, và responsive UI.

---

## Files đã tạo / cập nhật

### Stores
| File | Mô tả |
|------|-------|
| `stores/auth.js` | Auth store (đã có từ Unit 1) — JWT + user info, localStorage persist |
| `stores/notifications.js` | WebSocket notification store — auto-connect, auto-reconnect, broadcast |

### Components
| File | Mô tả |
|------|-------|
| `components/Navbar.svelte` | Navigation bar với user dropdown, roles display, logout |
| `components/StatusBadge.svelte` | Color-coded status badge cho request statuses |
| `components/Toast.svelte` | Real-time notification toasts từ WebSocket |
| `components/AiAnalysisPanel.svelte` | AI SQL analysis panel — risk score, performance/security/best practices issues, cost estimation, suggested SQL |

### Pages
| File | Route | Role | Mô tả |
|------|-------|------|-------|
| `pages/Login.svelte` | `/login` | Public | SSO login redirect |
| `pages/AuthCallback.svelte` | `/auth/callback` | Public | OIDC callback, token extract, user fetch |
| `pages/Dashboard.svelte` | `/` | Any | Stats overview + quick links theo role |
| `pages/maker/NewRequest.svelte` | `/requests/new` | Maker | Form tạo SQL request (save draft / submit) |
| `pages/maker/MyRequests.svelte` | `/requests` | Maker | List requests + filter + actions (submit/cancel) |
| `pages/checker/PendingList.svelte` | `/approvals` | Checker | List requests chờ duyệt |
| `pages/checker/ReviewDetail.svelte` | `/approvals/:id` | Checker | SQL review + validate SQL + approve/reject (validate bắt buộc trước approve) |
| `pages/admin/Databases.svelte` | `/admin/databases` | Admin | CRUD databases, test connection |
| `pages/admin/Users.svelte` | `/admin/users` | Admin | Manage users + grant/revoke permissions |

### Root
| File | Thay đổi |
|------|----------|
| `App.svelte` | Router setup, auth guard, WebSocket connect/disconnect |
| `package.json` | Thêm `svelte-spa-router` dependency |

---

## Tech Stack

| Library | Version | Purpose |
|---------|---------|---------|
| Svelte | 4.2.x | Component framework |
| Vite | 5.4.x | Build tool + dev server |
| Tailwind CSS | 3.4.x | Utility-first CSS |
| DaisyUI | 4.12.x | UI component library |
| svelte-spa-router | 4.0.x | Hash-based SPA routing |

---

## Key Features

| Feature | Implementation |
|---------|----------------|
| SSO Login | Redirect → `/auth/login` → OIDC → callback with token in fragment |
| Auth Guard | Reactive check in App.svelte, redirect to `/login` if no token |
| API Client | Centralized `api/index.js` — auto JWT attach, 401 auto-logout |
| WebSocket | Auto-connect on login, auto-reconnect, toast notifications |
| SQL Validation | Checker phải validate SQL (EXPLAIN) thành công trước khi approve — hiển thị execution plan + performance warnings |
| AI SQL Analysis | AiAnalysisPanel trong ReviewDetail — phân tích AI cho risk score, performance, security, best practices, cost estimation, suggested SQL |
| Role-based UI | Dashboard shows quick links per role, pages enforce role via API |
| Responsive | DaisyUI grid + responsive breakpoints |
| Dark/Light theme | DaisyUI themes configured, toggle via `data-theme` |

---

## Build Output

```
dist/index.html                  0.45 kB │ gzip:  0.31 kB
dist/assets/index-Cp0r94HA.css  65.14 kB │ gzip: 10.04 kB
dist/assets/index-WGIK7yia.js   62.56 kB │ gzip: 20.24 kB
```

Total: ~128 KB (30 KB gzipped) — excellent cho SPA nội bộ.

---

## Routing Structure

```
Hash-based routing (svelte-spa-router)

#/login              → Login.svelte (public)
#/auth/callback      → AuthCallback.svelte (public)
#/                   → Dashboard.svelte (authenticated)
#/requests/new       → NewRequest.svelte (Maker)
#/requests           → MyRequests.svelte (Maker)
#/approvals          → PendingList.svelte (Checker)
#/approvals/:id      → ReviewDetail.svelte (Checker)
#/admin/databases    → Databases.svelte (Admin)
#/admin/users        → Users.svelte (Admin)
```

---

## Ghi chú

- Frontend KHÔNG chứa logic phân quyền — chỉ hiển thị theo data trả về từ API
- Mọi API call đi qua `api/index.js` — single source of truth
- WebSocket dùng token qua query param (vì WS không hỗ trợ custom headers)
- A11y warning: DaisyUI dropdown pattern dùng tabindex trên ul (accepted pattern)
