<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import StatusBadge from "../../components/StatusBadge.svelte";
  import SqlEditor from "../../components/SqlEditor.svelte";
  import AiAnalysisPanel from "../../components/AiAnalysisPanel.svelte";
  import CrBadge from "../../components/CrBadge.svelte";
  import MaintenanceWindow from "../../components/MaintenanceWindow.svelte";
  import ConfirmModal from "../../components/ConfirmModal.svelte";
  import Skeleton from "../../components/Skeleton.svelte";
  import { toastStore } from "../../stores/toast.js";

  export let params = {};

  let request = null;
  let loading = true;
  let error = null;
  let rejectionReason = "";
  let actionLoading = false;
  let executionResult = null;
  let crInfo = null;

  // Validation state
  let validating = false;
  let validationResult = null;
  let validated = false;

  // Confirm modal state
  let confirmOpen = false;
  let confirmAction = null;
  let confirmTitle = "";
  let confirmMessage = "";
  let confirmVariant = "primary";
  let confirmLoading = false;

  $: id = params.id;

  onMount(async () => {
    try {
      request = await api.approvals.get(id);
      // Khôi phục validation result đã lưu từ server (nếu có)
      if (request.validation_result) {
        validationResult = request.validation_result;
        validated = request.validation_result.valid === true;
      }
      if (request.cr_id) {
        try {
          crInfo = await api.changeRequests.get(request.cr_id);
        } catch (_) {
          crInfo = null;
        }
      }
    } catch (e) {
      error = e.error || "Không tìm thấy request.";
    } finally {
      loading = false;
    }
  });

  /**
   * Validates the SQL content of the current request before approval.
   * Updates validationResult and validated state based on API response.
   */
  async function handleValidate() {
    validating = true;
    error = null;
    validationResult = null;
    validated = false;
    try {
      const result = await api.approvals.validate(id);
      validationResult = result;
      validated = result.valid === true;
    } catch (e) {
      toastStore.error(e.error || "Không thể validate SQL.");
    } finally {
      validating = false;
    }
  }

  /**
   * Opens the confirmation modal for approving the request.
   * Requires SQL validation to pass before allowing approval.
   * Adjusts confirmation message for multi-level approval flows.
   */
  function handleApprove() {
    if (!validated) {
      toastStore.warning("Phải validate SQL thành công trước khi Approve & Execute.");
      return;
    }
    const isMultiLevel = request.approval_levels_required > 1;
    const isFinalApproval = isMultiLevel && (request.approval_count || 0) + 1 >= request.approval_levels_required;
    confirmTitle = "Approve & Execute";
    confirmMessage = isMultiLevel && !isFinalApproval
      ? `Xác nhận APPROVE request này? (${(request.approval_count || 0) + 1}/${request.approval_levels_required} approvals)`
      : "Xác nhận APPROVE request này? SQL sẽ được thực thi ngay trên database đích.";
    confirmVariant = "success";
    confirmAction = doApprove;
    confirmOpen = true;
  }

  /**
   * Executes the approval action via API.
   * On final approval, displays execution result inline.
   * On intermediate approval (multi-level), redirects to approvals list.
   */
  async function doApprove() {
    confirmLoading = true;
    error = null;
    executionResult = null;
    try {
      const result = await api.approvals.approve(id);
      confirmOpen = false;
      if (result.execution_result) {
        executionResult = result.execution_result;
        request.status = result.status;
        if (result.execution_result.success) {
          toastStore.success("SQL đã thực thi thành công.");
        } else {
          toastStore.error("SQL thực thi thất bại.");
        }
      } else {
        toastStore.success("Request đã được approve.");
        push("/approvals");
      }
    } catch (e) {
      toastStore.error(e.error || "Không thể approve.");
    } finally {
      confirmLoading = false;
    }
  }

  /**
   * Rejects the current request with the provided reason.
   * Requires rejection reason to be at least 10 characters.
   * Redirects to approvals list on success.
   */
  async function handleReject() {
    if (rejectionReason.length < 10) {
      toastStore.warning("Lý do reject phải ít nhất 10 ký tự.");
      return;
    }
    actionLoading = true;
    error = null;
    try {
      await api.approvals.reject(id, rejectionReason);
      toastStore.info("Request đã bị reject.");
      push("/approvals");
    } catch (e) {
      toastStore.error(e.error || "Không thể reject.");
    } finally {
      actionLoading = false;
    }
  }

  /** Handles confirm modal confirmation by invoking the stored action callback. */
  function onConfirm() {
    if (confirmAction) confirmAction();
  }

  /** Closes the confirm modal and resets its state. */
  function onCancelModal() {
    confirmOpen = false;
    confirmAction = null;
    confirmLoading = false;
  }

  /**
   * Opens a confirmation modal before retrying execution of a FAILED request.
   */
  function handleRetry() {
    confirmTitle = "Thực thi lại";
    confirmMessage = "Thực thi lại câu SQL này? Hãy đảm bảo đã khắc phục lỗi kết nối hoặc cấu hình trước khi retry.";
    confirmVariant = "warning";
    confirmAction = doRetry;
    confirmOpen = true;
  }

  /**
   * Retries execution of a FAILED request.
   * Updates local request data and execution result on completion.
   */
  async function doRetry() {
    confirmLoading = true;
    try {
      const result = await api.approvals.retry(id);
      confirmOpen = false;
      executionResult = result.execution_result;
      request.status = result.status;
      if (result.execution_result?.success) {
        toastStore.success(`Thực thi thành công — ${result.execution_result.rows_affected} rows affected.`);
      } else {
        toastStore.error(`Thực thi lại thất bại: ${result.execution_result?.error || "Unknown error"}`);
      }
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
        <div class="h-7 w-48 bg-base-300 rounded animate-pulse"></div>
        <div class="h-5 w-20 bg-base-300 rounded animate-pulse"></div>
      </div>
      <Skeleton type="card" />
      <Skeleton type="card" />
      <Skeleton type="card" />
    </div>
  {:else if error && !request}
    <div class="alert alert-error"><span>{error}</span></div>
  {:else if request}
    <!-- Header -->
    <div class="flex items-center gap-3 mb-5">
      <a href="#/approvals" class="btn btn-ghost btn-sm">←</a>
      <h2 class="text-2xl font-bold">Review Request</h2>
      <StatusBadge status={request.status} />
      {#if request.approval_levels_required > 1}
        <span class="badge badge-info badge-sm ml-2">{request.approval_count || 0}/{request.approval_levels_required} approvals</span>
      {/if}
    </div>

    {#if error}
      <div class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}

    <!-- 2-column layout -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <!-- CỘT TRÁI: SQL + DDL + Validation + AI + Result (2/3) -->
      <div class="lg:col-span-2 space-y-4">
        <!-- SQL Content -->
        <div class="card bg-base-100 shadow">
          <div class="card-body p-4">
            <h3 class="font-semibold text-sm opacity-60 uppercase">SQL Content</h3>
            <SqlEditor value={request.sql_content} readonly={true} minHeight={200} />
          </div>
        </div>

        <!-- DDL Warning Banner (BR-005) -->
        {#if request.sql_analysis && request.sql_analysis.contains_ddl}
          <div class="alert {request.sql_analysis.has_danger ? 'alert-error' : 'alert-warning'}">
            <div>
              <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
              <div>
                <h3 class="font-bold">
                  {request.sql_analysis.has_danger ? '⚠️ DDL NGUY HIỂM' : '⚠️ DDL Statement'}
                </h3>
                <p class="text-sm">Kiểm tra kỹ trước khi approve.</p>
              </div>
            </div>
          </div>

          <div class="card bg-base-100 shadow">
            <div class="card-body p-4">
              <h3 class="font-semibold text-sm opacity-60 uppercase mb-2">DDL Analysis</h3>
              <div class="overflow-x-auto">
                <table class="table table-sm">
                  <thead><tr><th>Keyword</th><th>Mức độ</th><th>Mô tả</th></tr></thead>
                  <tbody>
                    {#each request.sql_analysis.ddl_warnings as warning}
                      <tr>
                        <td><code class="font-mono text-sm">{warning.keyword}</code></td>
                        <td>
                          {#if warning.severity === "DANGER"}
                            <span class="badge badge-error badge-sm">DANGER</span>
                          {:else}
                            <span class="badge badge-warning badge-sm">WARNING</span>
                          {/if}
                        </td>
                        <td class="text-sm">{warning.message}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        {/if}

        <!-- Validation Result -->
        {#if validationResult}
          <div class="card bg-base-100 shadow">
            <div class="card-body p-4">
              {#if validationResult.valid}
                <div class="alert alert-success">
                  <span>✓ SQL hợp lệ — sẵn sàng approve & execute</span>
                </div>
              {:else}
                <div class="alert alert-error">
                  <span>✗ SQL không hợp lệ: {validationResult.error}</span>
                </div>
              {/if}

              {#if validationResult.warnings && validationResult.warnings.length > 0}
                <div class="alert alert-warning mt-3">
                  <div>
                    <h4 class="font-semibold">⚠️ Performance Warnings</h4>
                    <ul class="list-disc list-inside text-sm mt-1">
                      {#each validationResult.warnings as warning}
                        <li>{warning}</li>
                      {/each}
                    </ul>
                  </div>
                </div>
              {/if}

              {#if validationResult.execution_plan && validationResult.execution_plan.length > 0}
                <details class="collapse collapse-arrow bg-base-200 mt-3">
                  <summary class="collapse-title font-medium text-sm">
                    Execution Plan ({validationResult.execution_plan.length} lines)
                  </summary>
                  <div class="collapse-content">
                    <pre class="text-xs overflow-x-auto bg-base-300 p-3 rounded">{validationResult.execution_plan.join('\n')}</pre>
                  </div>
                </details>
              {/if}
            </div>
          </div>
        {/if}

        <!-- AI Analysis Panel -->
        {#if request.status === "PENDING"}
          <AiAnalysisPanel requestId={id} autoLoad={false} />
        {/if}

        <!-- Execution Result -->
        {#if executionResult}
          <div class="card bg-base-100 shadow">
            <div class="card-body p-4">
              <h3 class="font-semibold mb-3 flex items-center gap-2">
                {#if executionResult.success}
                  <span class="text-success">✓ Thực thi thành công</span>
                {:else}
                  <span class="text-error">✗ Thực thi thất bại</span>
                {/if}
              </h3>
              {#if executionResult.success}
                <div class="stats shadow">
                  <div class="stat">
                    <div class="stat-title">Rows Affected</div>
                    <div class="stat-value text-success">{executionResult.rows_affected}</div>
                  </div>
                </div>
              {:else}
                <div class="alert alert-error">
                  <span>{executionResult.error}</span>
                </div>
              {/if}
              <div class="mt-4 flex gap-3">
                <button class="btn btn-outline btn-sm" on:click={() => push("/approvals")}>← Quay lại</button>
                {#if !executionResult.success}
                  <button class="btn btn-warning btn-sm" on:click={handleRetry} disabled={actionLoading}>▶ Thực thi lại</button>
                {/if}
              </div>
            </div>
          </div>
        {/if}

        <!-- Retry block cho FAILED -->
        {#if request.status === "FAILED" && !executionResult}
          <div class="card bg-base-100 shadow border-l-4 border-warning">
            <div class="card-body p-4">
              <h3 class="font-semibold mb-2 flex items-center gap-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-warning" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
                Thực thi thất bại
              </h3>
              {#if request.execution_error}
                <pre class="text-sm bg-base-200 p-3 rounded font-mono whitespace-pre-wrap break-all max-h-40 overflow-y-auto mb-3">{request.execution_error}</pre>
              {/if}
              <button class="btn btn-warning" on:click={handleRetry} disabled={actionLoading}>
                {#if actionLoading}<span class="loading loading-spinner loading-sm"></span>{/if}
                ▶ Thực thi lại
              </button>
            </div>
          </div>
        {/if}
      </div>

      <!-- CỘT PHẢI: Metadata + Mô tả + CR + Actions (1/3) -->
      <div class="space-y-4">
        <!-- Thông tin request -->
        <div class="card bg-base-100 shadow">
          <div class="card-body p-4">
            <h3 class="font-semibold text-sm opacity-60 uppercase">Thông tin</h3>
            <div class="space-y-2 mt-2 text-sm">
              <div class="flex justify-between">
                <span class="opacity-60">Ngày tạo</span>
                <span>{new Date(request.created_at).toLocaleString('vi-VN')}</span>
              </div>
              <div class="flex justify-between">
                <span class="opacity-60">Ngày gửi</span>
                <span>{request.submitted_at ? new Date(request.submitted_at).toLocaleString('vi-VN') : '—'}</span>
              </div>
              <div class="divider my-1"></div>
              <div class="flex justify-between items-center">
                <span class="opacity-60">Database</span>
                <span class="font-medium text-right">{request.database_name || request.database_id}</span>
              </div>
              {#if request.database_connection_string}
                <p class="font-mono text-xs opacity-70">{request.database_connection_string}</p>
              {/if}
            </div>
          </div>
        </div>

        <!-- Mô tả -->
        <div class="card bg-base-100 shadow">
          <div class="card-body p-4">
            <h3 class="font-semibold text-sm opacity-60 uppercase">Mô tả</h3>
            <p class="mt-1">{request.description}</p>
          </div>
        </div>

        <!-- CR Info -->
        {#if crInfo}
          <div class="card bg-base-100 shadow border-l-4 border-info">
            <div class="card-body p-4">
              <h3 class="font-semibold text-sm opacity-60 uppercase">Change Request</h3>
              <div class="flex items-center gap-2 mt-2 flex-wrap">
                <CrBadge crNumber={crInfo.cr_number} crUrl={crInfo.cr_url} />
                {#if crInfo.status}
                  <span class="badge badge-outline badge-xs">{crInfo.status}</span>
                {/if}
              </div>
              {#if crInfo.title}
                <p class="text-sm mt-1">{crInfo.title}</p>
              {/if}
              {#if crInfo.scheduled_start_time && crInfo.scheduled_end_time}
                <div class="mt-2">
                  <MaintenanceWindow startTime={crInfo.scheduled_start_time} endTime={crInfo.scheduled_end_time} />
                </div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Actions Panel (PENDING only) -->
        {#if request.status === "PENDING"}
          <div class="card bg-base-100 shadow">
            <div class="card-body p-4">
              <h3 class="font-semibold text-sm opacity-60 uppercase mb-3">Hành động</h3>
              <button
                class="btn btn-outline btn-info btn-sm w-full"
                on:click={handleValidate}
                disabled={validating || actionLoading}
              >
                {#if validating}
                  <span class="loading loading-spinner loading-xs"></span> Đang validate...
                {:else}
                  🔍 Validate SQL
                {/if}
              </button>
              {#if !validated}
                <p class="text-xs text-warning mt-2">⚠️ Validate trước khi approve</p>
              {/if}
              <button
                class="btn btn-success btn-sm w-full mt-3"
                on:click={handleApprove}
                disabled={!validated || actionLoading}
              >
                {#if actionLoading}
                  <span class="loading loading-spinner loading-xs"></span>
                {:else if request.approval_levels_required > 1}
                  {#if (request.approval_count || 0) + 1 >= request.approval_levels_required}
                    ✓ Approve & Execute
                  {:else}
                    ✓ Approve ({(request.approval_count || 0) + 1}/{request.approval_levels_required})
                  {/if}
                {:else}
                  ✓ Approve & Execute
                {/if}
              </button>
              <div class="divider my-3 text-xs">HOẶC</div>
              <textarea
                class="textarea textarea-bordered textarea-sm w-full"
                placeholder="Lý do reject (≥10 ký tự)..."
                bind:value={rejectionReason}
                rows="3"
              ></textarea>
              <button
                class="btn btn-error btn-sm w-full mt-2"
                on:click={handleReject}
                disabled={actionLoading || rejectionReason.length < 10}
              >
                ✗ Reject
              </button>
            </div>
          </div>
        {/if}
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
