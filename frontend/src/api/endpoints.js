/**
 * API Endpoints Constants — Single Source of Truth.
 *
 * TẤT CẢ endpoint paths được định nghĩa TẬP TRUNG tại đây.
 * Khi backend thay đổi path → chỉ cần sửa file này.
 *
 * Quy ước:
 * - Hàm nhận `id` hoặc params trả về path đầy đủ (sau BASE_URL)
 * - Static paths là string constants
 * - Dynamic paths là functions
 */

// ─── Auth ──────────────────────────────────────────────────────────────────────

/**
 * Authentication endpoint paths.
 * @property {string} LOGIN - Initiate SSO/OIDC login flow
 * @property {function(string, string): string} CALLBACK - OIDC callback with authorization code and state
 * @property {string} LOGOUT - End user session
 * @property {string} ME - Get current authenticated user info
 */
export const AUTH = {
  LOGIN: "/auth/login",
  CALLBACK: (code, state) => `/auth/callback?code=${code}&state=${state}`,
  LOGOUT: "/auth/logout",
  ME: "/auth/me",
};

// ─── Requests (Maker) ──────────────────────────────────────────────────────────

/**
 * SQL Request endpoint paths (Maker role).
 * @property {string} LIST - List all requests for current user
 * @property {string} CREATE - Create a new SQL request
 * @property {function(string): string} GET - Get request detail by ID
 * @property {function(string): string} UPDATE - Update a draft request by ID
 * @property {function(string): string} SUBMIT - Submit request for approval
 * @property {function(string): string} CANCEL - Cancel a pending request
 * @property {function(string): string} CLONE - Clone an existing request
 * @property {function(string): string} AUDIT - Get audit trail for a request
 * @property {function(string): string} ANALYZE - Trigger AI analysis for a request's SQL
 */
export const REQUESTS = {
  LIST: "/requests",
  CREATE: "/requests",
  GET: (id) => `/requests/${id}`,
  UPDATE: (id) => `/requests/${id}`,
  SUBMIT: (id) => `/requests/${id}/submit`,
  CANCEL: (id) => `/requests/${id}/cancel`,
  CLONE: (id) => `/requests/${id}/clone`,
  AUDIT: (id) => `/requests/${id}/audit`,
  ANALYZE: (id) => `/requests/${id}/analyze`,
};

// ─── Approvals (Checker) ───────────────────────────────────────────────────────

/**
 * Approval endpoint paths (Checker role).
 * @property {string} PENDING - List all pending approval requests
 * @property {function(string): string} GET - Get approval detail by request ID
 * @property {function(string): string} VALIDATE - Validate SQL before approval
 * @property {function(string): string} APPROVE - Approve a request
 * @property {function(string): string} REJECT - Reject a request
 * @property {function(string): string} RETRY - Retry execution of a previously failed request
 */
export const APPROVALS = {
  PENDING: "/approvals/pending",
  GET: (id) => `/approvals/${id}`,
  VALIDATE: (id) => `/approvals/${id}/validate`,
  APPROVE: (id) => `/approvals/${id}/approve`,
  REJECT: (id) => `/approvals/${id}/reject`,
  RETRY: (id) => `/approvals/${id}/retry`,
};

// ─── Databases (Admin) ─────────────────────────────────────────────────────────

/**
 * Database connection management endpoint paths (Admin role).
 * @property {string} LIST - List all database connections
 * @property {string} MY_PERMISSIONS - List databases current user has permission on
 * @property {string} CREATE - Register a new database connection
 * @property {string} TEST_NEW - Test connectivity for a new (unsaved) connection
 * @property {function(string): string} GET - Get database connection detail by ID
 * @property {function(string): string} UPDATE - Update database connection by ID
 * @property {function(string): string} TEST - Test connectivity for an existing connection
 * @property {function(string): string} TABLES - List tables/collections for a database connection
 * @property {function(string): string} COLUMNS - List columns/fields for a database connection
 */
export const DATABASES = {
  LIST: "/databases",
  MY_PERMISSIONS: "/databases/my-permissions",
  CREATE: "/databases",
  TEST_NEW: "/databases/test-new",
  GET: (id) => `/databases/${id}`,
  UPDATE: (id) => `/databases/${id}`,
  TEST: (id) => `/databases/${id}/test`,
  TABLES: (id) => `/databases/${id}/tables`,
  COLUMNS: (id) => `/databases/${id}/columns`,
};

// ─── Users (Admin) ─────────────────────────────────────────────────────────────

/**
 * User management endpoint paths (Admin role).
 * @property {string} LIST - List all users
 * @property {function(string): string} GET - Get user detail by ID
 * @property {function(string): string} UPDATE - Update user roles/status by ID
 * @property {function(string): string} PERMISSIONS - List/grant permissions for a user
 * @property {function(string, string): string} REVOKE_PERMISSION - Revoke a user's permission on a specific database
 * @property {function(string): string} APPROVER_UNITS - List approver units assigned to a user
 * @property {function(string, string): string} UNASSIGN_UNIT - Unassign an approver unit from a user
 */
