import { writable } from "svelte/store";

/**
 * Auth store — quản lý trạng thái đăng nhập.
 * Lưu JWT token và thông tin user.
 */
function createAuthStore() {
  // Khôi phục từ localStorage (nếu có)
  const stored = typeof localStorage !== "undefined"
    ? JSON.parse(localStorage.getItem("secs_auth") || "null")
    : null;

  const { subscribe, set, update } = writable(stored);

  return {
    subscribe,

    /** Đăng nhập — lưu token + user info */
    login(data) {
      const auth = { token: data.token, user: data.user };
      localStorage.setItem("secs_auth", JSON.stringify(auth));
      set(auth);
    },

    /** Đăng xuất — xóa token */
    logout() {
      localStorage.removeItem("secs_auth");
      set(null);
    },

    /** Kiểm tra đã đăng nhập chưa */
    get isAuthenticated() {
      let val;
      subscribe((v) => (val = v))();
      return val?.token != null;
    },
  };
}

export const authStore = createAuthStore();
