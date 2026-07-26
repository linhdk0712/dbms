import { writable } from "svelte/store";

let nextId = 0;

/**
 * Toast notification store.
 * Supports multiple concurrent toasts with auto-dismiss.
 */
function createToastStore() {
  const { subscribe, update } = writable([]);

  /**
   * Show a toast notification.
   * @param {string} message - Display message
   * @param {"info"|"success"|"warning"|"error"} [type="info"] - Alert variant
   * @param {number} [duration=5000] - Auto-dismiss in ms (0 = no auto-dismiss)
   */
  function show(message, type = "info", duration = 5000) {
    const id = ++nextId;
    update((toasts) => [...toasts, { id, message, type, duration }]);
    return id;
  }

  /** Show info toast */
  function info(message, duration = 5000) {
    return show(message, "info", duration);
  }

  /** Show success toast */
  function success(message, duration = 5000) {
    return show(message, "success", duration);
  }

  /** Show warning toast */
  function warning(message, duration = 6000) {
    return show(message, "warning", duration);
  }

  /** Show error toast */
  function error(message, duration = 8000) {
    return show(message, "error", duration);
  }

  /** Dismiss a specific toast by id */
  function dismiss(id) {
    update((toasts) => toasts.filter((t) => t.id !== id));
  }

  /** Clear all toasts */
  function clear() {
    update(() => []);
  }

  return {
    subscribe,
    show,
    info,
    success,
    warning,
    error,
    dismiss,
    clear,
  };
}

export const toastStore = createToastStore();
