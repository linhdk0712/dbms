<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import StatusBadge from "../../components/StatusBadge.svelte";
  import SqlEditor from "../../components/SqlEditor.svelte";
  import AiAnalysisPanel from "../../components/AiAnalysisPanel.svelte";
  import ConfirmModal from "../../components/ConfirmModal.svelte";
  import Skeleton from "../../components/Skeleton.svelte";
  import { toastStore } from "../../stores/toast.js";
  import { authStore } from "../../stores/auth.js";

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

  /** Thông tin database đích (name, type, host...) */
  let databaseInfo = null;

  // Confirm modal state
  let confirmOpen = false;
  let confirmAction = null;
  let confirmTitle = "";
  let confirmMessage = "";
  let confirmVariant = "primary";
  let confirmLoading = false;

  $: id = params.id;
  $: isChecker = ($authStore?.user?.roles || []).includes("CHECKER");
  $: isAdmin = ($authStore?.user?.roles || []).includes("ADMIN");

  onMount(async () => {
    try {
      request = await api.requests.get(id);
      loadAuditLogs();
      loadDatabaseInfo();
    } catch (e) {
      error = e.error || "Không tìm thấy request.";
    } finally {
      loading = false;
    }
  });

  /** Tải thông tin database đích từ danh sách databases user có quyền. */
  async function loadDatabaseInfo() {
    if (!request?.database_id) return;
    try {
      const dbs = await api.databases.myPermissions();
      databaseInfo = dbs.find((db) => db.id === request.database_id) || null;
    } catch (e) {
      // Không block — trang vẫn hiển thị
    }
  }

  /**
   * Phân tích execution error để hiển thị thân thiện hơn.
   * Đặc biệt hữu ích với MongoDB errors thường dài và khó đọc.
   */
  function parseExecutionError(errorStr) {
    if (!errorStr) return null;

    const result = { raw: errorStr, summary: "", details: [] };

    // MongoDB: "query execution failed: Kind: ..."
    if (errorStr.includes("Server selection timeout") || errorStr.includes("No available servers")) {
      result.summary = "Không thể kết nối đến MongoDB server";
      result.details.push("Server selection timeout — không tìm thấy server khả dụng");

      // Extract address nếu có
      const addrMatch = errorStr.match(/Address:\s*([^,]+)/);
      if (addrMatch) {
        result.details.push(`Address: ${addrMatch[1].trim()}`);
      }

      // Extract I/O error
      const ioMatch = errorStr.match(/I\/O error:\s*([^.]+)/);
      if (ioMatch) {
        result.details.push(`I/O Error: ${ioMatch[1].trim()}`);
      }

      // Extract OS error
      const osMatch = errorStr.match(/\(os error \d+\)/);
      if (osMatch) {
        result.details.push(osMatch[0]);
      }

      return result;
    }

    // MongoDB: authentication failed
    if (errorStr.includes("Authentication failed") || errorStr.includes("auth error")) {
      result.summary = "Xác thực MongoDB thất bại";
      result.details.push("Username hoặc password không đúng");
      return result;
    }

    // MongoDB: command errors
    if (errorStr.includes("command") && errorStr.includes("failed")) {
      result.summary = "MongoDB command thực thi thất bại";
      const codeMatch = errorStr.match(/code:\s*(\d+)/);
      if (codeMatch) {
        result.details.push(`Error code: ${codeMatch[1]}`);
      }
      const msgMatch = errorStr.match(/message:\s*"([^"]+)"/);
      if (msgMatch) {
        result.details.push(msgMatch[1]);
      }
      return result;
    }

    // Generic: connection refused / timeout
    if (errorStr.includes("Connection refused") || errorStr.includes("connection refused")) {
      result.summary = "Không thể kết nối đến database";
      result.details.push("Connection refused — server có thể chưa khởi động hoặc port sai");
      return result;
    }

    if (errorStr.includes("timed out") || errorStr.includes("timeout")) {
      result.summary = "Kết nối timeout";
      result.details.push("Không thể kết nối trong thời gian cho phép");
      return result;
    }

    // Fallback: không parse được — trả raw
    result.summary = "Lỗi thực thi câu lệnh";
    return result;
  }

  /**
   * Loads audit log entries for the current request from the API
   * and populates the auditLogs array for timeline display.
   */
  async function loadAuditLogs() {
    auditLoading = true;
    try {
      auditLogs = await api.requests.audit(id);
    } catch (e) {
      auditLogs = [];
    } finally {
      auditLoading = false;
    }
  }

  /**
   * Enters edit mode, populating form fields with the current request's
   * description and SQL content for inline editing.
   */
  function startEdit() {
    editDescription = request.description;
    editSqlContent = request.sql_content;
    editing = true;
  }

  /** Exits edit mode and clears any displayed error. */
  function cancelEdit() {
    editing = false;
    error = null;
  }

  /**
   * Validates and saves the edited description and SQL content via the API.
   * On success, exits edit mode and refreshes the request data.
   */
  async function saveEdit() {
    if (!editDescription.trim() || !editSqlContent.trim()) {
      toastStore.warning("Mô tả và SQL không được để trống.");
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
      toastStore.success("Đã lưu thay đổi.");
    } catch (e) {
      toastStore.error(e.error || "Không thể lưu thay đổi.");
    } finally {
      actionLoading = false;
    }
  }

  /**
   * Opens a confirmation modal before submitting the request for Checker review.
   * Warns that SQL content cannot be edited after submission.
   */
  function handleSubmit() {
    confirmTitle = "Gửi request";
    confirmMessage = "Gửi request này để Checker review? Sau khi gửi, bạn không thể chỉnh sửa nội dung SQL.";
    confirmVariant = "primary";
    confirmAction = doSubmit;
    confirmOpen = true;
  }

  /**
   * Submits the request to transition it from DRAFT to PENDING status,
   * then refreshes the local request data and closes the modal.
   */
  async function doSubmit() {
    confirmLoading = true;
    try {
      await api.requests.submit(id);
      request = await api.requests.get(id);
      confirmOpen = false;
      toastStore.success("Request đã được gửi để review.");
    } catch (e) {
      toastStore.error(e.error || "Không thể submit.");
    } finally {
      confirmLoading = false;
    }
  }

  /**
   * Opens a confirmation modal before cancelling the request.
   * Informs the user they can still clone the request afterward.
   */
  function handleCancel() {
    confirmTitle = "Hủy request";
    confirmMessage = "Hủy request này? Bạn có thể tạo request mới hoặc clone sau.";
    confirmVariant = "error";
    confirmAction = doCancel;
    confirmOpen = true;
  }

  /**
   * Cancels the request via the API, transitions it to CANCELLED status,
   * and navigates back to the requests list on success.
   */
  async function doCancel() {
    confirmLoading = true;
    try {
      await api.requests.cancel(id);
      confirmOpen = false;
      toastStore.success("Request đã được hủy.");
      push("/requests");
    } catch (e) {
      toastStore.error(e.error || "Không thể hủy.");
    } finally {
      confirmLoading = false;
    }
  }

  /**
   * Clones the current request into a new DRAFT request and navigates
   * to the newly created request's detail page.
   */
  async function handleClone() {
    actionLoading = true;
    error = null;
    try {
      const cloned = await api.requests.clone(id);
      toastStore.success("Đã clone thành request mới.");
      push(`/requests/${cloned.id}`);
    } catch (e) {
      toastStore.error(e.error || "Không thể clone request.");
    } finally {
      actionLoading = false;
    }
  }

  /** Dispatches the stored confirm action when the user confirms the modal. */
  function onConfirm() {
    if (confirmAction) confirmAction();
  }

  /** Closes the confirmation modal and resets all modal state. */
  function onCancelModal() {
    confirmOpen = false;
    confirmAction = null;
    confirmLoading = false;
  }

  /**
   * Opens a confirmation modal before retrying execution of a FAILED request.
   * Only available for Checker/Admin roles.
   */
  function handleRetry() {
    confirmTitle = "Thực thi lại";
    confirmMessage = "Thực thi lại câu SQL này? Hãy đảm bảo đã khắc phục lỗi kết nối hoặc cấu hình trước khi retry.";
    confirmVariant = "warning";
    confirmAction = doRetry;
    confirmOpen = true;
  }

  /**
   * Retries execution of a FAILED request via the approvals retry endpoint.
   * Updates local request data on completion.
   */
  async function doRetry() {
    confirmLoading = true;
    try {
      const result = await api.approvals.retry(id);
      if (result.execution_result?.success) {
        toastStore.success(`Thực thi thành công — ${result.execution_result.rows_affected} rows affected.`);
      } else {
        toastStore.error(`Thực thi lại thất bại: ${result.execution_result?.error || "Unknown error"}`);
      }
      // Reload request data to reflect new status
      request = await api.requests.get(id);
      loadAuditLogs();
      confirmOpen = false;
    } catch (e) {
      toastStore.error(e.error || "Không thể thực thi lại.");
    } finally {
      confirmLoading = false;
    }
  }
