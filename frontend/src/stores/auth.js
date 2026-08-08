import { writable } from "svelte/store";

/**
 * Auth store — quản lý trạng thái đăng nhập.
 *
 * JWT token được lưu trong httpOnly cookie (server-managed) — không accessible từ JS.
 * Store chỉ lưu user info để hiển thị UI (roles, display_name, email).
 *
 * SECURITY: Token KHÔNG được lưu trong localStorage nữa → chống XSS token theft.
 * Backward compat: nếu có token cũ trong localStorage (từ phiên trước), vẫn đọc được
 * để gửi qua Bearer header cho đến khi hết hạn.
 */
function createAuthStore() {
  // Khôi phục user info từ sessionStorage (không lưu token)
  const stored = typeof sessionStorage !== "undefined"
    ? JSON.parse(sessionStorage.getItem("secs_user") || "null")
    : null;

  // Backward compat: nếu có auth cũ trong localStorage (legacy), migrate
  const legacy = typeof localStorage !== "undefined"
    ? JSON.parse(localStorage.getItem("secs_auth") || "null")
    : null;

  const initial = stored || (legacy ? { user: legacy.user, token: legacy.token } : null);

  const { subscribe, set, update } = writable(initial);

  return {
    subscribe,

    /**
     * Đăng nhập — lưu user info (token nằm trong httpOnly cookie).
     * Nếu có token (backward compat từ legacy), vẫn lưu tạm.
     */
    login(data) {
      const auth = { user: data.user, token: data.token || null };
      // Chỉ lưu user info vào sessionStorage (tồn tại trong tab hiện tại)
      sessionStorage.setItem("secs_user", JSON.stringify(auth));
      // Xóa legacy localStorage
      localStorage.removeItem("secs_auth");
      set(auth);
    },

    /**
     * Đăng nhập chỉ với user info (khi dùng cookie auth — không có token trả về).
     */
    setUser(user) {
      const auth = { user, token: null };
      sessionStorage.setItem("secs_user", JSON.stringify(auth));
      localStorage.removeItem("secs_auth");
      set(auth);
    },

    /** Đăng xuất — xóa state (cookie sẽ bị clear bởi server response) */
    logout() {
      sessionStorage.removeItem("secs_user");
      localStorage.removeItem("secs_auth");
      set(null);
    },

    /** Kiểm tra đã đăng nhập chưa (có user info trong store) */
    get isAuthenticated() {
      let val;
      subscribe((v) => (val = v))();
      return val?.user != null;
    },
  };
}

export const authStore = createAuthStore();
