# AI-DLC State Tracking — SECS

## Project Information
- **Project Type**: Greenfield (tài liệu thiết kế đã có, chưa có code)
- **Start Date**: 2026-07-11T00:00:00Z
- **Current Stage**: CONSTRUCTION — Build & Test

## Workspace State
- **Existing Code**: No (chưa có source code application)
- **Existing Design Docs**: Yes (.kiro/steering/ với đầy đủ tài liệu)
- **Reverse Engineering Needed**: No (greenfield)
- **Workspace Root**: d:\Apps\dbms\

## Code Location Rules
- **Application Code**: d:\Apps\dbms\ (NEVER in aidlc-docs/)
- **Documentation**: aidlc-docs/ only

## Technical Decisions
- **Datetime Library**: `chrono 0.4` (đã cập nhật implementation-plan.md, libs-sqlx.md)
- **Claims Struct**: Có cả `sub: String` (JWT standard) VÀ `user_id: Uuid` (type-safe) (đã cập nhật libs-auth.md)
- **Oracle v1**: Implement đầy đủ với `oracle` crate native
- **async-trait**: Không dùng (Rust ≥ 1.75 hỗ trợ async fn in traits natively)
- **PBT Framework**: `proptest` crate cho Rust

## Extension Configuration
| Extension | Status | Enforcement |
|-----------|--------|-------------|
| Security Baseline | ✅ ENABLED | Blocking (15 rules) |
| Resiliency Baseline | ✅ ENABLED | Blocking — directional (15 rules) |
| Property-Based Testing | ✅ ENABLED | Blocking (10 rules) |

## Stage Progress
| Stage | Status | Notes |
|-------|--------|-------|
| Workspace Detection | ✅ COMPLETED | Greenfield, docs sẵn có |
| Reverse Engineering | ⏭️ SKIPPED | Greenfield project |
| Requirements Analysis | ✅ COMPLETED | Đã review docs, resolve conflicts, tạo requirements.md |
| User Stories | ⏭️ SKIPPED | Tài liệu đã có đầy đủ, không cần thêm user stories |
| Workflow Planning | ✅ COMPLETED | Approved, 4 units xác định |
| Application Design | ⏭️ SKIPPED | system-design.md đã có đầy đủ component design |
| Units Generation | ✅ COMPLETED | units-of-work.md với 4 units |

## Construction Progress
| Unit | Stage | Status | Notes |
|------|-------|--------|-------|
| Unit 1: Foundation | Functional Design | ✅ COMPLETED | domain-entities, business-rules, business-logic-model |
| Unit 1: Foundation | Sequence Diagrams | ✅ COMPLETED | 12 diagrams Mermaid |
| Unit 1: Foundation | Code Generation | ✅ COMPLETED | Build PASS, 14/14 tests PASS |
| Unit 2: Auth | Code Generation | ✅ COMPLETED | Build PASS, 35/35 tests PASS |
| Unit 3: Core Business | Code Generation | ✅ COMPLETED | Build PASS, 41/41 tests PASS |
| Unit 4: Frontend | Code Generation | ✅ COMPLETED | Build PASS (vite build) |

## Data Query Permission Feature
- **Status**: IN PROGRESS — backend API handlers created
- **Migration**: `010_create_data_queries.sql` ✅
- **Design Doc**: `aidlc-docs/construction/data-query-permission-best-practices.md` ✅
- **Tables Created**: data_query_requests, data_query_assigned_users, data_query_approval_rules, data_query_approval_levels, data_query_approval_tracking, data_query_executions
- **Migration 011**: `approver_units` (admin-defined), `user_approver_units` (user↔unit). Bỏ CHECK constraint cứng trên `data_query_approval_levels.approver_unit` → giá trị linh hoạt tham chiếu từ bảng `approver_units`
- **Backend API Handlers**: ✅
  - `api/data_queries.rs` — create, list, get_detail, update, submit, execute, list_executions
  - `api/data_query_approvals.rs` — list_pending, approve, reject
  - `api/data_query_rules.rs` — list_rules, create_rule, update_rule, delete_rule
- **Routes wired** in `api/mod.rs` ✅
- **Remaining**: DB layer functions (`db/data_queries.rs`, `db/data_query_approvals.rs`, `db/data_query_rules.rs`), models (`models/data_query.rs`), executor read-only method, frontend pages

## Build Environment
- **Toolchain**: `stable-x86_64-pc-windows-gnu` (máy không có MSVC linker)
- **C Compiler**: WinLibs MinGW-w64 gcc 16.1.0 (cần cho `ring` — dep của rustls/jsonwebtoken)
- **Build**: `cargo +stable-x86_64-pc-windows-gnu build`
- **Test**: `cargo +stable-x86_64-pc-windows-gnu test`

## Documents Updated
- `domain.md` — thêm CANCELLED vào state diagram ✅
- `system-design.md` — thêm HTTP security headers, alerting, dashboard API schema ✅
- `implementation-plan.md` — sửa time→chrono, bỏ async-trait ✅
- `libs-auth.md` — chuẩn hóa Claims struct ✅
