-- Migration 008: Expand audit_log action check constraint
-- Thêm admin actions: ROLE_CHANGED, USER_STATUS_CHANGED, CREDENTIAL_ROTATED

-- Drop old constraint và tạo lại với thêm admin actions
ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS audit_log_action_check;
ALTER TABLE audit_log ADD CONSTRAINT audit_log_action_check
    CHECK (action IN (
        'CREATED', 'SUBMITTED', 'APPROVED', 'REJECTED',
        'EXECUTED', 'FAILED', 'CANCELLED',
        'ROLE_CHANGED', 'USER_STATUS_CHANGED', 'CREDENTIAL_ROTATED'
    ));
