# Code Generation Plan — Unit 2: Authentication

**Unit**: unit-2-auth  
**Date**: 2026-07-11  
**Status**: Executing

---

## Unit Context

- **Workspace Root**: `d:\Apps\dbms\`
- **Backend path**: `d:\Apps\dbms\backend\`
- **Stories covered**: OIDC/SSO login, JWT generate/verify, auth middleware, RBAC extractors
- **Dependencies**: Unit 1 (Foundation)

---

## Files đã có (viết trước từ Unit 1 nhưng chưa link vào crate)

- `backend/src/auth/jwt.rs` — generate/verify JWT, Claims struct, tests
- `backend/src/auth/oidc.rs` — OIDC client init, build auth URL, handle callback

---

## Steps

### Step 1: Link Auth Module vào Crate
- [x] Tạo `backend/src/auth/mod.rs` — export jwt, oidc, middleware, extractors
- [x] Thêm `mod auth;` vào `main.rs`

### Step 2: Auth Middleware — JWT Verification
- [x] Tạo `backend/src/auth/middleware.rs` — extract Bearer token, verify, inject Claims vào extensions

### Step 3: RBAC Extractors
- [x] Tạo `backend/src/auth/extractors.rs` — `RequireAuth`, `RequireMaker`, `RequireChecker`, `RequireAdmin`

### Step 4: API Auth Handlers
- [x] Tạo `backend/src/api/auth.rs` — `GET /auth/login`, `GET /auth/callback`, `POST /auth/logout`, `GET /auth/me`
- [x] Thêm `pub mod auth;` vào `api/mod.rs`
- [x] Wire auth routes vào router

### Step 5: Session Store cho OIDC Flow
- [x] Implement in-memory session store cho PKCE verifier + CSRF token + nonce (phục vụ callback)
- [x] Thêm session store vào AppState

### Step 6: DB Layer — User Upsert
- [x] Tạo `backend/src/db/mod.rs` + `backend/src/db/users.rs` — find_or_create user sau OIDC login, fetch roles

### Step 7: Update AppState
- [x] Thêm OIDC client vào AppState (init lúc startup)
- [x] Thêm session store vào AppState

### Step 8: Update main.rs
- [x] Init OIDC client lúc startup
- [x] Graceful fallback nếu OIDC provider không available

### Step 9: Tests
- [x] Test auth middleware — valid/invalid/missing token
- [x] Test RBAC extractors — đúng role pass, sai role reject
- [ ] Test API handlers — login redirect, callback flow, me endpoint

### Step 10: Build & Verify
- [x] `cargo +stable-x86_64-pc-windows-gnu build` pass
- [x] `cargo +stable-x86_64-pc-windows-gnu test` pass (35/35)

### Step 11: Code Summary Documentation
- [x] Tạo `aidlc-docs/construction/unit-2-auth/code/summary.md`

---

## File Output Summary

```
d:\Apps\dbms\backend\src\
├── auth/
│   ├── mod.rs          (MỚI)
│   ├── jwt.rs          (đã có — giữ nguyên)
│   ├── oidc.rs         (đã có — giữ nguyên)
│   ├── middleware.rs   (MỚI)
│   └── extractors.rs  (MỚI)
├── api/
│   ├── mod.rs          (CẬP NHẬT — thêm auth routes)
│   └── auth.rs         (MỚI)
├── db/
│   ├── mod.rs          (MỚI)
│   └── users.rs        (MỚI)
├── main.rs             (CẬP NHẬT — thêm mod auth, init OIDC)
└── state.rs            (CẬP NHẬT — thêm OIDC client, session store)
```
