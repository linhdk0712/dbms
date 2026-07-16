# Workflow Plan — SECS Implementation

**Date**: 2026-07-11  
**Status**: Proposed — chờ approval

---

## Tổng quan

Dự án Greenfield với tài liệu thiết kế đầy đủ. Bỏ qua các giai đoạn Requirements Analysis chuyên sâu, User Stories, và Application Design (đã có trong steering docs). Tập trung vào Construction phase.

---

## Workflow Visualization

```
INCEPTION (hoàn thành)
├── [x] Workspace Detection
├── [-] Reverse Engineering     (SKIPPED — Greenfield)
├── [x] Requirements Analysis   (COMPLETED — review docs)
├── [-] User Stories            (SKIPPED — docs đầy đủ)
├── [x] Workflow Planning       (IN PROGRESS)
├── [-] Application Design      (SKIPPED — system-design.md đã có)
└── [ ] Units Generation        → xác định units of work

CONSTRUCTION
├── Unit 1: Foundation (DB migrations + core models + main.rs)
│   ├── Functional Design
│   ├── NFR Requirements
│   ├── Code Generation
│   └── Build & Test
├── Unit 2: Auth (OIDC + JWT + middleware)
│   ├── Functional Design
│   ├── Code Generation
│   └── Build & Test
├── Unit 3: Core Business (db/ layer + executor/ + API handlers)
│   ├── Functional Design
│   ├── NFR Design
│   ├── Code Generation
│   └── Build & Test
└── Unit 4: Frontend (Svelte + DaisyUI)
    ├── Functional Design
    ├── Code Generation
    └── Build & Test

OPERATIONS (Placeholder)
```

---

## Units of Work

### Unit 1 — Foundation
**Scope**: Database migrations, Rust models, AppState, main.rs skeleton  
**Dependency**: None — bắt đầu ngay  
**Output**: Running server với `/health` endpoint, migrations chạy OK

### Unit 2 — Authentication
**Scope**: OIDC/SSO flow, JWT generate/verify, auth middleware  
**Dependency**: Unit 1 (cần DB pool, AppState)  
**Output**: Login flow hoạt động với Keycloak local, JWT được verify mỗi request

### Unit 3 — Core Business Logic
**Scope**: db/ layer, executor/ (PostgreSQL, MySQL, Oracle, MongoDB), API handlers (requests, approvals, databases, users, dashboard), WebSocket  
**Dependency**: Unit 1 + Unit 2  
**Output**: Toàn bộ business workflow hoạt động, có thể submit/approve/execute request

### Unit 4 — Frontend
**Scope**: Svelte app với tất cả pages (Login, Dashboard, Maker, Checker, Admin), stores, api/ module, components  
**Dependency**: Unit 3 (cần API endpoints)  
**Output**: Web app đầy đủ chức năng

---

## Depth Levels

| Stage | Unit 1 | Unit 2 | Unit 3 | Unit 4 |
|-------|--------|--------|--------|--------|
| Functional Design | Standard | Standard | Comprehensive | Standard |
| NFR Requirements | Minimal | Minimal | Standard | Minimal |
| NFR Design | N/A | N/A | Standard | N/A |
| Infrastructure Design | N/A | N/A | N/A | N/A |
| Code Generation | Standard | Standard | Comprehensive | Standard |

---

## Extension Compliance Plan

### Security Rules (blocking)
- SECURITY-04 (HTTP headers): address trong Unit 1 (tower-http middleware)
- SECURITY-10 (Supply chain): thêm `cargo audit` vào build instructions
- SECURITY-14 (Alerting): document trong Build & Test stage

### Resiliency Rules (directional)
- RESILIENCY-02 (RTO/RPO): Pilot Light aligns với `system-design.md` §3.2 (RTO 30min, RPO 1h)
- RESILIENCY-03 (Change Management): hỏi user trong NFR Requirements
- RESILIENCY-04 (CI/CD): hỏi user trong NFR Requirements
- RESILIENCY-08 (Multi-zone): N/A cho nội bộ app, single-region acceptable

### PBT Rules (blocking)
- PBT-09 (Framework): `proptest` — thêm vào Cargo.toml trong Unit 1
- PBT-01 (Property Identification): thực hiện trong mỗi Functional Design stage
- PBT-02 đến PBT-10: thực hiện trong Code Generation của từng Unit

---

## Timeline ước tính

| Unit | Thời gian (1 dev) |
|------|------------------|
| Unit 1: Foundation | 3-5 ngày |
| Unit 2: Auth | 2-3 ngày |
| Unit 3: Core | 6-8 ngày |
| Unit 4: Frontend | 5-7 ngày |
| **Tổng** | **~4 tuần** |

---

## Câu hỏi cần người dùng xác nhận

1. Có muốn thực thi theo thứ tự Unit 1 → 2 → 3 → 4 như đề xuất không?
2. Unit 4 (Frontend) có thể bắt đầu song song với Unit 3 sau khi Unit 2 xong không?
3. Có muốn thêm giai đoạn Build & Test tổng hợp ở cuối không?
