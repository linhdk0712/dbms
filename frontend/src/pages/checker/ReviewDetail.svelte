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
  let rejectionReason = "";
  let actionLoading = false;
  let executionResult = null;

  // Validation state
  let validating = false;
  let validationResult = null; // { valid, execution_plan, warnings, error }
  let validated = false; // true khi validate thành công

  $: id = params.id;

  onMount(async () => {
    try {
      request = await api.approvals.get(id);
    } catch (e) {
      error = e.error || "Không tìm thấy request.";
    } finally {
      loading = false;
    }
  });

  /**
   * Validates the SQL content of the request against the target database.
   * Calls the backend validate API and updates validation state (validationResult, validated).
   * Must succeed before the Checker can approve/execute.
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
      if (!result.valid) {
        error = null; // Error will be shown in validation result section
      }
    } catch (e) {
      error = e.error || "Không thể validate SQL.";
    } finally {
      validating = false;
    }
  }

  /**
   * Approves the pending request and triggers SQL execution on the target database.
   * Requires prior successful validation. Handles both partial (multi-level) and
   * final approvals. On final approval, displays execution result inline.
   */
  async function handleApprove() {
    if (!validated) {
      error = "Phải validate SQL thành công trước khi Approve & Execute.";
      return;
    }
    const isMultiLevel = request.approval_levels_required > 1;
    const isFinalApproval = isMultiLevel && (request.approval_count || 0) + 1 >= request.approval_levels_required;
    const confirmMsg = isMultiLevel && !isFinalApproval
      ? `Xác nhận APPROVE request này? (${(request.approval_count || 0) + 1}/${request.approval_levels_required} approvals)`
      : "Xác nhận APPROVE request này? SQL sẽ được thực thi ngay.";
    if (!confirm(confirmMsg)) return;
    actionLoading = true;
    error = null;
    executionResult = null;
    try {
      const result = await api.approvals.approve(id);
      if (result.execution_result) {
        // Final approval with execution — show result inline
        executionResult = result.execution_result;
        request.status = result.status;
      } else {
        // Partial approval — navigate back to list
        push("/approvals");
      }
    } catch (e) {
      error = e.error || "Không thể approve.";
    } finally {
      actionLoading = false;
    }
  }

  /**
   * Rejects the pending request with a mandatory reason (minimum 10 characters).
   * On success, navigates back to the approvals list.
   */
  async function handleReject() {
    if (rejectionReason.length < 10) {
      error = "Lý do reject phải ít nhất 10 ký tự.";
      return;
    }
    actionLoading = true;
    error = null;
    try {
      await api.approvals.reject(id, rejectionReason);
      push("/approvals");
    } catch (e) {
      error = e.error || "Không thể reject.";
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
  {:else if request}
    <div class="flex items-center gap-3 mb-6">
      <h2 class="text-2xl font-bold">Review Request</h2>
      <StatusBadge status={request.status} />
    </div>

    {#if error}
      <div class="alert alert-error mb-4"><span>{error}</span></div>
    {/if}

    {#if request.approval_levels_required > 1}
      <div class="alert alert-info mb-4">
        <span>Tiến trình phê duyệt: {request.approval_count || 0}/{request.approval_levels_required} approvals</span>
      </div>
    {/if}

    <div class="card bg-base-100 shadow mb-6">
      <div class="card-body">
        <h3 class="font-semibold mb-2">Mô tả</h3>
        <p>{request.description}</p>
      </div>
    </div>

    <div class="card bg-base-100 shadow mb-6">
      <div class="card-body">
        <h3 class="font-semibold mb-2">SQL Content</h3>
        <SqlEditor value={request.sql_content} readonly={true} minHeight={200} />
      </div>
    </div>

    <!-- DDL Warning Banner (BR-005) -->
    {#if request.sql_analysis && request.sql_analysis.contains_ddl}
      <div class="alert {request.sql_analysis.has_danger ? 'alert-error' : 'alert-warning'} mb-6">
        <div>
          <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
          </svg>
          <div>
            <h3 class="font-bold">
              {request.sql_analysis.has_danger ? '⚠️ CẢNH BÁO: DDL NGUY HIỂM' : '⚠️ Chú ý: DDL Statement'}
            </h3>
            <p class="text-sm">SQL này chứa câu lệnh DDL — kiểm tra kỹ trước khi approve.</p>
          </div>
        </div>
      </div>

      <div class="card bg-base-100 shadow mb-6">
        <div class="card-body">
          <h3 class="font-semibold mb-3">Chi tiết DDL Analysis</h3>
          <div class="overflow-x-auto">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>Keyword</th>
                  <th>Mức độ</th>
                  <th>Mô tả</th>
                </tr>
              </thead>
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

    {#if request.status === "PENDING"}
      <div class="flex flex-col gap-4">
        <!-- Step 1: Validate SQL -->
        <div class="card bg-base-200 shadow">
          <div class="card-body">
            <h3 class="font-semibold flex items-center gap-2">
              <span class="badge badge-outline badge-sm">Bước 1</span>
              Validate SQL
            </h3>
            <p class="text-sm text-base-content/70">Kiểm tra cú pháp, cấu trúc bảng, và hiệu năng trước khi thực thi.</p>
            <button
              class="btn btn-outline btn-info mt-2"
              on:click={handleValidate}
              disabled={validating || actionLoading}
            >
              {#if validating}
                <span class="loading loading-spinner loading-sm"></span>
                Đang validate...
              {:else}
                🔍 Validate SQL
              {/if}
            </button>
          </div>
        </div>

        <!-- Validation Result -->
        {#if validationResult}
          <div class="card bg-base-100 shadow">
            <div class="card-body">
              {#if validationResult.valid}
                <div class="alert alert-success">
                  <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span>✓ SQL hợp lệ — sẵn sàng để approve & execute</span>
                </div>
              {:else}
                <div class="alert alert-error">
                  <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
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

        <!-- Step 2: Approve & Execute (chỉ enabled khi validated) -->
        <div class="card bg-base-200 shadow">
          <div class="card-body">
            <h3 class="font-semibold flex items-center gap-2">
              <span class="badge badge-outline badge-sm">Bước 2</span>
              Approve & Execute
            </h3>
            {#if !validated}
              <p class="text-sm text-warning">⚠️ Phải validate SQL thành công trước khi approve.</p>
            {/if}
            <button
              class="btn btn-success mt-2"
              on:click={handleApprove}
              disabled={!validated || actionLoading}
            >
              {#if actionLoading}
                <span class="loading loading-spinner loading-sm"></span>
                Đang thực thi SQL...
              {:else if request.approval_levels_required > 1}
                {#if (request.approval_count || 0) + 1 >= request.approval_levels_required}
                  ✓ Approve & Execute ({request.approval_levels_required}/{request.approval_levels_required})
                {:else}
                  ✓ Approve ({(request.approval_count || 0) + 1}/{request.approval_levels_required})
                {/if}
              {:else}
                ✓ Approve & Execute
              {/if}
            </button>
          </div>
        </div>

        <div class="divider">HOẶC</div>

        <div class="form-control">
          <label class="label" for="reason"><span class="label-text">Lý do reject (ít nhất 10 ký tự)</span></label>
          <textarea
            id="reason"
            class="textarea textarea-bordered"
            placeholder="Giải thích lý do từ chối..."
            bind:value={rejectionReason}
          ></textarea>
        </div>
        <button
          class="btn btn-error"
          on:click={handleReject}
          disabled={actionLoading || rejectionReason.length < 10}
        >
          ✗ Reject
        </button>
      </div>
    {/if}

    <!-- Execution Result Display -->
    {#if executionResult}
      <div class="card bg-base-100 shadow mt-6">
        <div class="card-body">
          <h3 class="font-semibold mb-3 flex items-center gap-2">
            {#if executionResult.success}
              <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span class="text-success">Thực thi thành công</span>
            {:else}
              <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-error" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span class="text-error">Thực thi thất bại</span>
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
              <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span>{executionResult.error}</span>
            </div>
          {/if}

          <div class="mt-4">
            <button class="btn btn-outline btn-sm" on:click={() => push("/approvals")}>
              ← Quay lại danh sách
            </button>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>
