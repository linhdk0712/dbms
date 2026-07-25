-- Migration 009: Change Request integration with ManageEngine ServiceDesk Plus
-- Cho phép Maker gắn SQL request với một Change Request để traceability trong quy trình golive.

CREATE TABLE IF NOT EXISTS change_requests (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cr_number               VARCHAR(100) NOT NULL,        -- Change ID từ ManageEngine (e.g. "12345")
    cr_url                  TEXT NOT NULL,                 -- Full URL user nhập
    title                   TEXT,                          -- Title lấy từ ManageEngine API
    status                  TEXT,                          -- Status từ ManageEngine (e.g. "Approved")
    scheduled_start_time    TIMESTAMPTZ,                  -- Maintenance window start
    scheduled_end_time      TIMESTAMPTZ,                  -- Maintenance window end
    synced_at               TIMESTAMPTZ,                  -- Lần cuối sync từ ManageEngine
    created_by              UUID NOT NULL REFERENCES users(id),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_cr_number UNIQUE (cr_number)
);

COMMENT ON TABLE change_requests IS 'Change Request metadata cached from ManageEngine ServiceDesk Plus';
COMMENT ON COLUMN change_requests.cr_number IS 'Change ID extracted from ManageEngine URL';
COMMENT ON COLUMN change_requests.scheduled_start_time IS 'Maintenance window start — SQL execution blocked outside this window';

-- Liên kết sql_requests với change_requests (optional, N-1)
ALTER TABLE sql_requests
ADD COLUMN cr_id UUID REFERENCES change_requests(id) ON DELETE SET NULL;

-- Indexes
CREATE INDEX idx_sql_requests_cr_id ON sql_requests(cr_id);
CREATE INDEX idx_change_requests_cr_number ON change_requests(cr_number);
CREATE INDEX idx_change_requests_created_by ON change_requests(created_by);
