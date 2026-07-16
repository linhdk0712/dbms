<script>
  import { onMount } from "svelte";
  import api from "../api/index.js";
  import { authStore } from "../stores/auth.js";

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

  $: roles = $authStore?.user?.roles || [];
  $: isMaker = roles.includes("MAKER");
  $: isChecker = roles.includes("CHECKER");
  $: isAdmin = roles.includes("ADMIN");
</script>

<div class="p-6">
  <h2 class="text-2xl font-bold mb-6">Dashboard</h2>

  {#if loading}
    <div class="flex justify-center"><span class="loading loading-spinner loading-lg"></span></div>
  {:else if error}
    <div class="alert alert-error"><span>{error}</span></div>
  {:else if stats}
    <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8">
      <div class="stat bg-base-100 shadow rounded-box">
        <div class="stat-title">Tổng</div>
        <div class="stat-value text-lg">{stats.total_requests}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-box">
        <div class="stat-title">Chờ duyệt</div>
        <div class="stat-value text-lg text-warning">{stats.pending_requests}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-box">
        <div class="stat-title">Đã approve</div>
        <div class="stat-value text-lg text-info">{stats.approved_requests}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-box">
        <div class="stat-title">Đã thực thi</div>
        <div class="stat-value text-lg text-success">{stats.executed_requests}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-box">
        <div class="stat-title">Thất bại</div>
        <div class="stat-value text-lg text-error">{stats.failed_requests}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-box">
        <div class="stat-title">Bị reject</div>
        <div class="stat-value text-lg text-error">{stats.rejected_requests}</div>
      </div>
    </div>

    <!-- Quick links theo role -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      {#if isMaker}
        <a href="#/requests/new" class="card bg-primary text-primary-content shadow-lg hover:shadow-xl transition-shadow">
          <div class="card-body">
            <h3 class="card-title">Tạo request mới</h3>
            <p>Soạn câu lệnh SQL và gửi yêu cầu thực thi</p>
          </div>
        </a>
        <a href="#/requests" class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow">
          <div class="card-body">
            <h3 class="card-title">Requests của tôi</h3>
            <p>Xem danh sách và trạng thái các request đã gửi</p>
          </div>
        </a>
      {/if}
      {#if isChecker}
        <a href="#/approvals" class="card bg-warning text-warning-content shadow-lg hover:shadow-xl transition-shadow">
          <div class="card-body">
            <h3 class="card-title">Chờ duyệt</h3>
            <p>Review và approve/reject các request</p>
          </div>
        </a>
      {/if}
      {#if isAdmin}
        <a href="#/admin/databases" class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow">
          <div class="card-body">
            <h3 class="card-title">Quản lý Databases</h3>
            <p>Thêm, sửa, test kết nối database</p>
          </div>
        </a>
        <a href="#/admin/users" class="card bg-base-100 shadow-lg hover:shadow-xl transition-shadow">
          <div class="card-body">
            <h3 class="card-title">Quản lý Users</h3>
            <p>Cấp/thu hồi quyền cho user</p>
          </div>
        </a>
      {/if}
    </div>
  {/if}
</div>
