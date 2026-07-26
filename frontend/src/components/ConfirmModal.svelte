<script>
  import { createEventDispatcher } from "svelte";

  /** @type {boolean} */
  export let open = false;
  /** @type {string} */
  export let title = "Xác nhận";
  /** @type {string} */
  export let message = "";
  /** @type {string} */
  export let confirmText = "Xác nhận";
  /** @type {string} */
  export let cancelText = "Hủy";
  /** @type {"primary" | "error" | "warning" | "success" | "info"} */
  export let variant = "primary";
  /** @type {boolean} */
  export let loading = false;

  const dispatch = createEventDispatcher();

  $: btnClass = {
    primary: "btn-primary",
    error: "btn-error",
    warning: "btn-warning",
    success: "btn-success",
    info: "btn-info",
  }[variant] || "btn-primary";

  $: if (open) {
    // Open modal via native dialog API
    setTimeout(() => {
      const el = document.getElementById("confirm-modal-dialog");
      if (el && !el.open) el.showModal();
    }, 0);
  }

  /** Dispatches the "confirm" event when the user clicks the confirm button. */
  function handleConfirm() {
    dispatch("confirm");
  }

  /** Closes the modal and dispatches the "cancel" event. */
  function handleCancel() {
    open = false;
    dispatch("cancel");
  }

  /**
   * Handles keydown events on the dialog.
   * Closes the modal when the Escape key is pressed.
   * @param {KeyboardEvent} e - The keyboard event.
   */
  function handleKeydown(e) {
    if (e.key === "Escape") {
      handleCancel();
    }
  }
</script>

{#if open}
  <dialog
    id="confirm-modal-dialog"
    class="modal modal-open"
    aria-labelledby="confirm-modal-title"
    aria-describedby="confirm-modal-desc"
  >
    <div class="modal-box">
      <h3 id="confirm-modal-title" class="text-lg font-bold">{title}</h3>
      <p id="confirm-modal-desc" class="py-4">{message}</p>
      <div class="modal-action">
        <button
          class="btn btn-ghost"
          on:click={handleCancel}
          disabled={loading}
        >
          {cancelText}
        </button>
        <button
          class="btn {btnClass}"
          on:click={handleConfirm}
          disabled={loading}
        >
          {#if loading}
            <span class="loading loading-spinner loading-sm"></span>
          {/if}
          {confirmText}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button on:click={handleCancel}>close</button>
    </form>
  </dialog>
{/if}
