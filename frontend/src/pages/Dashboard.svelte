<script>
  import { onMount } from "svelte";
  import api from "../api/index.js";
  import { authStore } from "../stores/auth.js";
  import Skeleton from "../components/Skeleton.svelte";

  let stats = null;
  let loading = true;
  let error = null;

  onMount(async () => {
    try {
      stats = await api.dashboard.get();
    } catch (e) {
      error = e.error || "Không thể tải dữ liệu dashboard.";
    } finally {
      loading = false;
    }
  });

  async function retry() {
    loading = true;
    error = null;
    try {
      stats = await api.dashboard.get();
    } catch (e) {
      error = e.error || "Không thể tải dữ liệu dashboard.";
    } finally {
      loading = false;
    }
  }

  $: roles = $authStore?.user?.roles || [];
  $: isMaker = roles.includes("MAKER");
  $: isChecker = roles.includes("CHECKER");
  $: isAdmin = roles.includes("ADMIN");
</script>

<div class="p-6">
  <!-- Header -->
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-2xl font-bold">Dashboard</h2>
      <p class="text-sm text-base-content/60 mt-1">
        Xin chào, {$authStore?.user?.display_name || "User"}
      </p>
    </div>
  </div>

  {#if loading}
    <!-- Skeleton loading for stats -->
    <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4 mb-8">
      {#each Array(4) as _}
        <Skeleton type="stat" />
      {/each}
    </div>
    <!-- Skeleton loading for quick links -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      {#each Array(3) as _}
        <Skeleton type="card" />
      {/each}
    </div>

  {:else if error}
    <div class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{error}</span>
      <button class="btn btn-sm btn-ghost" on:click={retry}>Thử lại</button>
    </div>

  {:else if stats}
    <!-- Stats Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      <div class="stat bg-base-100 shadow rounded-box">
        <div class="stat-figure text-base-content/30">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        </div>
        <div class="stat-title">Tổng requests</div>
        <div class="stat-value">{stats.total_requests}</div>
        <div class="stat-desc">Tất cả trạng thái</div>
      </div>

      <div class="stat bg-base-100 shadow rounded-box border-l-4 border-l-warning">
        <div class="stat-figure text-warning">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <div class="stat-title">Chờ duyệt</div>
        <div class="stat-value text-warning">{stats.pending_requests}</div>
        <div class="stat-desc">Đang chờ Checker review</div>
      </div>

      <div class="stat bg-base-100 shadow rounded-box border-l-4 border-l-success">
        <div class="stat-figure text-success">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <div class="stat-title">Đã thực thi</div>
        <div class="stat-value text-success">{stats.executed_requests}</div>
        <div class="stat-desc">Thực thi thành công</div>
      </div>

      <div class="stat bg-base-100 shadow rounded-box border-l-4 border-l-error">
        <div class="stat-figure text-error">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
        </div>
        <div class="stat-title">Thất bại / Reject</div>
        <div class="stat-value text-error">{stats.failed_requests + stats.rejected_requests}</div>
        <div class="stat-desc">
          {stats.failed_requests} lỗi, {stats.rejected_requests} bị từ chối
        </div>
      </div>
    </div>

    <!-- Secondary stats row -->
    {#if stats.approved_requests > 0 || stats.draft_requests > 0}
      <div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-8">
        {#if stats.approved_requests > 0}
          <div class="flex items-center gap-3 p-3 bg-base-100 rounded-box shadow-sm">
            <div class="badge badge-info badge-lg">{stats.approved_requests}</div>
            <span class="text-sm">Đã approve</span>
          </div>
        {/if}
        {#if stats.draft_requests > 0}
          <div class="flex items-center gap-3 p-3 bg-base-100 rounded-box shadow-sm">
            <div class="badge badge-neutral badge-lg">{stats.draft_requests || 0}</div>
            <span class="text-sm">Bản nháp</span>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Quick links theo role -->
    <h3 class="text-lg font-semibold mb-4">Hành động nhanh</h3>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#if isMaker}
        <a href="#/requests/new" class="card bg-primary text-primary-content shadow hover:shadow-lg transition-all hover:-translate-y-0.5">
          <div class="card-body py-5">
            <div class="flex items-center gap-3">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 opacity-80" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
              </svg>
              <div>
                <h3 class="card-title text-base">Tạo request mới</h3>
                <p class="text-sm opacity-80">Soạn SQL và gửi yêu cầu thực thi</p>
              </div>
            </div>
          </div>
        </a>
        <a href="#/requests" class="card bg-base-100 shadow hover:shadow-lg transition-all hover:-translate-y-0.5">
          <div class="card-body py-5">
            <div class="flex items-center gap-3">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 text-base-content/40" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <div>
                <h3 class="card-title text-base">Requests của tôi</h3>
                <p class="text-sm opacity-60">Xem trạng thái các request đã gửi</p>
              </div>
            </div>
          </div>
        </a>
      {/if}
      {#if isChecker}
        <a href="#/approvals" class="card bg-warning text-warning-content shadow hover:shadow-lg transition-all hover:-translate-y-0.5">
          <div class="card-body py-5">
            <div class="flex items-center gap-3">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 opacity-80" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <div>
                <h3 class="card-title text-base">Chờ duyệt</h3>
                <p class="text-sm opacity-80">
                  {stats.pending_requests > 0 ? `${stats.pending_requests} request đang chờ` : "Không có request mới"}
                </p>
              </div>
            </div>
          </div>
        </a>
      {/if}
      {#if isAdmin}
        <a href="#/admin/databases" class="card bg-base-100 shadow hover:shadow-lg transition-all hover:-translate-y-0.5">
          <div class="card-body py-5">
            <div class="flex items-center gap-3">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 text-base-content/40" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
              </svg>
              <div>
                <h3 class="card-title text-base">Quản lý Databases</h3>
                <p class="text-sm opacity-60">Thêm, sửa, test kết nối</p>
              </div>
            </div>
          </div>
        </a>
        <a href="#/admin/users" class="card bg-base-100 shadow hover:shadow-lg transition-all hover:-translate-y-0.5">
          <div class="card-body py-5">
            <div class="flex items-center gap-3">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 text-base-content/40" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
              </svg>
              <div>
                <h3 class="card-title text-base">Quản lý Users</h3>
                <p class="text-sm opacity-60">Cấp/thu hồi quyền</p>
              </div>
            </div>
          </div>
        </a>
      {/if}
    </div>
  {/if}
</div>
