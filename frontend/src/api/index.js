/**
 * API Client tập trung cho SECS Frontend.
 *
 * QUY TẮC:
 * - MỌI request đến backend PHẢI đi qua file này.
 * - KHÔNG gọi fetch() trực tiếp trong component.
 * - Endpoint paths được quản lý tại: src/api/endpoints.js
 * - Trước khi thêm endpoint mới, kiểm tra spec tại: .kiro/steering/api-spec.md
 *
 * CẤU TRÚC:
 * - endpoints.js: constants cho tất cả paths (single source of truth)
 * - apiFetch(): wrapper chung, tự attach JWT, handle errors
 * - Mỗi nhóm endpoint = 1 object (auth, requests, databases, users, dashboard)
 * - Export tất cả qua object `api`
 */

import { get } from "svelte/store";
import { authStore } from "../stores/auth.js";
import {
  AUTH,
  REQUESTS,
  APPROVALS,
  DATABASES,
  USERS,
  DASHBOARD,
  CHANGE_REQUESTS,
} from "./endpoints.js";

// ─── Base Config ───────────────────────────────────────────────────────────────

const BASE_URL = "/api/v1";

// ─── Core Fetch Wrapper ────────────────────────────────────────────────────────

/**
 * Wrapper fetch tập trung. Tự thêm Authorization header, parse JSON, handle errors.
 * @param {string} endpoint - Path (lấy từ endpoints.js)
 * @param {object} options - fetch options (method, body, headers)
 * @param {boolean} useBaseUrl - Có prepend /api/v1 không (default: true)
 * @returns {Promise<any>} - Response data (đã parse)
 * @throws {{ status: number, error: string }} - Error object
 */
async function apiFetch(endpoint, options = {}, useBaseUrl = true) {
  const token = get(authStore)?.token;

  const headers = {
    "Content-Type": "application/json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...options.headers,
  };

  const config = {
    ...options,
    headers,
  };

  if (options.body && typeof options.body === "object") {
    config.body = JSON.stringify(options.body);
  }

  const url = useBaseUrl ? `${BASE_URL}${endpoint}` : endpoint;
  const response = await fetch(url, config);

  // 204 No Content
  if (response.status === 204) {
    return null;
  }

  // 401 — token hết hạn hoặc không hợp lệ
  if (response.status === 401) {
    authStore.logout();
    throw { status: 401, error: "Session expired. Please login again." };
  }

  const data = await response.json();

  if (!response.ok) {
    throw { status: response.status, error: data.error || "Unknown error" };
  }

  // Nếu response có pagination metadata (paginated response), trả nguyên object
  if (data.data !== undefined && data.pagination !== undefined) {
    return data;
  }

  return data.data !== undefined ? data.data : data;
}

// ─── Auth ──────────────────────────────────────────────────────────────────────

const auth = {
  /** Redirect đến OIDC login */
  login() {
    window.location.href = `/auth/login`;
  },

  /** OIDC callback — exchange code for JWT */
  async callback(code, state) {
    return apiFetch(AUTH.CALLBACK(code, state), {}, false);
  },

  /** Logout */
  async logout() {
    return apiFetch(AUTH.LOGOUT, { method: "POST" }, false);
  },

  /** Lấy thông tin user hiện tại */
  async me() {
    return apiFetch(AUTH.ME, {}, false);
  },
};

// ─── Requests ──────────────────────────────────────────────────────────────────

const requests = {
  /** Tạo request mới (DRAFT hoặc submit luôn) */
  async create({ description, sql_content, database_id, submit = false, cr_url }) {
    const body = { description, sql_content, database_id, submit };
    if (cr_url) body.cr_url = cr_url;
    return apiFetch(REQUESTS.CREATE, {
      method: "POST",
      body,
    });
  },

  /** Danh sách requests (có pagination + filter) */
  async list({ status, database_id, page = 1, per_page = 20 } = {}) {
    const params = new URLSearchParams();
    if (status) params.set("status", status);
    if (database_id) params.set("database_id", database_id);
    params.set("page", page);
    params.set("per_page", per_page);
    return apiFetch(`${REQUESTS.LIST}?${params}`);
  },

  /** Chi tiết request (bao gồm sql_content + audit trail) */
  async get(id) {
    return apiFetch(REQUESTS.GET(id));
  },

  /** Sửa request (chỉ khi DRAFT) */
  async update(id, { description, sql_content }) {
    return apiFetch(REQUESTS.UPDATE(id), {
      method: "PATCH",
      body: { description, sql_content },
    });
  },

  /** Submit DRAFT -> PENDING */
  async submit(id) {
    return apiFetch(REQUESTS.SUBMIT(id), { method: "POST" });
  },

  /** Cancel PENDING request (chỉ owner) */
  async cancel(id) {
    return apiFetch(REQUESTS.CANCEL(id), { method: "POST" });
  },

  /** Clone request thành DRAFT mới */
  async clone(id) {
    return apiFetch(REQUESTS.CLONE(id), { method: "POST" });
  },

  /** Lấy audit trail của request */
  async audit(id) {
    return apiFetch(REQUESTS.AUDIT(id));
  },

  /** AI phân tích SQL — performance, security, best practices, cost */
  async analyze(id) {
    return apiFetch(REQUESTS.ANALYZE(id), { method: "POST" });
  },
};

