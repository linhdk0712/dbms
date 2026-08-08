-- Migration 013: Add validation_result column to sql_requests
-- Lưu kết quả validate SQL (EXPLAIN) để hiển thị lại khi reload trang.
-- Trước đây validation result chỉ tồn tại trong HTTP response, mất khi refresh.

ALTER TABLE sql_requests
    ADD COLUMN validation_result JSONB;

COMMENT ON COLUMN sql_requests.validation_result IS 'Kết quả validate SQL (EXPLAIN) — JSONB {valid, execution_plan, warnings, error, validated_at}';
