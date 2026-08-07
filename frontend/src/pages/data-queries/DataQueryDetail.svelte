<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import { toastStore } from "../../stores/toast.js";
  import StatusBadge from "../../components/StatusBadge.svelte";
  import SqlEditor from "../../components/SqlEditor.svelte";
  import QueryResultTable from "../../components/QueryResultTable.svelte";

  export let params = {};

  let query = null;
  let executions = [];
  let loading = true;
  let executing = false;
  let executionResult = null;
  let submitting = false;
  let approvalTracking = [];

  $: queryId = params.id;

  onMount(() => loadQuery());

  async function loadQuery() {
    loading = true;
    try {
      query = await api.dataQueries.get(queryId);
      approvalTracking = query?.approval_tracking || [];
      if (query?.status === "APPROVED") {
        await loadExecutions();
      }
    } catch (e) {
      toastStore.error("Không thể tải thông tin query.");
      push("/data-queries");
    } finally {
      loading = false;
    }
  }

  async function loadExecutions() {
    try {
      executions = await api.dataQueries.executions(queryId);
      if (!Array.isArray(executions)) executions = [];
    } catch (e) {
      executions = [];
    }
  }

  async function handleSubmit() {
    submitting = true;
    try {
      await api.dataQueries.submit(queryId);
      toastStore.success("Đã gửi phê duyệt.");
      await loadQuery();
    } catch (e) {
      toastStore.error(e.error || "Không thể gửi phê duyệt.");
    } finally {
      submitting = false;
    }
  }

  async function handleExecute() {
    executing = true;
    executionResult = null;
    try {
      executionResult = await api.dataQueries.execute(queryId);
      toastStore.success(`Query thực thi thành công — ${executionResult.rows_returned} rows.`);
      await loadExecutions();
    } catch (e) {
      toastStore.error(e.error || "Thực thi query thất bại.");
    } finally {
      executing = false;
    }
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

  $: canExecute = query?.status === "APPROVED" && (!query?.expires_at || new Date(query.expires_at) > new Date());
  $: isDraft = query?.status === "DRAFT";
</script>

<div class="p-6 max-w-5xl mx-auto">
  {#if loading}
    <div class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if query}
    <!-- Header -->
    <div class="mb-6">
      <div class="flex items-center gap-3">
        <a href="#/data-queries" class="btn btn-ghost btn-sm btn-circle">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </a>
        <div class="flex-1">
          <div class="flex items-center gap-3">
            <h2 class="text-2xl font-bold">{query.title}</h2>
            <StatusBadge status={query.status} />
          </div>
          <p class="text-sm text-base-content/60 mt-0.5">
            {query.query_mode} mode • Ticket: {query.ticket_id} • Tạo: {formatDate(query.created_at)}
          </p>
        </div>
        <div class="flex gap-2">
          {#if isDraft}
            <button class="btn btn-primary btn-sm" on:click={handleSubmit} disabled={submitting}>
              {#if submitting}<span class="loading loading-spinner loading-xs"></span>{/if}
              Gửi phê duyệt
            </button>
          {/if}
          {#if canExecute}
            <button class="btn btn-success btn-sm" on:click={handleExecute} disabled={executing}>
              {#if executing}<span class="loading loading-spinner loading-xs"></span>{/if}
              Thực thi Query
            </button>
          {/if}
        </div>
      </div>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Main content -->
      <div class="lg:col-span-2 space-y-4">
        <!-- SQL Content -->
        <div class="card bg-base-100 border border-base-300">
          <div class="card-body p-4">
            <h3 class="font-semibold text-sm uppercase text-base-content/70 mb-2">SQL Query</h3>
            <SqlEditor value={query.sql_query} readonly={true} language="sql" minHeight={150} />
          </div>
        </div>

        <!-- Execution Result -->
        {#if executionResult}
          <div class="card bg-base-100 border border-base-300">
            <div class="card-body p-4">
              <div class="flex items-center justify-between mb-2">
                <h3 class="font-semibold text-sm uppercase text-base-content/70">Kết quả</h3>
                <span class="text-sm text-base-content/60">
                  {executionResult.rows_returned} rows • {executionResult.execution_time_ms}ms
                  {#if executionResult.truncated}
                    <span class="badge badge-warning badge-xs ml-1">truncated</span>
                  {/if}
                </span>
              </div>
              <QueryResultTable columns={executionResult.columns} rows={executionResult.rows} />
            </div>
          </div>
        {/if}

        <!-- Execution History -->
        {#if executions.length > 0}
          <div class="card bg-base-100 border border-base-300">
            <div class="card-body p-4">
              <h3 class="font-semibold text-sm uppercase text-base-content/70 mb-2">Lịch sử thực thi</h3>
              <div class="overflow-x-auto">
                <table class="table table-xs">
                  <thead>
                    <tr>
                      <th>Người thực thi</th>
                      <th>Rows</th>
                      <th>Thời gian</th>
                      <th>Trạng thái</th>
                      <th>Ngày</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each executions as exec}
                      <tr>
                        <td>{exec.executed_by_name || exec.executed_by}</td>
                        <td>{exec.rows_returned}</td>
                        <td>{exec.execution_time_ms}ms</td>
                        <td>
                          <span class="badge badge-xs" class:badge-success={exec.status === "SUCCESS"} class:badge-error={exec.status === "FAILED"} class:badge-warning={exec.status === "TIMEOUT"}>
                            {exec.status}
                          </span>
                        </td>
                        <td class="text-xs">{formatDate(exec.executed_at)}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Sidebar info -->
      <div class="space-y-4">
        <!-- Metadata -->
        <div class="card bg-base-100 border border-base-300 p-4 space-y-3">
          <h3 class="font-semibold text-sm uppercase text-base-content/70">Chi tiết</h3>
          <div class="text-sm space-y-2">
            <div class="flex justify-between">
              <span class="text-base-content/60">Database</span>
              <span class="font-medium">{query.database_name || query.database_id}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/60">Schema</span>
              <span class="font-medium">{query.schema_name || "—"}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/60">Ticket</span>
              <span class="font-mono">{query.ticket_id}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/60">Hết hạn</span>
              <span>{formatDate(query.expires_at)}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-base-content/60">Người tạo</span>
              <span>{query.creator_name || query.created_by}</span>
            </div>
          </div>
        </div>

        <!-- Assigned Users -->
        {#if query.assigned_users && query.assigned_users.length > 0}
          <div class="card bg-base-100 border border-base-300 p-4">
            <h3 class="font-semibold text-sm uppercase text-base-content/70 mb-2">Users được assign</h3>
            <div class="flex flex-wrap gap-1">
              {#each query.assigned_users as user}
                <span class="badge badge-outline badge-sm">{user.display_name || user.email}</span>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Approval Tracking -->
        {#if approvalTracking.length > 0}
          <div class="card bg-base-100 border border-base-300 p-4">
            <h3 class="font-semibold text-sm uppercase text-base-content/70 mb-2">Phê duyệt</h3>
            <ul class="steps steps-vertical text-sm">
              {#each approvalTracking as level}
                <li
                  class="step"
                  class:step-success={level.status === "APPROVED"}
                  class:step-error={level.status === "REJECTED"}
                  class:step-warning={level.status === "PENDING"}
                >
                  <div class="text-left">
                    <span class="font-medium">Level {level.level_order}: {level.approver_unit}</span>
                    {#if level.status !== "PENDING"}
                      <span class="text-xs text-base-content/60 block">
                        {level.approved_by_name || ""} • {formatDate(level.acted_at)}
                      </span>
                    {/if}
                    {#if level.comment}
                      <span class="text-xs italic block">"{level.comment}"</span>
                    {/if}
                  </div>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
