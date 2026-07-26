<script>
  /**
   * @component StatusBadge
   * @description Displays a color-coded badge with icon indicating the current status of a SQL request.
   * Supports all RequestStatus values (DRAFT, PENDING, APPROVED, EXECUTED, FAILED, REJECTED, CANCELLED).
   *
   * @param {string} status - The request status value (e.g. "DRAFT", "PENDING", "EXECUTED")
   * @param {"sm" | "md" | "lg"} [size="md"] - Badge size variant
   */

  /** @type {string} */
  export let status;
  /** @type {"sm" | "md" | "lg"} */
  export let size = "md";

  const statusColors = {
    DRAFT: "badge-neutral",
    PENDING: "badge-warning",
    APPROVED: "badge-info",
    EXECUTED: "badge-success",
    FAILED: "badge-error",
    REJECTED: "badge-error",
    CANCELLED: "badge-ghost",
  };

  const statusLabels = {
    DRAFT: "Bản nháp",
    PENDING: "Chờ duyệt",
    APPROVED: "Đã approve",
    EXECUTED: "Đã thực thi",
    FAILED: "Thất bại",
    REJECTED: "Bị từ chối",
    CANCELLED: "Đã hủy",
  };

  const statusIcons = {
    DRAFT: "M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z",
    PENDING: "M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z",
    APPROVED: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z",
    EXECUTED: "M5 13l4 4L19 7",
    FAILED: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z",
    REJECTED: "M6 18L18 6M6 6l12 12",
    CANCELLED: "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z",
  };

  const iconSizes = {
    sm: "h-3 w-3",
    md: "h-3.5 w-3.5",
    lg: "h-4 w-4",
  };

  $: colorClass = statusColors[status] || "badge-ghost";
  $: label = statusLabels[status] || status;
  $: icon = statusIcons[status] || "";
  $: sizeClass = `badge-${size}`;
  $: iconSize = iconSizes[size] || "h-3.5 w-3.5";
</script>

<span
  class="badge {colorClass} {sizeClass} gap-1 font-medium"
  aria-label="Trạng thái: {label}"
  role="status"
>
  {#if icon}
    <svg xmlns="http://www.w3.org/2000/svg" class="{iconSize}" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d={icon} />
    </svg>
  {/if}
  {status}
</span>
