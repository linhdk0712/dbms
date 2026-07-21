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
 */
export const APPROVALS = {
  PENDING: "/approvals/pending",
  GET: (id) => `/approvals/${id}`,
  VALIDATE: (id) => `/approvals/${id}/validate`,
  APPROVE: (id) => `/approvals/${id}/approve`,
  REJECT: (id) => `/approvals/${id}/reject`,
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
 */
export const DATABASES = {
  LIST: "/databases",
  MY_PERMISSIONS: "/databases/my-permissions",
  CREATE: "/databases",
  TEST_NEW: "/databases/test-new",
  GET: (id) => `/databases/${id}`,
  UPDATE: (id) => `/databases/${id}`,
  TEST: (id) => `/databases/${id}/test`,
};

// ─── Users (Admin) ─────────────────────────────────────────────────────────────

/**
 * User management endpoint paths (Admin role).
 * @property {string} LIST - List all users
 * @property {function(string): string} GET - Get user detail by ID
 * @property {function(string): string} UPDATE - Update user roles/status by ID
 * @property {function(string): string} PERMISSIONS - List/grant permissions for a user
 * @property {function(string, string): string} REVOKE_PERMISSION - Revoke a user's permission on a specific database
 */
export const USERS = {
  LIST: "/users",
  GET: (id) => `/users/${id}`,
  UPDATE: (id) => `/users/${id}`,
  PERMISSIONS: (userId) => `/users/${userId}/permissions`,
  REVOKE_PERMISSION: (userId, databaseId) =>
    `/users/${userId}/permissions/${databaseId}`,
};

// ─── Dashboard ─────────────────────────────────────────────────────────────────

/**
 * Dashboard endpoint paths (role-aware statistics).
 * @property {string} STATS - Get dashboard stats for current user's role
 */
export const DASHBOARD = {
  STATS: "/dashboard",
};
