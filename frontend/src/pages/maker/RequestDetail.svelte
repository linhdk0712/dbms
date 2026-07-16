<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import StatusBadge from "../../components/StatusBadge.svelte";
  import SqlEditor from "../../components/SqlEditor.svelte";

  export let params = {};

  let request = null;
  let loading = true;
  let error = null;
  let actionLoading = false;
  let editing = false;
  let editDescription = "";
  let editSqlContent = "";
  let auditLogs = [];
  let auditLoading = false;

  $: id = params.id;

  onMount(async () => {
    try {
      request = await api.requests.get(id);
      loadAuditLogs();
    } catch (e) {
      error = e.error || "Không tìm thấy request.";
    } finally {
      loading = false;
    }
  });

  async function loadAuditLogs() {
    auditLoading = true;
    try {
      auditLogs = await api.requests.audit(id);
    } catch (e) {
      // Non-blocking — nếu không load được audit thì vẫn hiển thị request
      auditLogs = [];
    } finally {
      auditLoading = false;
    }
  }

  function startEdit() {
    editDescription = request.description;
    editSqlContent = request.sql_content;
    editing = true;
  }

  function cancelEdit() {
    editing = false;
    error = null;
  }

  async function saveEdit() {
    if (!editDescription.trim() || !editSqlContent.trim()) {
      error = "Mô tả và SQL không được để trống.";
      return;
    }
    actionLoading = true;
    error = null;
    try {
      request = await api.requests.update(id, {
        description: editDescription,
        sql_content: editSqlContent,
      });
      editing = false;
    } catch (e) {
      error = e.error || "Không thể lưu thay đổi.";
    } finally {
      actionLoading = false;
    }
  }

  async function handleSubmit() {
    if (!confirm("Gửi request này để Checker review?")) return;
    actionLoading = true;
    error = null;
    try {
      await api.requests.submit(id);
      request = await api.requests.get(id);
    } catch (e) {
      error = e.error || "Không thể submit.";
    } finally {
      actionLoading = false;
    }
  }

  async function handleCancel() {
    if (!confirm("Hủy request này? Bạn có thể tạo request mới.")) return;
    actionLoading = true;
    error = null;
    try {
      await api.requests.cancel(id);
      push("/requests");
    } catch (e) {
      error = e.error || "Không thể hủy.";
    } finally {
      actionLoading = false;
    }
  }

  async function handleClone() {
    actionLoading = true;
    error = null;
    try {
      const cloned = await api.requests.clone(id);
      push(`/requests/${cloned.id}`);
    } catch (e) {
      error = e.error || "Không thể clone request.";
    } finally {
      actionLoading = false;
    }
  }
</script>

