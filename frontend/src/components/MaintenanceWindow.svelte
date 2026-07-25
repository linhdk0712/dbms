<script>
  /** ISO datetime string — window start */
  export let startTime = null;
  /** ISO datetime string — window end */
  export let endTime = null;

  $: start = startTime ? new Date(startTime) : null;
  $: end = endTime ? new Date(endTime) : null;
  $: now = new Date();

  $: status = getStatus(start, end, now);

  /**
   * Determines the current status relative to the maintenance window.
   * @param {Date|null} s - Window start date
   * @param {Date|null} e - Window end date
   * @param {Date} n - Current date/time
   * @returns {"before"|"within"|"after"|null} Status relative to the window
   */
  function getStatus(s, e, n) {
    if (!s || !e) return null;
    if (n < s) return "before";
    if (n > e) return "after";
    return "within";
  }
</script>

{#if start && end}
  <div class="flex items-center gap-2 text-sm">
    <span class="opacity-60">Maintenance Window:</span>
    <span class="font-mono text-xs">
      {start.toLocaleString("vi")} → {end.toLocaleString("vi")}
    </span>
    {#if status === "within"}
      <span class="badge badge-success badge-xs">Trong window</span>
    {:else if status === "before"}
      <span class="badge badge-warning badge-xs">Chưa đến</span>
    {:else}
      <span class="badge badge-error badge-xs">Đã hết</span>
    {/if}
  </div>
{/if}
