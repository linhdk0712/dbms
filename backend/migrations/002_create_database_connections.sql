-- Migration 002: Database Connections table
-- Lưu thông tin kết nối đến các database đích (Oracle, PostgreSQL, MySQL, MongoDB).
-- Credentials được encrypt AES-256-GCM trước khi lưu.

CREATE TABLE IF NOT EXISTS database_connections (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name                    TEXT NOT NULL UNIQUE,   -- tên hiển thị, VD: "ORACLE_PROD_ACCOUNTS"
    db_type                 TEXT NOT NULL CHECK (db_type IN ('postgresql', 'mysql', 'oracle', 'mongodb')),
    host                    TEXT NOT NULL,
    port                    INTEGER NOT NULL CHECK (port > 0 AND port < 65536),
    database_name           TEXT NOT NULL,
    schema_name             TEXT,                   -- optional, dùng cho Oracle/PostgreSQL schema
    encrypted_credentials   TEXT NOT NULL,          -- AES-256-GCM encrypted JSON {user, password, ...}
    timeout_secs            INTEGER NOT NULL DEFAULT 30 CHECK (timeout_secs > 0 AND timeout_secs <= 300),
    is_active               BOOLEAN NOT NULL DEFAULT TRUE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE database_connections IS 'Thông tin kết nối đến database đích — credentials được mã hóa';
COMMENT ON COLUMN database_connections.encrypted_credentials IS 'AES-256-GCM encrypted JSON — không bao giờ log hoặc trả về trong API';
COMMENT ON COLUMN database_connections.timeout_secs IS 'Timeout cho SQL execution (giây), default 30, max 300';
