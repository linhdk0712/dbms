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

  async function handleApprove() {
    const isMultiLevel = request.approval_levels_required > 1;
    const isFinalApproval = isMultiLevel && (request.approval_count || 0) + 1 >= request.approval_levels_required;
    const confirmMsg = isMultiLevel && !isFinalApproval
      ? `Xác nhận APPROVE request này? (${(request.approval_count || 0) + 1}/${request.approval_levels_required} approvals)`
      : "Xác nhận APPROVE request này? SQL sẽ được thực thi ngay.";
    if (!confirm(confirmMsg)) return;
    actionLoading = true;
    error = null;
    try {
      await api.approvals.approve(id);
      push("/approvals");
    } catch (e) {
      error = e.error || "Không thể approve.";
    } finally {
      actionLoading = false;
    }
  }

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
        <button
          class="btn btn-success"
          on:click={handleApprove}
          disabled={actionLoading}
        >
          {#if actionLoading}
            Đang xử lý...
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
  {/if}
</div>
