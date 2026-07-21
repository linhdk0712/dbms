<script>
  /**
   * AiAnalysisPanel — Hiển thị kết quả phân tích AI cho SQL.
   *
   * Props:
   * - requestId: UUID của request cần analyze
   * - autoLoad: tự động gọi analyze khi component mount (default: false)
   *
   * Hiển thị:
   * - Risk score (progress bar)
   * - Performance issues
   * - Security issues
   * - Best practices
   * - Cost estimation
   * - Suggested SQL (nếu có)
   */

  import { api } from "../api/index.js";

  export let requestId = null;
  export let autoLoad = false;

  let analysis = null;
  let loading = false;
  let error = null;

  // Severity badge colors
  const severityColors = {
    critical: "badge-error",
    high: "badge-error",
    medium: "badge-warning",
    low: "badge-info",
  };

  // Risk score color
  function getRiskColor(score) {
    if (score >= 8) return "text-error";
    if (score >= 5) return "text-warning";
    return "text-success";
  }

  // Risk score progress color
  function getRiskProgressClass(score) {
    if (score >= 8) return "progress-error";
    if (score >= 5) return "progress-warning";
    return "progress-success";
  }

  async function runAnalysis() {
    if (!requestId) return;
    loading = true;
    error = null;
    analysis = null;

    try {
      analysis = await api.requests.analyze(requestId);
    } catch (e) {
      error = e.error || "Không thể phân tích SQL. Vui lòng thử lại sau.";
    } finally {
      loading = false;
    }
  }

  function copyToClipboard(text) {
    navigator.clipboard.writeText(text);
  }

  // Auto-load if configured
  $: if (autoLoad && requestId && !analysis && !loading) {
    runAnalysis();
  }
</script>

