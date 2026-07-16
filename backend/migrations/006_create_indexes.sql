-- Migration 006: Indexes
-- Tối ưu các truy vấn phổ biến: lọc theo status, theo maker, theo database, audit theo request.

-- SqlRequest queries
CREATE INDEX IF NOT EXISTS idx_sql_requests_status        ON sql_requests(status);
CREATE INDEX IF NOT EXISTS idx_sql_requests_created_by    ON sql_requests(created_by);
CREATE INDEX IF NOT EXISTS idx_sql_requests_database_id   ON sql_requests(database_id);

-- Audit log queries
CREATE INDEX IF NOT EXISTS idx_audit_log_request_id       ON audit_log(request_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor_id         ON audit_log(actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp        ON audit_log(timestamp);

-- Permission lookups
CREATE INDEX IF NOT EXISTS idx_user_db_permissions_user_id ON user_db_permissions(user_id);

-- User roles lookup
CREATE INDEX IF NOT EXISTS idx_user_roles_user_id          ON user_roles(user_id);
