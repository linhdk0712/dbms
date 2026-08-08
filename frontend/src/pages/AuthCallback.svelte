<script>
  /**
   * @component AuthCallback
   * @description Handles the OIDC authentication callback after SSO redirect.
   * Supports two flows:
   * - Cookie-based auth (primary): server sets httpOnly cookie, this page calls /auth/me to get user info.
   * - Legacy token flow (backward compat): reads token from URL fragment and stores it.
   * On success, redirects to dashboard. On failure, shows error with retry link.
   */
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { authStore } from "../stores/auth.js";
  import api from "../api/index.js";

  let error = null;

  onMount(async () => {
    // Cookie-based auth: token đã được set bởi server qua httpOnly cookie.
    // Chỉ cần gọi /auth/me để lấy user info.
    //
    // Backward compat: nếu có token trong URL (legacy flow), vẫn xử lý.
    const params = new URLSearchParams(window.location.hash.split("?")[1] || "");
    const token = params.get("token");

    try {
      if (token) {
        // Legacy flow: token trong URL → lưu vào store cho backward compat
        authStore.login({ token, user: null });
      }

      // Gọi /auth/me — cookie httpOnly tự đính kèm, hoặc Bearer header (legacy)
      const user = await api.auth.me();

      if (token) {
        // Legacy: lưu cả token + user
        authStore.login({ token, user });
      } else {
        // Cookie flow: chỉ lưu user info (token nằm trong cookie)
        authStore.setUser(user);
      }

      // Redirect về dashboard
      push("/");
    } catch (e) {
      error = e.error || "Đăng nhập thất bại.";
      authStore.logout();
    }
  });
</script>

<main class="min-h-screen flex items-center justify-center bg-base-200">
  <div class="card bg-base-100 shadow-xl w-96">
    <div class="card-body items-center text-center">
      {#if error}
        <div class="alert alert-error">
          <span>{error}</span>
        </div>
        <a href="#/login" class="btn btn-primary mt-4">Thử lại</a>
      {:else}
        <span class="loading loading-spinner loading-lg"></span>
        <p class="mt-4">Đang xác thực...</p>
      {/if}
    </div>
  </div>
</main>
