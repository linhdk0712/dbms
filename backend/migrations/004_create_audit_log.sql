-- Migration 004: Audit Log table
-- Append-only — app DB user chỉ có INSERT + SELECT, KHÔNG có UPDATE/DELETE.
-- Mọi thay đổi trạng thái của SqlRequest phải ghi một bản ghi audit tương ứng.

CREATE TABLE IF NOT EXISTS audit_log (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id          UUID NOT NULL REFERENCES sql_requests(id),
    action              TEXT NOT NULL
                            CHECK (action IN ('CREATED','SUBMITTED','APPROVED','REJECTED','EXECUTED','FAILED','CANCELLED')),
    actor_id            UUID NOT NULL REFERENCES users(id),
    detail              TEXT,           -- lý do reject, error message khi FAILED, v.v.
    execution_result    BIGINT,         -- số rows affected (nếu EXECUTED)
    ip_address          TEXT,           -- IP của actor tại thời điểm hành động
    timestamp           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE audit_log IS 'Audit trail append-only — giữ tối thiểu 7 năm (SOX compliance)';
COMMENT ON COLUMN audit_log.action IS 'Hành động: CREATED, SUBMITTED, APPROVED, REJECTED, EXECUTED, FAILED, CANCELLED';
COMMENT ON COLUMN audit_log.execution_result IS 'Số rows affected khi thực thi thành công';
