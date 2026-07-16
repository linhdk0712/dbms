<script>
  /** @type {string} */
  export let status;

  const statusColors = {
    DRAFT: "badge-neutral",
    PENDING: "badge-warning",
    APPROVED: "badge-info",
    EXECUTED: "badge-success",
    FAILED: "badge-error",
    REJECTED: "badge-error",
    CANCELLED: "badge-ghost",
  };

  $: colorClass = statusColors[status] || "badge-ghost";
</script>

<span class="badge {colorClass} badge-sm">{status}</span>
