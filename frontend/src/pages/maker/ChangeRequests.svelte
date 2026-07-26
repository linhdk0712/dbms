<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";
  import CrBadge from "../../components/CrBadge.svelte";
  import MaintenanceWindow from "../../components/MaintenanceWindow.svelte";
  import Skeleton from "../../components/Skeleton.svelte";
  import { toastStore } from "../../stores/toast.js";

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

  /**
   * Loads the paginated list of Change Requests from the API.
   * Updates `crs`, `pagination`, and `error` state accordingly.
   */
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

  /**
   * Navigates to a specific page and reloads the CR list.
   * @param {number} page - The page number to navigate to.
   */
  function goToPage(page) {
    pagination.page = page;
    loadCrs();
  }

  /**
   * Handles creating a new Change Request from the ManageEngine URL input.
   * Validates the URL, calls the API, and refreshes the list on success.
   */
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

  /**
   * Syncs a Change Request with ManageEngine to refresh its data.
   * @param {string} id - The CR identifier to sync.
   */
  async function handleSync(id) {
    try {
      await api.changeRequests.sync(id);
      toastStore.success("Đã sync CR từ ManageEngine.");
      loadCrs();
    } catch (e) {
      toastStore.error(e.error || "Không thể sync.");
    }
  }
</script>

<div class="p-6">
  <div class="flex justify-between items-center mb-6">
    <div>
      <h2 class="text-2xl font-bold">Change Requests</h2>
      <p class="text-sm text-base-content/60 mt-1">Quản lý Change Request từ ManageEngine</p>
    </div>
    <button
      class="btn btn-primary btn-sm gap-1"
      on:click={() => (showCreateForm = !showCreateForm)}
    >
      {#if showCreateForm}
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
        Đóng
      {:else}
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        Thêm CR
      {/if}
    </button>
  </div>

  <!-- Create form -->
  {#if showCreateForm}
    <div class="card bg-base-100 shadow-sm border border-base-300 mb-6">
      <div class="card-body">
        <h3 class="text-lg font-semibold mb-4 flex items-center gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
          </svg>
          Thêm Change Request mới
        </h3>

        <div class="form-control">
          <label class="label" for="cr-url"><span class="label-text font-medium">ManageEngine CR URL</span></label>
          <input
            id="cr-url"
            type="url"
            class="input input-bordered w-full"
            placeholder="https://servicedesk.company.com/WorkOrder.do?woMode=viewWO&woID=..."
            bind:value={newCrUrl}
            disabled={creating}
          />
          <label class="label" for="cr-url">
            <span class="label-text-alt text-base-content/50">Paste link của Change Request từ ManageEngine ServiceDesk</span>
          </label>
        </div>

        {#if createError}
          <div class="alert alert-error mt-2">
            <span class="text-sm">{createError}</span>
          </div>
        {/if}

        <div class="divider my-2"></div>

        <div class="flex gap-2">
          <button
            class="btn btn-primary"
            on:click={handleCreate}
            disabled={creating || !newCrUrl.trim()}
          >
            {#if creating}
              <span class="loading loading-spinner loading-xs"></span>
            {:else}
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
              </svg>
            {/if}
            Tạo
          </button>
          <button class="btn btn-ghost" on:click={() => { showCreateForm = false; createError = null; }}>Hủy</button>
        </div>
      </div>
    </div>
  {/if}

  {#if loading}
    <Skeleton type="table" rows={5} cols={6} />
  {:else if error}
    <div class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{error}</span>
      <button class="btn btn-sm btn-ghost" on:click={loadCrs}>Thử lại</button>
    </div>
  {:else if crs.length === 0}
    <div class="text-center py-16">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 mx-auto text-base-content/20 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4" />
      </svg>
      <p class="text-base-content/50 mb-4">Chưa có Change Request nào.</p>
      <button class="btn btn-primary btn-sm" on:click={() => (showCreateForm = true)}>Thêm CR đầu tiên</button>
    </div>
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
