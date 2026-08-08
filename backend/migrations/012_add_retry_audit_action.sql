-- Migration 012: Add RETRY and EXECUTING audit actions
-- RETRY: khi Checker re-execute request FAILED
-- EXECUTING: ghi trước khi bắt đầu thực thi SQL

ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS audit_log_action_check;
ALTER TABLE audit_log ADD CONSTRAINT audit_log_action_check
    CHECK (action IN (
        'CREATED', 'SUBMITTED', 'APPROVED', 'REJECTED',
        'EXECUTED', 'FAILED', 'CANCELLED',
        'ROLE_CHANGED', 'USER_STATUS_CHANGED', 'CREDENTIAL_ROTATED',
        'EXECUTING', 'RETRY'
    ));
