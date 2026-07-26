<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";
  import StatusBadge from "../../components/StatusBadge.svelte";
  import Skeleton from "../../components/Skeleton.svelte";
  import ConfirmModal from "../../components/ConfirmModal.svelte";
  import { toastStore } from "../../stores/toast.js";

  let requests = [];
  let loading = true;
  let error = null;
  let statusFilter = "";
  let pagination = { page: 1, per_page: 20, total: 0, total_pages: 0 };

  // Confirm modal state
  let confirmOpen = false;
  let confirmAction = null;
  let confirmTitle = "";
  let confirmMessage = "";
  let confirmVariant = "primary";
  let confirmLoading = false;

  onMount(() => loadRequests());

  /**
   * Tải danh sách requests của user hiện tại từ API.
   * Hỗ trợ filter theo status và phân trang.
   */
  async function loadRequests() {
    loading = true;
    error = null;
    try {
      const response = await api.requests.list({
        status: statusFilter || undefined,
        page: pagination.page,
        per_page: pagination.per_page,
      });
      if (response.data && response.pagination) {
        requests = response.data;
        pagination = response.pagination;
      } else {
        requests = Array.isArray(response) ? response : [];
      }
    } catch (e) {
      error = e.error || "Không thể tải danh sách.";
    } finally {
      loading = false;
    }
  }

  /** Xử lý khi user thay đổi bộ lọc trạng thái — reset về trang 1 và reload. */
  function onFilterChange() {
    pagination.page = 1;
    loadRequests();
  }

  /**
   * Chuyển đến trang được chỉ định và reload danh sách.
   * @param {number} page - Số trang cần chuyển đến
   */
  function goToPage(page) {
    pagination.page = page;
    loadRequests();
  }

  /**
   * Mở confirm modal để xác nhận hủy request.
   * @param {string} id - ID của request cần hủy
   */
  function requestCancel(id) {
    confirmTitle = "Hủy request";
    confirmMessage = "Bạn chắc chắn muốn hủy request này? Hành động này không thể hoàn tác.";
    confirmVariant = "error";
    confirmAction = () => doCancel(id);
    confirmOpen = true;
  }

  /**
   * Thực hiện hủy request sau khi user xác nhận qua modal.
   * @param {string} id - ID của request cần hủy
   */
  async function doCancel(id) {
    confirmLoading = true;
    try {
      await api.requests.cancel(id);
      confirmOpen = false;
      toastStore.success("Request đã được hủy.");
      loadRequests();
    } catch (e) {
      toastStore.error(e.error || "Không thể hủy request.");
    } finally {
      confirmLoading = false;
    }
  }

  /**
   * Mở confirm modal để xác nhận gửi request cho Checker review.
   * @param {string} id - ID của request cần submit
   */
  function requestSubmit(id) {
    confirmTitle = "Gửi request";
    confirmMessage = "Gửi request này để Checker review? Sau khi gửi, bạn không thể chỉnh sửa nội dung SQL.";
    confirmVariant = "primary";
    confirmAction = () => doSubmit(id);
    confirmOpen = true;
  }

  /**
   * Thực hiện submit request sau khi user xác nhận qua modal.
   * @param {string} id - ID của request cần submit
   */
  async function doSubmit(id) {
    confirmLoading = true;
    try {
      await api.requests.submit(id);
      confirmOpen = false;
      toastStore.success("Request đã được gửi để review.");
      loadRequests();
    } catch (e) {
      toastStore.error(e.error || "Không thể gửi request.");
    } finally {
      confirmLoading = false;
    }
  }

  /** Callback khi user nhấn nút xác nhận trong ConfirmModal — thực thi action đã lưu. */
  function handleConfirm() {
    if (confirmAction) confirmAction();
  }

  /** Callback khi user đóng hoặc hủy ConfirmModal — reset modal state. */
  function handleCancelModal() {
    confirmOpen = false;
    confirmAction = null;
    confirmLoading = false;
  }
</script>

<div class="p-6">
  <div class="flex justify-between items-center mb-6">
    <div>
      <h2 class="text-2xl font-bold">Requests của tôi</h2>
      <p class="text-sm text-base-content/60 mt-1">Danh sách và trạng thái các SQL request đã tạo</p>
    </div>
    <a href="#/requests/new" class="btn btn-primary btn-sm gap-1">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
      </svg>
      Tạo mới
    </a>
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
    <Skeleton type="table" rows={8} cols={4} />
  {:else if error}
    <div class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{error}</span>
      <button class="btn btn-sm btn-ghost" on:click={loadRequests}>Thử lại</button>
    </div>
  {:else if requests.length === 0}
    <div class="text-center py-16">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 mx-auto text-base-content/20 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
      <p class="text-base-content/50 mb-4">Chưa có request nào.</p>
      <a href="#/requests/new" class="btn btn-primary btn-sm">Tạo request đầu tiên</a>
    </div>
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
            <tr class="hover">
              <td>
                <a href="#/requests/{req.id}" class="link link-hover font-medium">{req.description}</a>
              </td>
              <td><StatusBadge status={req.status} /></td>
              <td class="text-sm text-base-content/60">{new Date(req.created_at).toLocaleDateString("vi")}</td>
              <td class="flex gap-1">
                {#if req.status === "DRAFT"}
                  <button
                    class="btn btn-xs btn-primary"
                    on:click={() => requestSubmit(req.id)}
                  >
                    Submit
                  </button>
                {/if}
                {#if req.status === "PENDING"}
                  <button
                    class="btn btn-xs btn-ghost text-error"
                    on:click={() => requestCancel(req.id)}
                  >
                    Hủy
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    {#if pagination.total_pages > 1}
      <div class="flex justify-center items-center gap-1 mt-6">
        <button
          class="btn btn-sm btn-ghost"
          disabled={pagination.page <= 1}
          on:click={() => goToPage(pagination.page - 1)}
        >
          ← Trước
        </button>

        {#each Array(Math.min(pagination.total_pages, 7)) as _, i}
          {@const pageNum = i + 1}
          <button
            class="btn btn-sm {pagination.page === pageNum ? 'btn-active' : 'btn-ghost'}"
            on:click={() => goToPage(pageNum)}
          >
            {pageNum}
          </button>
        {/each}

        {#if pagination.total_pages > 7}
          <span class="px-2">...</span>
          <button
            class="btn btn-sm btn-ghost"
            on:click={() => goToPage(pagination.total_pages)}
          >
            {pagination.total_pages}
          </button>
        {/if}

        <button
          class="btn btn-sm btn-ghost"
          disabled={pagination.page >= pagination.total_pages}
          on:click={() => goToPage(pagination.page + 1)}
        >
          Sau →
        </button>

        <span class="text-xs text-base-content/50 ml-2">
          ({pagination.total} kết quả)
        </span>
      </div>
    {/if}
  {/if}
</div>

<!-- Confirm Modal -->
<ConfirmModal
  bind:open={confirmOpen}
  title={confirmTitle}
  message={confirmMessage}
  variant={confirmVariant}
  loading={confirmLoading}
  on:confirm={handleConfirm}
  on:cancel={handleCancelModal}
/>
