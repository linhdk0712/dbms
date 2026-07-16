<script>
  import api from "../api/index.js";

  function handleLogin() {
    // Redirect đến backend /auth/login → OIDC provider
    window.location.href = "/auth/login";
  }
</script>

<main class="min-h-screen flex items-center justify-center bg-base-200">
  <div class="card bg-base-100 shadow-xl w-96">
    <div class="card-body items-center text-center">
      <h1 class="card-title text-3xl font-bold">SECS</h1>
      <p class="text-sm opacity-70 mb-4">SQL Execution Control System</p>
      <p class="text-sm mb-6">Hệ thống quản lý và kiểm soát thực thi SQL</p>
      <button class="btn btn-primary btn-wide" on:click={handleLogin}>
        Đăng nhập SSO
      </button>
      <p class="text-xs opacity-50 mt-4">Đăng nhập qua hệ thống SSO của công ty</p>
    </div>
  </div>
</main>