</script>

<div class="p-4 lg:p-6 max-w-7xl mx-auto">
  {#if loading}
    <div class="space-y-4">
      <div class="flex items-center gap-3 mb-6">
        <div class="h-8 w-8 bg-base-300 rounded animate-pulse"></div>
        <div class="h-7 w-48 bg-base-300 rounded animate-pulse"></div>
        <div class="h-5 w-20 bg-base-300 rounded animate-pulse"></div>
      </div>
      <Skeleton type="card" />
      <Skeleton type="card" />
    </div>
  {:else if error && !request}
    <div class="alert alert-error"><span>{error}</span></div>
    <a href="#/requests" class="btn btn-ghost mt-4">← Quay lại</a>
  {:else if request}
    <!-- Header -->
    <div class="flex items-center gap-3 mb-5">
      <a href="#/requests" class="btn btn-ghost btn-sm">←</a>
      <h2 class="text-2xl font-bold">Chi tiết Request</h2>
      <StatusBadge status={request.status} />

      <!-- Actions inline trong header -->
      <div class="ml-auto flex gap-2">
        {#if editing}
          <button
            class="btn btn-primary btn-sm"
            on:click={saveEdit}
            disabled={actionLoading}
          >
            {#if actionLoading}<span class="loading loading-spinner loading-xs"></span>{/if}
            Lưu
          </button>
          <button class="btn btn-ghost btn-sm" on:click={cancelEdit}>Hủy sửa</button>
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
              class="btn btn-primary btn-sm"
              on:click={handleSubmit}
              disabled={actionLoading}
            >
              {#if actionLoading}<span class="loading loading-spinner loading-xs"></span>{/if}
              Gửi yêu cầu
            </button>
          {/if}
          {#if request.status === "PENDING"}
            <button
              class="btn btn-ghost btn-sm text-error"
              on:click={handleCancel}
              disabled={actionLoading}
            >
              Hủy request
            </button>
          {/if}
          {#if request.status === "REJECTED" || request.status === "FAILED" || request.status === "CANCELLED"}
            <button
              class="btn btn-outline btn-info btn-sm"
              on:click={handleClone}
              disabled={actionLoading}
            >
              {#if actionLoading}<span class="loading loading-spinner loading-xs"></span>{/if}
              ↻ Clone DRAFT mới
            </button>
          {/if}
          {#if request.status === "FAILED" && (isChecker || isAdmin)}
            <button
              class="btn btn-warning btn-sm"
              on:click={handleRetry}
              disabled={actionLoading}
            >
              {#if actionLoading}<span class="loading loading-spinner loading-xs"></span>{/if}
              ▶ Thực thi lại
            </button>
          {/if}
        {/if}
      </div>
    </div>

    {#if error}
      <div class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}

    <!-- 2-column layout: main content (left) + sidebar (right) -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <!-- CỘT TRÁI: SQL Content + Error/Result + AI Analysis (chiếm 2/3) -->
      <div class="lg:col-span-2 space-y-4">
        <!-- SQL Content -->
        <div class="card bg-base-100 shadow">
          <div class="card-body p-4">
            <h3 class="font-semibold text-sm opacity-60 uppercase">SQL Content</h3>
            {#if editing}
              <SqlEditor
                bind:value={editSqlContent}
                placeholder="Nhập SQL..."
                minHeight={200}
              />
            {:else}
              <pre class="bg-base-200 p-4 rounded-lg overflow-x-auto font-mono text-sm mt-2 max-h-96">{request.sql_content}</pre>
            {/if}
          </div>
        </div>

        <!-- Rejection reason nếu bị reject -->
        {#if request.rejection_reason}
          <div class="alert alert-error">
            <div>
              <p class="font-semibold">Lý do từ chối:</p>
              <p>{request.rejection_reason}</p>
            </div>
          </div>
        {/if}

        <!-- Execution error nếu thất bại -->
        {#if request.execution_error}
          {@const parsed = parseExecutionError(request.execution_error)}
          <div class="alert alert-warning">
            <div class="w-full">
              <div class="flex items-center gap-2 mb-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                <p class="font-semibold">Lỗi thực thi:</p>
                {#if databaseInfo}
                  <span class="badge badge-sm badge-outline ml-auto">{databaseInfo.db_type}</span>
                {/if}
              </div>

              {#if parsed.summary}
                <p class="font-medium text-sm mb-1">{parsed.summary}</p>
              {/if}

              {#if parsed.details.length > 0}
                <ul class="list-disc ml-5 text-sm space-y-0.5 mb-2">
                  {#each parsed.details as detail}
                    <li>{detail}</li>
                  {/each}
                </ul>
              {/if}

              <!-- Raw error (collapsible) -->
              <details class="mt-2">
                <summary class="text-xs opacity-70 cursor-pointer hover:opacity-100">
                  Xem chi tiết lỗi gốc
                </summary>
                <pre class="mt-1 p-2 bg-base-200 rounded text-xs font-mono whitespace-pre-wrap break-all max-h-40 overflow-y-auto">{request.execution_error}</pre>
              </details>
            </div>
          </div>
        {/if}

        <!-- AI Analysis Panel — hiển thị cho PENDING, DRAFT (sau khi có SQL) -->
        {#if request.status === "PENDING" || request.status === "DRAFT"}
          <AiAnalysisPanel requestId={id} autoLoad={request.status === "PENDING"} />
        {/if}
      </div>

      <!-- CỘT PHẢI: Metadata + Database info + Audit (chiếm 1/3) -->
      <div class="space-y-4">
        <!-- Thông tin chung -->
        <div class="card bg-base-100 shadow">
          <div class="card-body p-4">
            <h3 class="font-semibold text-sm opacity-60 uppercase">Thông tin</h3>
            {#if editing}
              <input
                type="text"
                class="input input-bordered input-sm w-full mt-2"
                bind:value={editDescription}
                placeholder="Mô tả request..."
              />
            {:else}
              <p class="text-base font-medium mt-1">{request.description}</p>
            {/if}
            <div class="divider my-2"></div>
            <div class="space-y-2 text-sm">
              <div class="flex justify-between">
                <span class="opacity-60">Ngày tạo</span>
                <span>{new Date(request.created_at).toLocaleString("vi")}</span>
              </div>
              {#if request.submitted_at}
                <div class="flex justify-between">
                  <span class="opacity-60">Ngày gửi</span>
                  <span>{new Date(request.submitted_at).toLocaleString("vi")}</span>
                </div>
              {/if}
              {#if request.rows_affected !== null && request.rows_affected !== undefined}
                <div class="flex justify-between">
                  <span class="opacity-60">Rows affected</span>
                  <span class="font-mono text-success">{request.rows_affected}</span>
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- Thông tin Database đích -->
        {#if databaseInfo}
          <div class="card bg-base-100 shadow">
            <div class="card-body p-4">
              <h3 class="font-semibold text-sm opacity-60 uppercase">Database đích</h3>
              <div class="space-y-2 mt-2 text-sm">
                <div class="flex justify-between items-center">
                  <span class="opacity-60">Tên</span>
                  <span class="font-medium">{databaseInfo.name}</span>
                </div>
                <div class="flex justify-between items-center">
                  <span class="opacity-60">Loại</span>
                  <span class="badge badge-sm badge-outline">{databaseInfo.db_type}</span>
                </div>
                <div class="flex justify-between items-center">
                  <span class="opacity-60">Host</span>
                  <span class="font-mono text-xs">{databaseInfo.host}:{databaseInfo.port}</span>
                </div>
                <div class="flex justify-between items-center">
                  <span class="opacity-60">Database</span>
                  <span class="font-mono text-xs text-right">{databaseInfo.database_name}{databaseInfo.schema_name ? ` / ${databaseInfo.schema_name}` : ''}</span>
                </div>
              </div>
            </div>
          </div>
        {/if}

        <!-- Audit Timeline -->
        <div class="card bg-base-100 shadow">
          <div class="card-body p-4">
            <h3 class="font-semibold text-sm opacity-60 uppercase mb-2">Lịch sử</h3>
            {#if auditLoading}
              <span class="loading loading-dots loading-sm"></span>
            {:else if auditLogs.length === 0}
              <p class="text-sm opacity-50">Chưa có hoạt động nào.</p>
            {:else}
              <ul class="steps steps-vertical text-sm">
                {#each auditLogs as log}
                  <li class="step step-primary">
                    <div class="text-left">
                      <span class="badge badge-sm badge-outline">{log.action}</span>
                      <span class="text-xs opacity-60 ml-1">{new Date(log.timestamp).toLocaleString("vi")}</span>
                      {#if log.detail}
                        <p class="text-xs opacity-70 mt-0.5">{log.detail}</p>
                      {/if}
                      {#if log.execution_result !== null && log.execution_result !== undefined}
                        <span class="text-xs font-mono text-success">({log.execution_result} rows)</span>
                      {/if}
                    </div>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>

<!-- Confirm Modal -->
<ConfirmModal
  bind:open={confirmOpen}
  title={confirmTitle}
  message={confirmMessage}
  variant={confirmVariant}
  loading={confirmLoading}
  on:confirm={onConfirm}
  on:cancel={onCancelModal}
/>
