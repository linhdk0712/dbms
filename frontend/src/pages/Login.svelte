<script>
  function handleLogin() {
    window.location.href = "/auth/login";
  }
</script>

<main class="min-h-screen flex items-center justify-center bg-base-200">
  <div class="card bg-base-100 shadow-xl w-96">
    <div class="card-body items-center text-center">
      <!-- Logo -->
      <div class="flex items-center gap-3 mb-2">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-10 w-10 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
        </svg>
        <h1 class="text-3xl font-bold tracking-tight">SECS</h1>
      </div>

      <p class="text-sm text-base-content/60 mb-1">SQL Execution Control System</p>
      <p class="text-sm text-base-content/80 mb-8">Hệ thống quản lý và kiểm soát thực thi SQL</p>

      <button class="btn btn-primary btn-wide gap-2" on:click={handleLogin}>
        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 16l-4-4m0 0l4-4m-4 4h14m-5 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h7a3 3 0 013 3v1" />
        </svg>
        Đăng nhập SSO
      </button>

      <p class="text-xs text-base-content/40 mt-6">Đăng nhập qua hệ thống SSO của công ty</p>
    </div>
  </div>
</main>
