<script>
  import { onDestroy } from "svelte";
  import { toastStore } from "../stores/toast.js";

  // Auto-dismiss timers
  let timers = {};

  const alertVariants = {
    info: "alert-info",
    success: "alert-success",
    warning: "alert-warning",
    error: "alert-error",
  };

  const icons = {
    info: "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
    success: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z",
    warning: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z",
    error: "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z",
  };

  $: toasts = $toastStore;

  // Setup auto-dismiss for new toasts
  $: {
    toasts.forEach((toast) => {
      if (!timers[toast.id] && toast.duration > 0) {
        timers[toast.id] = setTimeout(() => {
          toastStore.dismiss(toast.id);
          delete timers[toast.id];
        }, toast.duration);
      }
    });
  }

  function dismiss(id) {
    if (timers[id]) {
      clearTimeout(timers[id]);
      delete timers[id];
    }
    toastStore.dismiss(id);
  }

  onDestroy(() => {
    Object.values(timers).forEach(clearTimeout);
    timers = {};
  });
</script>

<div class="toast toast-top toast-end z-50 gap-2">
  {#each toasts.slice(0, 5) as toast (toast.id)}
    <div
      class="alert {alertVariants[toast.type] || 'alert-info'} shadow-lg max-w-sm animate-slide-in relative overflow-hidden"
      role="alert"
      aria-live="polite"
    >
      <!-- Icon -->
      <svg
        xmlns="http://www.w3.org/2000/svg"
        class="stroke-current shrink-0 h-5 w-5"
        fill="none"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d={icons[toast.type] || icons.info}
        />
      </svg>

      <!-- Message -->
      <span class="text-sm flex-1">{toast.message}</span>

      <!-- Dismiss button -->
      <button
        class="btn btn-ghost btn-xs btn-circle"
        on:click={() => dismiss(toast.id)}
        aria-label="Đóng thông báo"
      >
        ✕
      </button>

      <!-- Progress bar for auto-dismiss -->
      {#if toast.duration > 0}
        <div
          class="absolute bottom-0 left-0 h-0.5 bg-current opacity-30 animate-progress"
          style="animation-duration: {toast.duration}ms"
        ></div>
      {/if}
    </div>
  {/each}
</div>

<style>
  @keyframes slide-in {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  @keyframes progress {
    from {
      width: 100%;
    }
    to {
      width: 0%;
    }
  }

  .animate-slide-in {
    animation: slide-in 0.3s ease-out;
  }

  .animate-progress {
    animation: progress linear forwards;
  }
</style>
