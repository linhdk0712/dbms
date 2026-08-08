-- Migration 010: Data Query Permission
-- Cho phép user tạo SELECT query requests với approval matrix linh hoạt,
-- assign users để run, và audit mỗi lần execute.

-- ============================================================
-- Table: data_query_requests
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_requests (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title                   TEXT NOT NULL,
    database_id             UUID NOT NULL REFERENCES database_connections(id),
    schema_name             TEXT NOT NULL,
    sql_query               TEXT NOT NULL,
    tables_accessed         TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    columns_accessed        TEXT[] NOT NULL DEFAULT '{}'::TEXT[],
    query_mode              TEXT NOT NULL DEFAULT 'BASIC'
                            CHECK (query_mode IN ('BASIC', 'ADVANCED')),
    ticket_id               TEXT NOT NULL,
    status                  TEXT NOT NULL DEFAULT 'DRAFT'
                            CHECK (status IN (
                                'DRAFT','PENDING_APPROVAL','APPROVED',
                                'REJECTED','EXPIRED','REVOKED'
                            )),
    created_by              UUID NOT NULL REFERENCES users(id),
    current_approval_level  INT NOT NULL DEFAULT 0,
    total_approval_levels   INT NOT NULL DEFAULT 1,
    expires_at              TIMESTAMPTZ,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at             TIMESTAMPTZ,
    revoked_at              TIMESTAMPTZ,
    revoke_reason           TEXT
);

COMMENT ON TABLE data_query_requests IS 'Data Query requests — SELECT only queries with multi-level approval';
COMMENT ON COLUMN data_query_requests.sql_query IS 'Immutable after APPROVED — the exact SQL to execute';
COMMENT ON COLUMN data_query_requests.current_approval_level IS 'Current approval level reached (0 = not yet submitted)';

-- ============================================================
-- Table: data_query_assigned_users
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_assigned_users (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_request_id        UUID NOT NULL REFERENCES data_query_requests(id) ON DELETE CASCADE,
    user_id                 UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    assigned_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(query_request_id, user_id)
);

COMMENT ON TABLE data_query_assigned_users IS 'Users assigned by Maker to run an approved data query';

-- ============================================================
-- Table: data_query_approval_rules (DBA config)
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_approval_rules (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    database_id             UUID NOT NULL REFERENCES database_connections(id) ON DELETE CASCADE,
    schema_name             TEXT,
    table_name              TEXT,
    created_by              UUID NOT NULL REFERENCES users(id),
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE data_query_approval_rules IS 'Approval matrix rules configured by DBA per database/schema/table';

-- ============================================================
-- Table: data_query_approval_levels
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_approval_levels (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id                 UUID NOT NULL REFERENCES data_query_approval_rules(id) ON DELETE CASCADE,
    level_order             INT NOT NULL,
    approver_unit           TEXT NOT NULL CHECK (approver_unit IN ('ITO', 'ITS', 'COMPLIANCE')),
    required                BOOLEAN NOT NULL DEFAULT TRUE,

    UNIQUE(rule_id, level_order)
);

COMMENT ON TABLE data_query_approval_levels IS 'Approval levels within a rule — sequential order';

-- ============================================================
-- Table: data_query_approval_tracking
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_approval_tracking (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_request_id        UUID NOT NULL REFERENCES data_query_requests(id) ON DELETE CASCADE,
    level_order             INT NOT NULL,
    approver_unit           TEXT NOT NULL,
    status                  TEXT NOT NULL DEFAULT 'PENDING'
                            CHECK (status IN ('PENDING', 'APPROVED', 'REJECTED', 'SKIPPED')),
    approved_by             UUID REFERENCES users(id),
    comment                 TEXT,
    acted_at                TIMESTAMPTZ,

    UNIQUE(query_request_id, level_order)
);

COMMENT ON TABLE data_query_approval_tracking IS 'Tracks approval status per request per level';

-- ============================================================
-- Table: data_query_executions
-- ============================================================
CREATE TABLE IF NOT EXISTS data_query_executions (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    query_request_id        UUID NOT NULL REFERENCES data_query_requests(id),
    executed_by             UUID NOT NULL REFERENCES users(id),
    rows_returned           BIGINT NOT NULL DEFAULT 0,
    execution_time_ms       BIGINT NOT NULL DEFAULT 0,
    client_ip               TEXT,
    status                  TEXT NOT NULL DEFAULT 'SUCCESS'
                            CHECK (status IN ('SUCCESS', 'FAILED', 'TIMEOUT')),
    error_message           TEXT,
    executed_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE data_query_executions IS 'Audit log for every query execution';

-- ============================================================
-- Indexes
-- ============================================================
CREATE INDEX idx_dqr_status ON data_query_requests(status)
    WHERE status IN ('APPROVED', 'PENDING_APPROVAL');
CREATE INDEX idx_dqr_created_by ON data_query_requests(created_by, created_at DESC);
CREATE INDEX idx_dqr_ticket ON data_query_requests(ticket_id);
CREATE INDEX idx_dqr_expires ON data_query_requests(expires_at)
    WHERE expires_at IS NOT NULL AND status = 'APPROVED';

CREATE INDEX idx_dqau_user ON data_query_assigned_users(user_id);
CREATE INDEX idx_dqau_request ON data_query_assigned_users(query_request_id);

CREATE UNIQUE INDEX idx_dqar_unique_rule ON data_query_approval_rules(
    database_id, COALESCE(schema_name, '*'), COALESCE(table_name, '*')
) WHERE is_active = TRUE;

CREATE INDEX idx_dqar_lookup ON data_query_approval_rules(database_id, schema_name, table_name)
    WHERE is_active = TRUE;

CREATE INDEX idx_dqat_pending ON data_query_approval_tracking(approver_unit, status)
    WHERE status = 'PENDING';
CREATE INDEX idx_dqat_request ON data_query_approval_tracking(query_request_id);

CREATE INDEX idx_dqe_request ON data_query_executions(query_request_id, executed_at DESC);
CREATE INDEX idx_dqe_user ON data_query_executions(executed_by, executed_at DESC);
CREATE INDEX idx_dqe_date ON data_query_executions(executed_at DESC);
