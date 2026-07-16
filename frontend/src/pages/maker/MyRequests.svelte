<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";
  import StatusBadge from "../../components/StatusBadge.svelte";

  let requests = [];
  let loading = true;
  let error = null;
  let statusFilter = "";
  let pagination = { page: 1, per_page: 20, total: 0, total_pages: 0 };

  onMount(() => loadRequests());

  async function loadRequests() {
    loading = true;
    try {
      const response = await api.requests.list({
        status: statusFilter || undefined,
        page: pagination.page,
        per_page: pagination.per_page,
      });
      // Backend trả paginated response: { data: [...], pagination: {...} }
      if (response.data && response.pagination) {
        requests = response.data;
        pagination = response.pagination;
      } else {
        // Fallback nếu backend trả array (backward compat)
        requests = Array.isArray(response) ? response : [];
      }
    } catch (e) {
      error = e.error || "Không thể tải danh sách.";
    } finally {
      loading = false;
    }
  }

  function onFilterChange() {
    pagination.page = 1;
    loadRequests();
  }

  function goToPage(page) {
    pagination.page = page;
    loadRequests();
  }

  async function handleCancel(id) {
    if (!confirm("Bạn chắc chắn muốn hủy request này?")) return;
    try {
      await api.requests.cancel(id);
      loadRequests();
    } catch (e) {
      alert(e.error || "Không thể hủy.");
    }
  }

  async function handleSubmit(id) {
    try {
      await api.requests.submit(id);
      loadRequests();
    } catch (e) {
      alert(e.error || "Không thể submit.");
    }
  }
</script>

<div class="p-6">
  <div class="flex justify-between items-center mb-6">
    <h2 class="text-2xl font-bold">Requests của tôi</h2>
    <a href="#/requests/new" class="btn btn-primary btn-sm">+ Tạo mới</a>
  </div>

  <div class="flex gap-2 mb-4">
    <select class="select select-bordered select-sm" bind:value={statusFilter} on:change={onFilterChange}>
      <option value="">Tất cả trạng thái</option>
      <option value="DRAFT">DRAFT</option>
      <option value="PENDING">PENDING</option>
      <option value="APPROVED">APPROVED</option>
      <option value="EXECUTED">EXECUTED</option>
      <option value="FAILED">FAILED</option>
      <option value="REJECTED">REJECTED</option>
      <option value="CANCELLED">CANCELLED</option>
    </select>
  </div>

  {#if loading}
    <div class="flex justify-center py-10"><span class="loading loading-spinner"></span></div>
  {:else if error}
    <div class="alert alert-error"><span>{error}</span></div>
  {:else if requests.length === 0}
    <div class="text-center py-10 opacity-60">Chưa có request nào.</div>
  {:else}
    <div class="overflow-x-auto">
      <table class="table table-zebra w-full">
        <thead>
          <tr>
            <th>Mô tả</th>
            <th>Trạng thái</th>
            <th>Ngày tạo</th>
            <th>Hành động</th>
          </tr>
        </thead>
        <tbody>
          {#each requests as req}
            <tr>
              <td>
                <a href="#/requests/{req.id}" class="link link-hover">{req.description}</a>
              </td>
              <td><StatusBadge status={req.status} /></td>
              <td class="text-sm opacity-70">{new Date(req.created_at).toLocaleDateString("vi")}</td>
              <td>
                {#if req.status === "DRAFT"}
                  <button class="btn btn-xs btn-primary" on:click={() => handleSubmit(req.id)}>Submit</button>
                {/if}
                {#if req.status === "PENDING"}
                  <button class="btn btn-xs btn-ghost text-error" on:click={() => handleCancel(req.id)}>Hủy</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    {#if pagination.total_pages > 1}
      <div class="flex justify-center items-center gap-2 mt-4">
        <button
          class="btn btn-sm"
          disabled={pagination.page <= 1}
          on:click={() => goToPage(pagination.page - 1)}
        >
          ←
        </button>
        <span class="text-sm">
          Trang {pagination.page} / {pagination.total_pages}
          <span class="opacity-60">({pagination.total} kết quả)</span>
        </span>
        <button
          class="btn btn-sm"
          disabled={pagination.page >= pagination.total_pages}
          on:click={() => goToPage(pagination.page + 1)}
        >
          →
        </button>
      </div>
    {/if}
  {/if}
</div>
