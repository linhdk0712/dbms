<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import { authStore } from "../stores/auth.js";
  import api from "../api/index.js";

  let error = null;

  onMount(async () => {
    // Token được truyền qua URL fragment: /#/auth/callback?token=xxx
    const params = new URLSearchParams(window.location.hash.split("?")[1] || "");
    const token = params.get("token");

    if (!token) {
      error = "Không nhận được token từ server.";
      return;
    }

    try {
      // Lưu token tạm để gọi /auth/me
      authStore.login({ token, user: null });

      // Lấy thông tin user
      const user = await api.auth.me();
      authStore.login({ token, user });

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
