-- Migration 007: Gap Fixes
-- Req 10: Multi-level approval support
-- Req 12: Nullable request_id for admin audit entries

-- Req 10: Multi-level approval - configurable levels per database
ALTER TABLE database_connections
ADD COLUMN approval_levels_required INT NOT NULL DEFAULT 1
CHECK (approval_levels_required BETWEEN 1 AND 5);

-- Req 10: Snapshot threshold on request (captured at submit time)
ALTER TABLE sql_requests
ADD COLUMN approval_levels_required INT NOT NULL DEFAULT 1;

-- Req 10: Approval tracking table
CREATE TABLE request_approvals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL REFERENCES sql_requests(id),
    approver_id UUID NOT NULL REFERENCES users(id),
    level INT NOT NULL,
    approved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(request_id, approver_id)
);

CREATE INDEX idx_request_approvals_request_id ON request_approvals(request_id);

-- Req 12: Make request_id nullable for admin audit entries (role change, status change)
ALTER TABLE audit_log ALTER COLUMN request_id DROP NOT NULL;
