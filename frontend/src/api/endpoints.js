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

export const AUTH = {
  LOGIN: "/auth/login",
  CALLBACK: (code, state) => `/auth/callback?code=${code}&state=${state}`,
  LOGOUT: "/auth/logout",
  ME: "/auth/me",
};

// ─── Requests (Maker) ──────────────────────────────────────────────────────────

export const REQUESTS = {
  LIST: "/requests",
  CREATE: "/requests",
  GET: (id) => `/requests/${id}`,
  UPDATE: (id) => `/requests/${id}`,
  SUBMIT: (id) => `/requests/${id}/submit`,
  CANCEL: (id) => `/requests/${id}/cancel`,
  CLONE: (id) => `/requests/${id}/clone`,
  AUDIT: (id) => `/requests/${id}/audit`,
};

// ─── Approvals (Checker) ───────────────────────────────────────────────────────

export const APPROVALS = {
  PENDING: "/approvals/pending",
  GET: (id) => `/approvals/${id}`,
  APPROVE: (id) => `/approvals/${id}/approve`,
  REJECT: (id) => `/approvals/${id}/reject`,
};

// ─── Databases (Admin) ─────────────────────────────────────────────────────────

export const DATABASES = {
  LIST: "/databases",
  MY_PERMISSIONS: "/databases/my-permissions",
  CREATE: "/databases",
  GET: (id) => `/databases/${id}`,
  UPDATE: (id) => `/databases/${id}`,
  TEST: (id) => `/databases/${id}/test`,
};

// ─── Users (Admin) ─────────────────────────────────────────────────────────────

export const USERS = {
  LIST: "/users",
  GET: (id) => `/users/${id}`,
  UPDATE: (id) => `/users/${id}`,
  PERMISSIONS: (userId) => `/users/${userId}/permissions`,
  REVOKE_PERMISSION: (userId, databaseId) =>
    `/users/${userId}/permissions/${databaseId}`,
};

// ─── Dashboard ─────────────────────────────────────────────────────────────────

export const DASHBOARD = {
  STATS: "/dashboard",
};