<div class="card bg-base-200 shadow-sm">
  <div class="card-body p-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h3 class="card-title text-sm gap-2">
        <span>🤖</span> AI SQL Analysis
      </h3>
      <button
        class="btn btn-ghost btn-xs"
        on:click={runAnalysis}
        disabled={loading}
        title="Phân tích lại"
      >
        <svg
          class="w-4 h-4"
          class:animate-spin={loading}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
          />
        </svg>
      </button>
    </div>

    <!-- Loading state -->
    {#if loading}
      <div class="flex items-center gap-2 py-4">
        <span class="loading loading-spinner loading-sm"></span>
        <span class="text-sm opacity-70">Đang phân tích SQL...</span>
      </div>

    <!-- Error state -->
    {:else if error}
      <div class="alert alert-warning text-sm py-2 mt-2">
        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L4.082 16.5c-.77.833.192 2.5 1.732 2.5z" />
        </svg>
        <span>{error}</span>
      </div>

    <!-- No analysis yet -->
    {:else if !analysis}
      <div class="text-sm opacity-60 py-2">
        Nhấn nút để phân tích SQL bằng AI.
      </div>

    <!-- Analysis results -->
    {:else}
      <div class="space-y-3 mt-2">
        <!-- Risk Score -->
        <div class="flex items-center gap-3">
          <span class="text-xs font-medium opacity-70">Risk Score:</span>
          <progress
            class="progress w-24 {getRiskProgressClass(analysis.risk_score)}"
            value={analysis.risk_score}
            max="10"
          ></progress>
          <span class="text-sm font-bold {getRiskColor(analysis.risk_score)}">
            {analysis.risk_score}/10
          </span>
        </div>

        <!-- Summary -->
        {#if analysis.summary}
          <p class="text-sm opacity-80">{analysis.summary}</p>
        {/if}

        <!-- Performance -->
        {#if analysis.performance.length > 0}
          <div class="collapse collapse-arrow bg-base-100">
            <input type="checkbox" checked />
            <div class="collapse-title text-sm font-medium py-2 min-h-0">
              ⚡ Performance ({analysis.performance.length} vấn đề)
            </div>
            <div class="collapse-content px-2">
              <ul class="space-y-2">
                {#each analysis.performance as item}
                  <li class="text-xs">
                    <span class="badge badge-xs {severityColors[item.severity]}">{item.severity.toUpperCase()}</span>
                    <span class="font-medium ml-1">{item.message}</span>
                    {#if item.suggestion}
                      <p class="opacity-70 mt-0.5 ml-4">→ {item.suggestion}</p>
                    {/if}
                  </li>
                {/each}
              </ul>
            </div>
          </div>
        {/if}

        <!-- Security -->
        {#if analysis.security.length > 0}
          <div class="collapse collapse-arrow bg-base-100">
            <input type="checkbox" checked />
            <div class="collapse-title text-sm font-medium py-2 min-h-0">
              🔒 Security ({analysis.security.length} vấn đề)
            </div>
            <div class="collapse-content px-2">
              <ul class="space-y-2">
                {#each analysis.security as item}
                  <li class="text-xs">
                    <span class="badge badge-xs {severityColors[item.severity]}">{item.severity.toUpperCase()}</span>
                    <span class="font-medium ml-1">{item.message}</span>
                    {#if item.remediation}
                      <p class="opacity-70 mt-0.5 ml-4">→ {item.remediation}</p>
                    {/if}
                  </li>
                {/each}
              </ul>
            </div>
          </div>
        {/if}

        <!-- Best Practices -->
        {#if analysis.best_practices.length > 0}
          <div class="collapse collapse-arrow bg-base-100">
            <input type="checkbox" />
            <div class="collapse-title text-sm font-medium py-2 min-h-0">
              💡 Best Practices ({analysis.best_practices.length} gợi ý)
            </div>
            <div class="collapse-content px-2">
              <ul class="space-y-2">
                {#each analysis.best_practices as item}
                  <li class="text-xs">
                    <span class="badge badge-xs {severityColors[item.severity]}">{item.severity.toUpperCase()}</span>
                    <span class="font-medium ml-1">{item.message}</span>
                    {#if item.suggestion}
                      <p class="opacity-70 mt-0.5 ml-4">→ {item.suggestion}</p>
                    {/if}
                  </li>
                {/each}
              </ul>
            </div>
          </div>
        {/if}

        <!-- Cost Estimation -->
        {#if analysis.cost_estimation}
          <div class="collapse collapse-arrow bg-base-100">
            <input type="checkbox" />
            <div class="collapse-title text-sm font-medium py-2 min-h-0">
              💰 Cost Estimation
            </div>
            <div class="collapse-content px-2">
              <div class="text-xs space-y-1">
                {#if analysis.cost_estimation.estimated_rows}
                  <p><span class="opacity-70">Rows affected:</span> ~{analysis.cost_estimation.estimated_rows}</p>
                {/if}
                <p><span class="opacity-70">Lock impact:</span> {analysis.cost_estimation.lock_impact}</p>
                {#if analysis.cost_estimation.estimated_duration}
                  <p><span class="opacity-70">Duration:</span> {analysis.cost_estimation.estimated_duration}</p>
                {/if}
                {#if analysis.cost_estimation.replication_note}
                  <p><span class="opacity-70">Replication:</span> {analysis.cost_estimation.replication_note}</p>
                {/if}
              </div>
            </div>
          </div>
        {/if}

        <!-- Suggested SQL -->
        {#if analysis.suggested_sql}
          <div class="collapse collapse-arrow bg-base-100">
            <input type="checkbox" />
            <div class="collapse-title text-sm font-medium py-2 min-h-0">
              📝 SQL Đề xuất
            </div>
            <div class="collapse-content px-2">
              <div class="relative">
                <pre class="text-xs bg-base-300 p-2 rounded overflow-x-auto"><code>{analysis.suggested_sql}</code></pre>
                <button
                  class="btn btn-ghost btn-xs absolute top-1 right-1"
                  on:click={() => copyToClipboard(analysis.suggested_sql)}
                  title="Copy"
                >
                  📋
                </button>
              </div>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
