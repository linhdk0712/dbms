-- Migration 001: Users table
-- Users được tạo/cập nhật khi OIDC login.
-- Không có password — auth hoàn toàn qua OIDC/SSO.

CREATE TABLE IF NOT EXISTS users (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    oidc_subject    TEXT NOT NULL UNIQUE,   -- sub claim từ OIDC provider
    email           TEXT NOT NULL UNIQUE,
    display_name    TEXT NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,  -- soft delete
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Roles được lưu trong bảng riêng để support nhiều role
CREATE TABLE IF NOT EXISTS user_roles (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role        TEXT NOT NULL CHECK (role IN ('ADMIN', 'MAKER', 'CHECKER')),
    granted_by  UUID REFERENCES users(id),
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, role)
);

COMMENT ON TABLE users IS 'Users của hệ thống SECS — auth qua OIDC';
COMMENT ON COLUMN users.oidc_subject IS 'sub claim từ SSO provider — identifier duy nhất';
COMMENT ON COLUMN users.is_active IS 'Soft delete — set FALSE thay vì DELETE để giữ audit trail';