export const USERS = {
  LIST: "/users",
  GET: (id) => `/users/${id}`,
  UPDATE: (id) => `/users/${id}`,
  PERMISSIONS: (userId) => `/users/${userId}/permissions`,
  REVOKE_PERMISSION: (userId, databaseId) =>
    `/users/${userId}/permissions/${databaseId}`,
  APPROVER_UNITS: (userId) => `/users/${userId}/approver-units`,
  UNASSIGN_UNIT: (userId, unitId) => `/users/${userId}/approver-units/${unitId}`,
};

// ─── Approver Units ────────────────────────────────────────────────────────────

/**
 * Approver Units management endpoint paths (Admin).
 * @property {string} LIST - List all approver units
 * @property {string} CREATE - Create a new approver unit
 * @property {function(string): string} UPDATE - Update an approver unit
 * @property {function(string): string} DELETE - Delete an approver unit
 */
export const APPROVER_UNITS = {
  LIST: "/approver-units",
  CREATE: "/approver-units",
  UPDATE: (id) => `/approver-units/${id}`,
  DELETE: (id) => `/approver-units/${id}`,
};

// ─── Dashboard ─────────────────────────────────────────────────────────────────

/**
 * Dashboard endpoint paths (role-aware statistics).
 * @property {string} STATS - Get dashboard stats for current user's role
 */
export const DASHBOARD = {
  STATS: "/dashboard",
};

// ─── Change Requests (CR) ──────────────────────────────────────────────────────

/**
 * Change Request endpoint paths (ManageEngine integration).
 * @property {string} LIST - List all change requests
 * @property {string} CREATE - Create/link a new change request from URL
 * @property {function(string): string} GET - Get CR detail by ID
 * @property {function(string): string} SYNC - Re-sync CR metadata from ManageEngine
 * @property {function(string): string} DELETE - Delete CR (Admin only)
 */
export const CHANGE_REQUESTS = {
  LIST: "/change-requests",
  CREATE: "/change-requests",
  GET: (id) => `/change-requests/${id}`,
  SYNC: (id) => `/change-requests/${id}/sync`,
  DELETE: (id) => `/change-requests/${id}`,
};

// ─── Data Queries ──────────────────────────────────────────────────────────────

/**
 * Data Query Permission endpoint paths.
 * @property {string} LIST - List data query requests (filter by status, user)
 * @property {string} CREATE - Create a new data query request
 * @property {function(string): string} GET - Get data query detail by ID
 * @property {function(string): string} UPDATE - Update a draft data query
 * @property {function(string): string} SUBMIT - Submit data query for approval
 * @property {function(string): string} EXECUTE - Execute an approved data query
 * @property {function(string): string} EXECUTIONS - Get execution history of a query
 */
export const DATA_QUERIES = {
  LIST: "/data-queries",
  CREATE: "/data-queries",
  GET: (id) => `/data-queries/${id}`,
  UPDATE: (id) => `/data-queries/${id}`,
  SUBMIT: (id) => `/data-queries/${id}/submit`,
  EXECUTE: (id) => `/data-queries/${id}/execute`,
  EXECUTIONS: (id) => `/data-queries/${id}/executions`,
};

// ─── Data Query Approvals ──────────────────────────────────────────────────────

/**
 * Data Query Approval endpoint paths (Approvers: ITO/ITS/Compliance).
 * @property {string} PENDING - List pending data query approvals for current user's unit
 * @property {function(string): string} APPROVE - Approve a data query at current level
 * @property {function(string): string} REJECT - Reject a data query
 */
export const DATA_QUERY_APPROVALS = {
  PENDING: "/data-query-approvals/pending",
  APPROVE: (id) => `/data-queries/${id}/approve`,
  REJECT: (id) => `/data-queries/${id}/reject`,
};

// ─── Data Query Approval Rules (DBA/Admin) ─────────────────────────────────────

/**
 * Approval Matrix management endpoint paths (Admin/DBA).
 * @property {string} LIST - List all approval rules
 * @property {string} CREATE - Create a new approval rule
 * @property {function(string): string} UPDATE - Update an approval rule
 * @property {function(string): string} DELETE - Delete an approval rule
 */
export const DATA_QUERY_RULES = {
  LIST: "/data-query-rules",
  CREATE: "/data-query-rules",
  UPDATE: (id) => `/data-query-rules/${id}`,
  DELETE: (id) => `/data-query-rules/${id}`,
};

// ─── Data Query Executions Audit (Admin) ───────────────────────────────────────

/**
 * Data Query Execution audit log endpoints.
 * @property {string} LIST - List all executions (Admin/DBA audit)
 */
export const DATA_QUERY_EXECUTIONS = {
  LIST: "/data-query-executions",
};
