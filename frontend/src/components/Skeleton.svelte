<script>
  /**
   * Skeleton loading placeholder.
   * @type {"stat" | "table" | "card" | "text" | "custom"}
   */
  export let type = "text";
  /** Number of rows for table skeleton */
  export let rows = 5;
  /** Number of columns for table skeleton */
  export let cols = 4;
</script>

{#if type === "stat"}
  <!-- Stat card skeleton -->
  <div class="stat bg-base-100 shadow rounded-box animate-pulse">
    <div class="stat-title">
      <div class="h-3 w-16 bg-base-300 rounded"></div>
    </div>
    <div class="stat-value">
      <div class="h-7 w-12 bg-base-300 rounded mt-1"></div>
    </div>
    <div class="stat-desc">
      <div class="h-2 w-20 bg-base-300 rounded mt-1"></div>
    </div>
  </div>

{:else if type === "table"}
  <!-- Table skeleton -->
  <div class="overflow-x-auto animate-pulse">
    <table class="table w-full">
      <thead>
        <tr>
          {#each Array(cols) as _, i}
            <th><div class="h-3 w-20 bg-base-300 rounded"></div></th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each Array(rows) as _, rowIdx}
          <tr>
            {#each Array(cols) as _, colIdx}
              <td>
                <div
                  class="h-3 bg-base-300 rounded"
                  style="width: {60 + Math.random() * 30}%"
                ></div>
              </td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

{:else if type === "card"}
  <!-- Card skeleton -->
  <div class="card bg-base-100 shadow animate-pulse">
    <div class="card-body">
      <div class="h-5 w-2/3 bg-base-300 rounded"></div>
      <div class="h-3 w-full bg-base-300 rounded mt-2"></div>
      <div class="h-3 w-4/5 bg-base-300 rounded mt-1"></div>
      <div class="flex gap-2 mt-3">
        <div class="h-5 w-16 bg-base-300 rounded"></div>
        <div class="h-5 w-20 bg-base-300 rounded"></div>
      </div>
    </div>
  </div>

{:else}
  <!-- Text line skeleton -->
  <div class="animate-pulse space-y-2">
    <div class="h-3 w-full bg-base-300 rounded"></div>
    <div class="h-3 w-4/5 bg-base-300 rounded"></div>
    <div class="h-3 w-3/5 bg-base-300 rounded"></div>
  </div>
{/if}
