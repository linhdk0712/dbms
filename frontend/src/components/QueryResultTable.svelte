<script>
  /**
   * @component QueryResultTable
   * @description Hiển thị kết quả truy vấn SQL dưới dạng bảng với pagination và export CSV.
   *
   * @param {string[]} columns - Danh sách tên cột
   * @param {any[][]} rows - Dữ liệu (mảng 2D)
   * @param {number} [pageSize=50] - Số dòng mỗi trang
   */

  /** @type {string[]} */
  export let columns = [];
  /** @type {any[][]} */
  export let rows = [];
  /** @type {number} */
  export let pageSize = 50;

  let currentPage = 1;

  $: totalPages = Math.ceil(rows.length / pageSize);
  $: displayedRows = rows.slice((currentPage - 1) * pageSize, currentPage * pageSize);

  function goToPage(p) {
    if (p >= 1 && p <= totalPages) {
      currentPage = p;
    }
  }

  function exportCsv() {
    const header = columns.join(",");
    const body = rows
      .map((row) =>
        row
          .map((cell) => {
            const str = cell == null ? "" : String(cell);
            // Escape CSV values with commas or quotes
            if (str.includes(",") || str.includes('"') || str.includes("\n")) {
              return `"${str.replace(/"/g, '""')}"`;
            }
            return str;
          })
          .join(",")
      )
      .join("\n");

    const csv = `${header}\n${body}`;
    const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `query-result-${new Date().toISOString().slice(0, 10)}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  }
</script>

<div class="space-y-2">
  <!-- Controls -->
  <div class="flex items-center justify-between">
    <span class="text-xs text-base-content/60">
      {rows.length} rows
      {#if totalPages > 1}
        • Trang {currentPage}/{totalPages}
      {/if}
    </span>
    <button class="btn btn-ghost btn-xs gap-1" on:click={exportCsv} disabled={rows.length === 0}>
      <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
      CSV
    </button>
  </div>

  <!-- Table -->
  {#if columns.length > 0}
    <div class="overflow-x-auto max-h-96 border border-base-300 rounded-lg">
      <table class="table table-xs table-pin-rows">
        <thead>
          <tr>
            <th class="w-10">#</th>
            {#each columns as col}
              <th>{col}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each displayedRows as row, i}
            <tr class="hover">
              <td class="text-base-content/40 font-mono text-xs">
                {(currentPage - 1) * pageSize + i + 1}
              </td>
              {#each row as cell}
                <td class="max-w-xs truncate" title={cell != null ? String(cell) : ""}>
                  {#if cell == null}
                    <span class="text-base-content/30 italic">null</span>
                  {:else}
                    {String(cell)}
                  {/if}
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Pagination -->
    {#if totalPages > 1}
      <div class="flex justify-center">
        <div class="join">
          <button class="join-item btn btn-xs" disabled={currentPage === 1} on:click={() => goToPage(currentPage - 1)}>«</button>
          {#each Array(Math.min(totalPages, 7)) as _, i}
            {@const pageNum = totalPages <= 7 ? i + 1 : (currentPage <= 4 ? i + 1 : currentPage - 3 + i)}
            {#if pageNum >= 1 && pageNum <= totalPages}
              <button
                class="join-item btn btn-xs"
                class:btn-active={currentPage === pageNum}
                on:click={() => goToPage(pageNum)}
              >
                {pageNum}
              </button>
            {/if}
          {/each}
          <button class="join-item btn btn-xs" disabled={currentPage === totalPages} on:click={() => goToPage(currentPage + 1)}>»</button>
        </div>
      </div>
    {/if}
  {:else}
    <div class="text-center py-4 text-base-content/50 text-sm">Không có dữ liệu.</div>
  {/if}
</div>
