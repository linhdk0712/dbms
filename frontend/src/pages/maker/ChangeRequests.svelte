<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";
  import CrBadge from "../../components/CrBadge.svelte";
  import MaintenanceWindow from "../../components/MaintenanceWindow.svelte";

  let crs = [];
  let loading = true;
  let error = null;
  let pagination = { page: 1, per_page: 20, total: 0, total_pages: 0 };

  // Create CR
  let showCreateForm = false;
  let newCrUrl = "";
  let creating = false;
  let createError = null;

  onMount(() => loadCrs());

  async function loadCrs() {
    loading = true;
    error = null;
    try {
      const response = await api.changeRequests.list({
        page: pagination.page,
        per_page: pagination.per_page,
      });
      if (response.data && response.pagination) {
        crs = response.data;
        pagination = response.pagination;
      } else {
        crs = Array.isArray(response) ? response : [];
      }
    } catch (e) {
      error = e.error || "Không thể tải danh sách CR.";
    } finally {
      loading = false;
    }
  }

  function goToPage(page) {
    pagination.page = page;
    loadCrs();
  }

  async function handleCreate() {
    if (!newCrUrl.trim()) {
      createError = "Vui lòng nhập URL.";
      return;
    }
    creating = true;
    createError = null;
    try {
      await api.changeRequests.create(newCrUrl.trim());
      newCrUrl = "";
      showCreateForm = false;
      loadCrs();
    } catch (e) {
      createError = e.error || "Không thể tạo CR.";
    } finally {
      creating = false;
    }
  }

  async function handleSync(id) {
    try {
      await api.changeRequests.sync(id);
      loadCrs();
    } catch (e) {
      alert(e.error || "Không thể sync.");
    }
  }
</script>

<div class="p-6">
  <div class="flex justify-between items-center mb-6">
    <h2 class="text-2xl font-bold">Change Requests</h2>
    <button
      class="btn btn-primary btn-sm"
      on:click={() => (showCreateForm = !showCreateForm)}
    >
      + Thêm CR
    </button>
  </div>

  <!-- Create form -->
  {#if showCreateForm}
    <div class="card bg-base-100 shadow mb-6">
      <div class="card-body">
        <h3 class="font-semibold mb-2">Thêm Change Request mới</h3>
        <div class="flex gap-2">
          <input
            type="url"
            class="input input-bordered flex-1"
            placeholder="Paste ManageEngine CR URL..."
            bind:value={newCrUrl}
            disabled={creating}
          />
          <button
            class="btn btn-primary"
            on:click={handleCreate}
            disabled={creating}
          >
            {creating ? "Đang tạo..." : "Tạo"}
          </button>
          <button class="btn btn-ghost" on:click={() => (showCreateForm = false)}>Hủy</button>
        </div>
        {#if createError}
          <div class="text-sm text-error mt-2">{createError}</div>
        {/if}
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="flex justify-center py-10"><span class="loading loading-spinner"></span></div>
  {:else if error}
    <div class="alert alert-error"><span>{error}</span></div>
  {:else if crs.length === 0}
    <div class="text-center py-10 opacity-60">Chưa có Change Request nào.</div>
  {:else}
    <div class="overflow-x-auto">
      <table class="table table-zebra w-full">
        <thead>
          <tr>
            <th>CR Number</th>
            <th>Title</th>
            <th>Status</th>
            <th>Maintenance Window</th>
            <th>Requests</th>
            <th>Hành động</th>
          </tr>
        </thead>
        <tbody>
          {#each crs as cr}
            <tr>
              <td>
                <CrBadge crNumber={cr.cr_number} crUrl={cr.cr_url} />
              </td>
              <td class="max-w-xs truncate">{cr.title || "—"}</td>
              <td>
                {#if cr.status}
                  <span class="badge badge-outline badge-sm">{cr.status}</span>
                {:else}
                  <span class="opacity-40">—</span>
                {/if}
              </td>
              <td>
                <MaintenanceWindow startTime={cr.scheduled_start_time} endTime={cr.scheduled_end_time} />
              </td>
              <td>
                <span class="badge badge-ghost badge-sm">{cr.request_count}</span>
              </td>
              <td>
                <button
                  class="btn btn-ghost btn-xs"
                  on:click={() => handleSync(cr.id)}
                  title="Sync từ ManageEngine"
                >
                  🔄
                </button>
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
          <span class="opacity-60">({pagination.total} CRs)</span>
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
