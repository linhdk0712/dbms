<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import { toastStore } from "../../stores/toast.js";
  import StatusBadge from "../../components/StatusBadge.svelte";

  let queries = [];
  let loading = true;
  let statusFilter = "";
  let page = 1;
  let pagination = null;

  const statusOptions = [
    { value: "", label: "Tất cả" },
    { value: "DRAFT", label: "Bản nháp" },
    { value: "PENDING_APPROVAL", label: "Chờ duyệt" },
    { value: "APPROVED", label: "Đã duyệt" },
    { value: "REJECTED", label: "Bị từ chối" },
    { value: "EXPIRED", label: "Hết hạn" },
    { value: "REVOKED", label: "Đã thu hồi" },
  ];

  onMount(() => loadQueries());

  async function loadQueries() {
    loading = true;
    try {
      const result = await api.dataQueries.list({ status: statusFilter || undefined, page });
      if (result?.data) {
        queries = result.data;
        pagination = result.pagination;
      } else {
        queries = Array.isArray(result) ? result : [];
      }
    } catch (e) {
      toastStore.error("Không thể tải danh sách data queries.");
      queries = [];
    } finally {
      loading = false;
    }
  }

  function handleFilterChange() {
    page = 1;
    loadQueries();
  }

  function goToPage(p) {
    page = p;
    loadQueries();
  }

  function formatDate(dateStr) {
    if (!dateStr) return "—";
    return new Date(dateStr).toLocaleDateString("vi-VN", {
      day: "2-digit",
      month: "2-digit",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  }
</script>

<div class="p-6">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-2xl font-bold">Data Queries</h2>
      <p class="text-sm text-base-content/60 mt-0.5">Danh sách các yêu cầu truy vấn dữ liệu</p>
    </div>
    <a href="#/data-queries/new" class="btn btn-primary">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
      </svg>
      Tạo Query mới
    </a>
  </div>

  <!-- Filters -->
  <div class="flex gap-3 mb-4">
    <select class="select select-bordered select-sm" bind:value={statusFilter} on:change={handleFilterChange}>
      {#each statusOptions as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </div>

  <!-- Table -->
  {#if loading}
    <div class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if queries.length === 0}
    <div class="text-center py-12 text-base-content/50">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-3 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
      <p>Chưa có data query nào.</p>
    </div>
  {:else}
    <div class="overflow-x-auto">
      <table class="table table-sm">
        <thead>
          <tr>
            <th>Tiêu đề</th>
            <th>Database</th>
            <th>Mode</th>
            <th>Ticket</th>
            <th>Trạng thái</th>
            <th>Hết hạn</th>
            <th>Ngày tạo</th>
          </tr>
        </thead>
        <tbody>
          {#each queries as q}
            <tr class="hover cursor-pointer" on:click={() => push(`/data-queries/${q.id}`)}>
              <td class="font-medium">{q.title}</td>
              <td class="text-sm">{q.database_name || "—"}</td>
              <td>
                <span class="badge badge-sm badge-ghost">{q.query_mode}</span>
              </td>
              <td class="text-sm font-mono">{q.ticket_id}</td>
              <td><StatusBadge status={q.status} size="sm" /></td>
              <td class="text-sm">{formatDate(q.expires_at)}</td>
              <td class="text-sm">{formatDate(q.created_at)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    {#if pagination && pagination.total_pages > 1}
      <div class="flex justify-center mt-4">
        <div class="join">
          {#each Array(pagination.total_pages) as _, i}
            <button
              class="join-item btn btn-sm"
              class:btn-active={page === i + 1}
              on:click={() => goToPage(i + 1)}
            >
              {i + 1}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>
