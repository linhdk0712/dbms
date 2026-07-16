-- Migration 005: User Database Permissions table
-- Phân quyền explicit theo từng DatabaseConnection — không có quyền global.
-- Admin cấp/thu quyền. Thu hồi quyền không ảnh hưởng các PENDING request đang tồn tại.

CREATE TABLE IF NOT EXISTS user_db_permissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    database_id     UUID NOT NULL REFERENCES database_connections(id) ON DELETE CASCADE,
    granted_by      UUID NOT NULL REFERENCES users(id),
    granted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Một user chỉ có một bản ghi quyền cho mỗi database
    UNIQUE(user_id, database_id)
);

COMMENT ON TABLE user_db_permissions IS 'Phân quyền explicit user - database, least privilege';
COMMENT ON COLUMN user_db_permissions.granted_by IS 'Admin đã cấp quyền này';
