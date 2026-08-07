<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import { toastStore } from "../../stores/toast.js";
  import StatusBadge from "../../components/StatusBadge.svelte";
  import ConfirmModal from "../../components/ConfirmModal.svelte";
  import SqlEditor from "../../components/SqlEditor.svelte";

  let pendingQueries = [];
  let loading = true;
  let selectedQuery = null;
  let showRejectModal = false;
  let rejectComment = "";
  let approveComment = "";
  let processing = false;

  onMount(() => loadPending());

  /**
   * Tải danh sách data query đang chờ phê duyệt từ API.
   * @returns {Promise<void>}
   */
  async function loadPending() {
    loading = true;
    try {
      const result = await api.dataQueryApprovals.pending();
      pendingQueries = Array.isArray(result) ? result : [];
    } catch (e) {
      toastStore.error("Không thể tải danh sách chờ phê duyệt.");
      pendingQueries = [];
    } finally {
      loading = false;
    }
  }

  /**
   * Chọn một data query để xem chi tiết và reset các trường comment.
   * @param {object} q - Data query object được chọn
   */
  function selectQuery(q) {
    selectedQuery = q;
    approveComment = "";
    rejectComment = "";
  }

  /**
   * Xử lý phê duyệt data query đang được chọn.
   * Gửi request approve lên API, hiển thị toast kết quả và reload danh sách.
   * @returns {Promise<void>}
   */
  async function handleApprove() {
    if (!selectedQuery) return;
    processing = true;
    try {
      await api.dataQueryApprovals.approve(selectedQuery.id, approveComment.trim() || undefined);
      toastStore.success("Đã phê duyệt data query.");
      selectedQuery = null;
      await loadPending();
    } catch (e) {
      toastStore.error(e.error || "Không thể phê duyệt.");
    } finally {
      processing = false;
    }
  }

  /**
   * Xử lý từ chối data query đang được chọn.
   * Validate comment tối thiểu, gửi request reject lên API.
   * @returns {Promise<void>}
   */
  async function handleReject() {
    if (!selectedQuery || !rejectComment.trim()) {
      toastStore.warning("Vui lòng nhập lý do từ chối.");
      return;
    }
    processing = true;
    try {
      await api.dataQueryApprovals.reject(selectedQuery.id, rejectComment.trim());
      toastStore.success("Đã từ chối data query.");
      selectedQuery = null;
      showRejectModal = false;
      await loadPending();
    } catch (e) {
      toastStore.error(e.error || "Không thể từ chối.");
    } finally {
      processing = false;
    }
  }

  /**
   * Format chuỗi ngày ISO thành định dạng hiển thị tiếng Việt.
   * @param {string|null} dateStr - Chuỗi ngày ISO 8601
   * @returns {string} Ngày đã format hoặc "—" nếu không có giá trị
   */
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
  <div class="mb-6">
    <h2 class="text-2xl font-bold">Phê duyệt Data Query</h2>
    <p class="text-sm text-base-content/60 mt-0.5">Danh sách yêu cầu truy vấn dữ liệu chờ phê duyệt</p>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if pendingQueries.length === 0}
    <div class="text-center py-12 text-base-content/50">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-3 opacity-30" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <p>Không có yêu cầu nào chờ phê duyệt.</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- List -->
      <div class="space-y-3">
        {#each pendingQueries as q}
          <div
            class="card bg-base-100 border cursor-pointer transition-all hover:border-primary"
            class:border-primary={selectedQuery?.id === q.id}
            on:click={() => selectQuery(q)}
            on:keydown={(e) => e.key === "Enter" && selectQuery(q)}
            role="button"
            tabindex="0"
          >
            <div class="card-body p-4">
              <div class="flex items-start justify-between">
                <div>
                  <h4 class="font-medium">{q.title}</h4>
                  <p class="text-xs text-base-content/60 mt-0.5">
                    {q.creator_name || "Unknown"} • {q.database_name || ""} • {q.ticket_id}
                  </p>
                </div>
                <div class="text-right">
                  <span class="badge badge-sm badge-warning">Level {q.current_approval_level}</span>
                  <span class="badge badge-sm badge-ghost ml-1">{q.approver_unit}</span>
                </div>
              </div>
              <p class="text-xs text-base-content/50 mt-1">{formatDate(q.created_at)}</p>
            </div>
          </div>
        {/each}
      </div>

      <!-- Detail panel -->
      <div>
        {#if selectedQuery}
          <div class="card bg-base-100 border border-base-300 sticky top-4">
            <div class="card-body p-4 space-y-4">
              <div>
                <h3 class="font-bold text-lg">{selectedQuery.title}</h3>
                <div class="flex gap-2 mt-1">
                  <span class="badge badge-sm badge-ghost">{selectedQuery.query_mode}</span>
                  <span class="badge badge-sm badge-outline">{selectedQuery.ticket_id}</span>
                </div>
              </div>

              <div class="grid grid-cols-2 gap-2 text-sm">
                <div>
                  <span class="text-base-content/60">Database:</span>
                  <span class="font-medium ml-1">{selectedQuery.database_name}</span>
                </div>
                <div>
                  <span class="text-base-content/60">Schema:</span>
                  <span class="font-medium ml-1">{selectedQuery.schema_name || "—"}</span>
                </div>
                <div>
                  <span class="text-base-content/60">Người tạo:</span>
                  <span class="font-medium ml-1">{selectedQuery.creator_name}</span>
                </div>
                <div>
                  <span class="text-base-content/60">Hết hạn:</span>
                  <span class="font-medium ml-1">{formatDate(selectedQuery.expires_at)}</span>
                </div>
              </div>

              <!-- Tables accessed -->
              {#if selectedQuery.tables_accessed?.length > 0}
                <div>
                  <span class="text-xs text-base-content/60 uppercase font-semibold">Tables accessed:</span>
                  <div class="flex flex-wrap gap-1 mt-1">
                    {#each selectedQuery.tables_accessed as table}
                      <span class="badge badge-xs badge-outline">{table}</span>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- SQL -->
              <div>
                <span class="text-xs text-base-content/60 uppercase font-semibold">SQL Query:</span>
                <div class="mt-1">
                  <SqlEditor value={selectedQuery.sql_query} readonly={true} language="sql" minHeight={120} />
                </div>
              </div>

              <!-- Assigned users -->
              {#if selectedQuery.assigned_users?.length > 0}
                <div>
                  <span class="text-xs text-base-content/60 uppercase font-semibold">Assigned Users:</span>
                  <div class="flex flex-wrap gap-1 mt-1">
                    {#each selectedQuery.assigned_users as user}
                      <span class="badge badge-xs badge-primary">{user.display_name || user.email}</span>
                    {/each}
                  </div>
                </div>
              {/if}

              <!-- Actions -->
              <div class="divider my-1"></div>
              <div class="form-control">
                <label class="label py-0"><span class="label-text text-xs">Comment (tùy chọn)</span></label>
                <input type="text" class="input input-bordered input-sm" placeholder="Ghi chú..." bind:value={approveComment} />
              </div>
              <div class="flex gap-2">
                <button class="btn btn-success btn-sm flex-1" on:click={handleApprove} disabled={processing}>
                  {#if processing}<span class="loading loading-spinner loading-xs"></span>{/if}
                  Phê duyệt
                </button>
                <button class="btn btn-error btn-sm flex-1" on:click={() => (showRejectModal = true)} disabled={processing}>
                  Từ chối
                </button>
              </div>
            </div>
          </div>
        {:else}
          <div class="card bg-base-100 border border-base-300">
            <div class="card-body text-center text-base-content/50 py-12">
              <p>Chọn một yêu cầu để xem chi tiết</p>
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Reject Modal -->
{#if showRejectModal}
  <div class="modal modal-open">
    <div class="modal-box">
      <h3 class="font-bold text-lg">Từ chối Data Query</h3>
      <p class="text-sm text-base-content/60 mt-1">Vui lòng nhập lý do từ chối (tối thiểu 10 ký tự).</p>
      <div class="form-control mt-4">
        <textarea
          class="textarea textarea-bordered"
          rows="3"
          placeholder="Lý do từ chối..."
          bind:value={rejectComment}
        ></textarea>
      </div>
      <div class="modal-action">
        <button class="btn btn-ghost" on:click={() => (showRejectModal = false)}>Hủy</button>
        <button
          class="btn btn-error"
          on:click={handleReject}
          disabled={processing || rejectComment.trim().length < 10}
        >
          {#if processing}<span class="loading loading-spinner loading-xs"></span>{/if}
          Xác nhận từ chối
        </button>
      </div>
    </div>
    <div class="modal-backdrop" on:click={() => (showRejectModal = false)} on:keydown={() => {}} role="presentation"></div>
  </div>
{/if}
