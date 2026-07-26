<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";
  import Skeleton from "../../components/Skeleton.svelte";
  import { toastStore } from "../../stores/toast.js";

  const STORAGE_KEY = "secs_pending_filters";

  let requests = [];
  let loading = true;
  let error = null;

  // Filter/sort state
  let selectedDatabase = "";
  let searchText = "";
  let sortOrder = "newest";

  /**
   * @description Khôi phục filter/sort preferences từ sessionStorage.
   * Đọc giá trị đã lưu trước đó để giữ trạng thái filter khi navigate giữa các trang.
   */
  function restorePreferences() {
    try {
      const saved = sessionStorage.getItem(STORAGE_KEY);
      if (saved) {
        const prefs = JSON.parse(saved);
        if (prefs.selectedDatabase) selectedDatabase = prefs.selectedDatabase;
        if (prefs.sortOrder) sortOrder = prefs.sortOrder;
      }
    } catch (_) {
      // Ignore parse errors
    }
  }

  /**
   * @description Lưu filter/sort preferences vào sessionStorage.
   * Được gọi mỗi khi selectedDatabase hoặc sortOrder thay đổi.
   */
  function savePreferences() {
    try {
      sessionStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ selectedDatabase, sortOrder })
      );
    } catch (_) {
      // Ignore storage errors
    }
  }

  // Extract unique database names from requests
  $: databases = [
    ...new Set(requests.map((r) => r.database_name).filter(Boolean)),
  ].sort();

  // Filtered and sorted requests (reactive)
  $: filteredRequests = requests
    .filter((req) => {
      // Database filter
      if (selectedDatabase && req.database_name !== selectedDatabase) {
        return false;
      }
      // Text search (case-insensitive)
      if (
        searchText &&
        !req.description?.toLowerCase().includes(searchText.toLowerCase())
      ) {
        return false;
      }
      return true;
    })
    .sort((a, b) => {
      const dateA = new Date(a.submitted_at || 0);
      const dateB = new Date(b.submitted_at || 0);
      return sortOrder === "newest" ? dateB - dateA : dateA - dateB;
    });

  // Persist on filter/sort change
  $: if (!loading) {
    selectedDatabase, sortOrder;
    savePreferences();
  }

  onMount(async () => {
    restorePreferences();
    try {
      requests = await api.approvals.pending();
    } catch (e) {
      error = e.error || "Không thể tải danh sách.";
    } finally {
      loading = false;
    }
  });
</script>

<div class="p-6">
  <div class="mb-6">
    <h2 class="text-2xl font-bold">Requests chờ duyệt</h2>
    <p class="text-sm text-base-content/60 mt-1">Review và phê duyệt các SQL request từ Maker</p>
  </div>

  {#if loading}
    <!-- Skeleton loading -->
    <div class="space-y-4">
      <div class="flex gap-3 mb-4">
        <div class="h-8 w-40 bg-base-300 rounded animate-pulse"></div>
        <div class="h-8 w-64 bg-base-300 rounded animate-pulse"></div>
        <div class="h-8 w-36 bg-base-300 rounded animate-pulse"></div>
      </div>
      {#each Array(4) as _}
        <Skeleton type="card" />
      {/each}
    </div>
  {:else if error}
    <div class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{error}</span>
      <button class="btn btn-sm btn-ghost" on:click={() => location.reload()}>Thử lại</button>
    </div>
  {:else}
    <!-- Toolbar: filters, search, sort -->
    <div class="flex flex-wrap gap-3 items-center mb-4">
      <!-- Database filter -->
      <select
        class="select select-bordered select-sm"
        bind:value={selectedDatabase}
      >
        <option value="">Tất cả database</option>
        {#each databases as db}
          <option value={db}>{db}</option>
        {/each}
      </select>

      <!-- Text search -->
      <input
        type="text"
        class="input input-bordered input-sm w-64"
        placeholder="Tìm theo mô tả..."
        bind:value={searchText}
      />

      <!-- Sort -->
      <select class="select select-bordered select-sm" bind:value={sortOrder}>
        <option value="newest">Mới nhất trước</option>
        <option value="oldest">Cũ nhất trước</option>
      </select>
    </div>

    <!-- Count display -->
    <div class="text-sm opacity-70 mb-4">
      Hiển thị {filteredRequests.length} / {requests.length} requests
    </div>

    {#if requests.length === 0}
      <div class="text-center py-16">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 mx-auto text-base-content/20 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <p class="text-base-content/50 text-lg mb-2">Tất cả đã xong!</p>
        <p class="text-base-content/40 text-sm">Không có request nào chờ duyệt.</p>
      </div>
    {:else if filteredRequests.length === 0}
      <div class="text-center py-10">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto text-base-content/20 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <p class="text-base-content/50">Không tìm thấy request phù hợp.</p>
      </div>
    {:else}
      <div class="grid gap-4">
        {#each filteredRequests as req}
          <a
            href="#/approvals/{req.id}"
            class="card bg-base-100 shadow-sm border border-base-300 hover:shadow-md transition-shadow"
          >
            <div class="card-body">
              <h3 class="card-title text-base">{req.description}</h3>
              <div class="flex gap-4 text-sm opacity-70">
                <span
                  >Submitted: {req.submitted_at
                    ? new Date(req.submitted_at).toLocaleString("vi")
                    : "-"}</span
                >
                {#if req.database_name}
                  <span class="badge badge-outline badge-sm"
                    >{req.database_name}</span
                  >
                {/if}
                {#if req.approval_levels_required > 1}
                  <span class="badge badge-info badge-sm">
                    {req.approval_count || 0}/{req.approval_levels_required} approvals
                  </span>
                {/if}
              </div>
            </div>
          </a>
        {/each}
      </div>
    {/if}
  {/if}
</div>
