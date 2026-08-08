-- Migration 011: Approver Units — linh hoạt, admin-defined
-- Thay thế CHECK constraint cứng ('ITO', 'ITS', 'COMPLIANCE') bằng bảng cấu hình.

-- ============================================================
-- Table: approver_units — Admin define các đơn vị phê duyệt
-- ============================================================
CREATE TABLE IF NOT EXISTS approver_units (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE approver_units IS 'Danh sách đơn vị phê duyệt (admin-defined, không hardcode)';

-- Seed default units (tương thích với data cũ)
INSERT INTO approver_units (name, description) VALUES
    ('ITO', 'IT Operations'),
    ('ITS', 'IT Security'),
    ('COMPLIANCE', 'Compliance Team')
ON CONFLICT (name) DO NOTHING;

-- ============================================================
-- Table: user_approver_units — Gán user vào đơn vị phê duyệt
-- ============================================================
CREATE TABLE IF NOT EXISTS user_approver_units (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    unit_id     UUID NOT NULL REFERENCES approver_units(id) ON DELETE CASCADE,
    granted_by  UUID REFERENCES users(id),
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(user_id, unit_id)
);

COMMENT ON TABLE user_approver_units IS 'Gán user vào approver unit — mỗi user có thể thuộc nhiều units';

CREATE INDEX idx_uau_user ON user_approver_units(user_id);
CREATE INDEX idx_uau_unit ON user_approver_units(unit_id);

-- ============================================================
-- Bỏ CHECK constraint cứng trên data_query_approval_levels.approver_unit
-- Cho phép giá trị tùy ý (tham chiếu theo tên từ approver_units)
-- ============================================================
ALTER TABLE data_query_approval_levels DROP CONSTRAINT IF EXISTS data_query_approval_levels_approver_unit_check;


-- ============================================================
-- Thêm role APPROVER vào CHECK constraint của user_roles
-- ============================================================
ALTER TABLE user_roles DROP CONSTRAINT IF EXISTS user_roles_role_check;
ALTER TABLE user_roles ADD CONSTRAINT user_roles_role_check
    CHECK (role IN ('ADMIN', 'MAKER', 'CHECKER', 'APPROVER'));