<div class="p-6 max-w-4xl mx-auto">
  {#if loading}
    <div class="flex justify-center py-10"><span class="loading loading-spinner loading-lg"></span></div>
  {:else if error && !request}
    <div class="alert alert-error"><span>{error}</span></div>
    <a href="#/requests" class="btn btn-ghost mt-4">← Quay lại</a>
  {:else if request}
    <div class="flex items-center gap-3 mb-6">
      <a href="#/requests" class="btn btn-ghost btn-sm">←</a>
      <h2 class="text-2xl font-bold">Chi tiết Request</h2>
      <StatusBadge status={request.status} />
    </div>

    {#if error}
      <div class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}

    <!-- Thông tin chung -->
    <div class="card bg-base-100 shadow mb-4">
      <div class="card-body">
        <h3 class="font-semibold text-sm opacity-60 uppercase">Mô tả</h3>
        {#if editing}
          <input
            type="text"
            class="input input-bordered w-full"
            bind:value={editDescription}
            placeholder="Mô tả request..."
          />
        {:else}
          <p class="text-lg">{request.description}</p>
        {/if}
        <div class="grid grid-cols-2 gap-4 mt-4 text-sm">
          <div>
            <span class="opacity-60">Ngày tạo:</span>
            <span>{new Date(request.created_at).toLocaleString("vi")}</span>
          </div>
          {#if request.submitted_at}
            <div>
              <span class="opacity-60">Ngày gửi:</span>
              <span>{new Date(request.submitted_at).toLocaleString("vi")}</span>
            </div>
          {/if}
          {#if request.rows_affected !== null && request.rows_affected !== undefined}
            <div>
              <span class="opacity-60">Rows affected:</span>
              <span class="font-mono text-success">{request.rows_affected}</span>
            </div>
          {/if}
        </div>
      </div>
    </div>

    <!-- SQL Content -->
    <div class="card bg-base-100 shadow mb-4">
      <div class="card-body">
        <h3 class="font-semibold text-sm opacity-60 uppercase">SQL Content</h3>
        {#if editing}
          <SqlEditor
            bind:value={editSqlContent}
            placeholder="Nhập SQL..."
            minHeight={200}
          />
        {:else}
          <pre class="bg-base-200 p-4 rounded-lg overflow-x-auto font-mono text-sm mt-2">{request.sql_content}</pre>
        {/if}
      </div>
    </div>

    <!-- Rejection reason nếu bị reject -->
    {#if request.rejection_reason}
      <div class="alert alert-error mb-4">
        <div>
          <p class="font-semibold">Lý do từ chối:</p>
          <p>{request.rejection_reason}</p>
        </div>
      </div>
    {/if}

    <!-- Execution error nếu thất bại -->
    {#if request.execution_error}
      <div class="alert alert-warning mb-4">
        <div>
          <p class="font-semibold">Lỗi thực thi:</p>
          <p class="font-mono text-sm">{request.execution_error}</p>
        </div>
      </div>
    {/if}

    <!-- Actions -->
    <div class="flex gap-3 mt-6">
      {#if editing}
        <button
          class="btn btn-primary"
          on:click={saveEdit}
          disabled={actionLoading}
        >
          {actionLoading ? "Đang lưu..." : "Lưu thay đổi"}
        </button>
        <button class="btn btn-ghost" on:click={cancelEdit}>Hủy sửa</button>
      {:else}
        {#if request.status === "DRAFT"}
          <button
            class="btn btn-outline btn-sm"
            on:click={startEdit}
            disabled={actionLoading}
          >
            ✎ Sửa
          </button>
          <button
            class="btn btn-primary"
            on:click={handleSubmit}
            disabled={actionLoading}
          >
            {actionLoading ? "Đang gửi..." : "Gửi yêu cầu"}
          </button>
        {/if}
        {#if request.status === "PENDING"}
          <button
            class="btn btn-ghost text-error"
            on:click={handleCancel}
            disabled={actionLoading}
          >
            {actionLoading ? "Đang hủy..." : "Hủy request"}
          </button>
        {/if}
        {#if request.status === "REJECTED" || request.status === "FAILED" || request.status === "CANCELLED"}
          <button
            class="btn btn-outline btn-info"
            on:click={handleClone}
            disabled={actionLoading}
          >
            {actionLoading ? "Đang tạo..." : "↻ Clone thành DRAFT mới"}
          </button>
        {/if}
      {/if}
    </div>

    <!-- Audit Timeline -->
    <div class="card bg-base-100 shadow mt-6">
      <div class="card-body">
        <h3 class="font-semibold text-sm opacity-60 uppercase mb-3">Lịch sử hoạt động</h3>
        {#if auditLoading}
          <span class="loading loading-dots loading-sm"></span>
        {:else if auditLogs.length === 0}
          <p class="text-sm opacity-50">Chưa có hoạt động nào.</p>
        {:else}
          <ul class="timeline timeline-vertical timeline-compact">
            {#each auditLogs as log}
              <li>
                <div class="timeline-start text-xs opacity-60">
                  {new Date(log.timestamp).toLocaleString("vi")}
                </div>
                <div class="timeline-middle">
                  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4 text-primary">
                    <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.857-9.809a.75.75 0 00-1.214-.882l-3.483 4.79-1.88-1.88a.75.75 0 10-1.06 1.061l2.5 2.5a.75.75 0 001.137-.089l4-5.5z" clip-rule="evenodd" />
                  </svg>
                </div>
                <div class="timeline-end timeline-box">
                  <span class="badge badge-sm badge-outline mr-2">{log.action}</span>
                  {#if log.detail}
                    <span class="text-xs opacity-70">{log.detail}</span>
                  {/if}
                  {#if log.execution_result !== null && log.execution_result !== undefined}
                    <span class="text-xs font-mono text-success ml-1">({log.execution_result} rows)</span>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>
  {/if}
</div>
