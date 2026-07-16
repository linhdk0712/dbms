<script>
  import { notifications } from "../stores/notifications.js";

  const typeLabels = {
    NEW_REQUEST: "Có request mới chờ duyệt",
    REQUEST_APPROVED: "Request đã được approve",
    REQUEST_REJECTED: "Request bị reject",
    REQUEST_EXECUTED: "SQL đã thực thi thành công",
    REQUEST_FAILED: "SQL thực thi thất bại",
    REQUEST_CANCELLED: "Request đã bị hủy",
    PARTIAL_APPROVAL: "Request nhận được một approval",
  };
</script>

<div class="toast toast-top toast-end z-50">
  {#each $notifications.slice(0, 3) as notif (notif.request_id + notif.type)}
    <div class="alert alert-info shadow-lg">
      <span class="text-sm">{typeLabels[notif.type] || notif.type}</span>
    </div>
  {/each}
</div>