// ─── Approvals (sử dụng endpoint approvals thay vì requests) ──────────────────

const approvals = {
  /** Danh sách requests chờ duyệt */
  async pending() {
    return apiFetch(APPROVALS.PENDING);
  },

  /** Chi tiết request (Checker view — bao gồm sql_content) */
  async get(id) {
    return apiFetch(APPROVALS.GET(id));
  },

  /** Validate SQL trước khi approve (EXPLAIN check) */
  async validate(id) {
    return apiFetch(APPROVALS.VALIDATE(id), { method: "POST" });
  },

  /** Approve request (Checker only) */
  async approve(id) {
    return apiFetch(APPROVALS.APPROVE(id), { method: "POST" });
  },

  /** Reject request (Checker only, cần rejection_reason min 10 chars) */
  async reject(id, rejection_reason) {
    return apiFetch(APPROVALS.REJECT(id), {
      method: "POST",
      body: { rejection_reason },
    });
  },
};

// ─── Databases ─────────────────────────────────────────────────────────────────

const databases = {
  /** Danh sách databases (ADMIN thấy hết, user khác thấy DB có quyền) */
  async list() {
    return apiFetch(DATABASES.LIST);
  },

  /** Danh sách databases mà user hiện tại có quyền (Maker endpoint) */
  async myPermissions() {
    return apiFetch(DATABASES.MY_PERMISSIONS);
  },

  /** Tạo database connection mới (Admin only) */
  async create(payload) {
    return apiFetch(DATABASES.CREATE, { method: "POST", body: payload });
  },

  /** Cập nhật database connection (Admin only) */
  async update(id, payload) {
    return apiFetch(DATABASES.UPDATE(id), { method: "PATCH", body: payload });
  },

  /** Test kết nối (Admin only) */
  async test(id) {
    return apiFetch(DATABASES.TEST(id), { method: "POST" });
  },

  /** Test kết nối với raw params trước khi tạo (Admin only) */
  async testNew(payload) {
    return apiFetch(DATABASES.TEST_NEW, { method: "POST", body: payload });
  },
};

// ─── Users ─────────────────────────────────────────────────────────────────────

const users = {
  /** Danh sách users (Admin only) */
  async list() {
    return apiFetch(USERS.LIST);
  },

  /** Cập nhật user: activate/deactivate, gán roles (Admin only) */
  async update(id, payload) {
    return apiFetch(USERS.UPDATE(id), { method: "PATCH", body: payload });
  },

  /** Lấy danh sách permissions của user (Admin only) */
  async getPermissions(userId) {
    return apiFetch(USERS.PERMISSIONS(userId));
  },

  /** Cấp quyền user trên database (Admin only) */
  async grantPermission(userId, databaseId) {
    return apiFetch(USERS.PERMISSIONS(userId), {
      method: "POST",
      body: { database_id: databaseId },
    });
  },

  /** Thu hồi quyền (Admin only) */
  async revokePermission(userId, databaseId) {
    return apiFetch(USERS.REVOKE_PERMISSION(userId, databaseId), {
      method: "DELETE",
    });
  },
};

// ─── Dashboard ─────────────────────────────────────────────────────────────────

const dashboard = {
  /** Lấy stats theo role hiện tại */
  async get() {
    return apiFetch(DASHBOARD.STATS);
  },
};

// ─── Change Requests ───────────────────────────────────────────────────────────

const changeRequests = {
  /** Tạo/link CR từ ManageEngine URL */
  async create(cr_url) {
    return apiFetch(CHANGE_REQUESTS.CREATE, {
      method: "POST",
      body: { cr_url },
    });
  },

  /** Danh sách CRs (pagination) */
  async list({ page = 1, per_page = 20 } = {}) {
    const params = new URLSearchParams();
    params.set("page", page);
    params.set("per_page", per_page);
    return apiFetch(`${CHANGE_REQUESTS.LIST}?${params}`);
  },

  /** Chi tiết CR */
  async get(id) {
    return apiFetch(CHANGE_REQUESTS.GET(id));
  },

  /** Re-sync metadata từ ManageEngine */
  async sync(id) {
    return apiFetch(CHANGE_REQUESTS.SYNC(id), { method: "POST" });
  },

  /** Xóa CR (Admin only) */
  async delete(id) {
    return apiFetch(CHANGE_REQUESTS.DELETE(id), { method: "DELETE" });
  },
};

// ─── Export tập trung ──────────────────────────────────────────────────────────

export const api = {
  auth,
  requests,
  approvals,
  databases,
  users,
  dashboard,
  changeRequests,
};

export default api;
