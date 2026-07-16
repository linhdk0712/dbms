-- Migration 003: SQL Requests table
-- Đơn vị trung tâm của hệ thống — mỗi request là một câu SQL cần được approve trước khi thực thi.

CREATE TABLE IF NOT EXISTS sql_requests (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    description         TEXT NOT NULL CHECK (length(description) >= 1),
    sql_content         TEXT NOT NULL CHECK (length(sql_content) >= 1),
    status              TEXT NOT NULL DEFAULT 'DRAFT'
                            CHECK (status IN ('DRAFT','PENDING','APPROVED','REJECTED','EXECUTED','FAILED','CANCELLED')),
    database_id         UUID NOT NULL REFERENCES database_connections(id),
    created_by          UUID NOT NULL REFERENCES users(id),
    approved_by         UUID REFERENCES users(id),
    rejection_reason    TEXT,
    executed_at         TIMESTAMPTZ,
    rows_affected       BIGINT,
    execution_error     TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at        TIMESTAMPTZ,

    -- 4-eyes principle: người approve phải khác người tạo
    CONSTRAINT four_eyes_check
        CHECK (approved_by IS NULL OR approved_by != created_by),

    -- rejection_reason bắt buộc khi REJECTED
    CONSTRAINT rejection_reason_required
        CHECK (status != 'REJECTED' OR (rejection_reason IS NOT NULL AND length(rejection_reason) >= 10))
);

COMMENT ON TABLE sql_requests IS 'SQL request workflow — Maker tạo, Checker approve, system thực thi';
COMMENT ON COLUMN sql_requests.sql_content IS 'Nội dung SQL — không trả về trong list API';
COMMENT ON CONSTRAINT four_eyes_check ON sql_requests IS '4-eyes principle: approved_by phải khác created_by';
