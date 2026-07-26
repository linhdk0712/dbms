# Code Summary — Unit 2: Authentication

**Unit**: unit-2-auth  
**Ngày**: 2026-07-11  
**Trạng thái**: ✅ COMPLETED — build pass, 35/35 tests pass

---

## Tổng quan

Unit 2 triển khai toàn bộ authentication stack: OIDC/SSO login flow với PKCE, JWT generation/verification, auth middleware, và RBAC extractors. User được tự động tạo/cập nhật khi đăng nhập lần đầu qua SSO.

---

## Files đã tạo / cập nhật

### Backend — Auth Module (MỚI)
| File | Mô tả |
|------|-------|
| `auth/mod.rs` | Module exports: jwt, oidc, middleware, extractors |
| `auth/jwt.rs` | JWT generate/verify, Claims struct, 8h expiry, HS256 |
| `auth/oidc.rs` | OIDC client init, PKCE flow, callback handler, token verification |
| `auth/middleware.rs` | `require_auth` — extract Bearer token, verify, inject Claims |
| `auth/extractors.rs` | `RequireAuth`, `RequireAdmin`, `RequireMaker`, `RequireChecker` |

### Backend — API Auth Handlers (MỚI)
| File | Mô tả |
|------|-------|
| `api/auth.rs` | `/auth/login`, `/auth/callback`, `/auth/me`, `/auth/logout` |

### Backend — DB Layer (MỚI)
| File | Mô tả |
|------|-------|
| `db/mod.rs` | Module exports |
| `db/users.rs` | `upsert_from_oidc`, `get_user_roles`, `find_by_subject`, `is_user_active`, `list_all`, `get_by_id`, `update_active_status`, `replace_roles` |

### Backend — Updated
| File | Thay đổi |
|------|----------|
| `main.rs` | Thêm `mod auth`, `mod db`, init OIDC client (graceful fallback) |
| `state.rs` | Thêm `oidc_client: Option<Arc<CoreClient>>`, `oidc_sessions: OidcSessionStore` |
| `api/mod.rs` | Thêm auth routes, split public vs protected routes, auth middleware |
| `Cargo.toml` | Thêm `async-trait`, `url` |

---

## Kiến trúc Auth Flow

```
Browser → GET /auth/login → Redirect đến OIDC Provider (Keycloak)
                                        ↓
                              User đăng nhập SSO
                                        ↓
Browser ← GET /auth/callback?code=xxx&state=yyy
                                        ↓
Backend: verify CSRF → exchange code → verify ID token → upsert user → gen JWT
                                        ↓
Browser ← Redirect: frontend/#/auth/callback?token=<JWT>
                                        ↓
Frontend: lưu JWT vào localStorage, gửi kèm Bearer token mọi request
```

## OIDC Security Features

| Feature | Implementation |
|---------|----------------|
| PKCE (S256) | `PkceCodeChallenge::new_random_sha256()` |
| CSRF Protection | `CsrfToken` lưu server-side, verify trước exchange |
| Nonce Verification | Verify nonce trong ID token claims |
| Access Token Hash | Verify `at_hash` chống substitution attack |
| Force Re-login | `prompt=login` — buộc user đăng nhập lại, không dùng IdP session cũ |
| Graceful Startup | OIDC client init có fallback — server vẫn chạy nếu IdP down |

## JWT Details

| Property | Value |
|----------|-------|
| Algorithm | HS256 |
| Expiry | 8 giờ (security rule) |
| Issuer | `secs-backend` |
| Custom Claims | `user_id: Uuid`, `email`, `display_name`, `roles: Vec<Role>` |
| Validation | exp, iss, signature |
| Leeway | 60s (jsonwebtoken default) |

## Router Structure

```
Public (no auth):
  GET  /health         → health_check
  GET  /auth/login     → OIDC redirect
  GET  /auth/callback  → OIDC callback + JWT issue

Protected (Bearer token required):
  GET  /auth/me        → current user info
  POST /auth/logout    → placeholder (stateless JWT)
```

## RBAC Extractors

| Extractor | Yêu cầu |
|-----------|----------|
| `RequireAuth(claims)` | Token hợp lệ (bất kỳ role) |
| `RequireAdmin(claims)` | Token + role ADMIN |
| `RequireMaker(claims)` | Token + role MAKER |
| `RequireChecker(claims)` | Token + role CHECKER |

---

## Kiểm thử

**35/35 tests pass**, gồm:
- JWT: generate/verify, wrong secret, tampered, expired (>leeway), roles preserved, issuer, expiry bounds
- Middleware: valid Bearer, missing header, wrong scheme, empty token
- Extractors: Claims in extensions, missing claims, role checks (pass/fail cho từng role)
- PBT: 2 tests từ Unit 1 vẫn pass

---

## Dependencies thêm mới

| Crate | Version | Lý do |
|-------|---------|-------|
| `async-trait` | 0.1 | axum-core 0.4.5 dùng `#[async_trait]` cho `FromRequestParts` |
| `url` | 2 | `Url` type dùng bởi openidconnect |

---

## Ghi chú

- **Session store**: In-memory `HashMap<String, OidcSession>` — phù hợp cho single-instance app nội bộ. Production multi-instance cần Redis.
- **Logout**: Stateless JWT — logout chỉ xóa token phía client. Token blacklist là enhancement sau nếu cần.
- **Graceful OIDC init**: Nếu OIDC provider down lúc startup, server vẫn chạy nhưng `/auth/login` trả 500. Cho phép restart khi IdP recover.
- **Dead code warnings**: 18 warnings (dead_code) — bình thường, sẽ được wire vào Unit 3 (Core Business).

---

## Next: Unit 3 — Core Business Logic

- `db/` layer: requests, audit, databases
- `executor/` — DatabaseExecutor trait + implementations
- `api/` handlers: requests, approvals, databases, users, dashboard
- `ws/` — WebSocket notification
