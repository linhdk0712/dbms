<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";

  const STORAGE_KEY = "secs_pending_filters";

  let requests = [];
  let loading = true;
  let error = null;

  // Filter/sort state
  let selectedDatabase = "";
  let searchText = "";
  let sortOrder = "newest";

  // Restore preferences from sessionStorage
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

  // Persist preferences to sessionStorage
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
  <h2 class="text-2xl font-bold mb-6">Requests chờ duyệt</h2>

  {#if loading}
    <div class="flex justify-center py-10">
      <span class="loading loading-spinner"></span>
    </div>
  {:else if error}
    <div class="alert alert-error"><span>{error}</span></div>
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
      <div class="text-center py-10 opacity-60">
        Không có request nào chờ duyệt.
      </div>
    {:else if filteredRequests.length === 0}
      <div class="text-center py-10 opacity-60">
        Không tìm thấy request phù hợp.
      </div>
    {:else}
      <div class="grid gap-4">
        {#each filteredRequests as req}
          <a
            href="#/approvals/{req.id}"
            class="card bg-base-100 shadow hover:shadow-lg transition-shadow"
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
