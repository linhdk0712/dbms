<script>
  import { createEventDispatcher } from "svelte";
  import api from "../api/index.js";

  /** Bind:value — CR URL string */
  export let value = "";

  const dispatch = createEventDispatcher();

  let loading = false;
  let crPreview = null; // { cr_number, title, scheduled_start_time, scheduled_end_time }
  let error = null;

  async function handleBlur() {
    if (!value.trim()) {
      crPreview = null;
      error = null;
      return;
    }

    loading = true;
    error = null;
    crPreview = null;

    try {
      const cr = await api.changeRequests.create(value.trim());
      crPreview = cr;
      dispatch("resolved", cr);
    } catch (e) {
      error = e.error || "URL không hợp lệ hoặc không thể kết nối ManageEngine.";
      crPreview = null;
    } finally {
      loading = false;
    }
  }

  function handleClear() {
    value = "";
    crPreview = null;
    error = null;
    dispatch("resolved", null);
  }
</script>

<div class="form-control">
  <label class="label" for="cr-url">
    <span class="label-text">Change Request (optional)</span>
  </label>
  <div class="flex gap-2">
    <input
      id="cr-url"
      type="url"
      class="input input-bordered flex-1"
      class:input-error={error}
      class:input-success={crPreview}
      placeholder="Paste ManageEngine CR URL..."
      bind:value
      on:blur={handleBlur}
      disabled={loading}
    />
    {#if value}
      <button class="btn btn-ghost btn-sm self-center" on:click={handleClear} title="Xóa">✕</button>
    {/if}
  </div>

  {#if loading}
    <div class="flex items-center gap-2 mt-2 text-sm opacity-70">
      <span class="loading loading-spinner loading-xs"></span>
      Đang kiểm tra CR...
    </div>
  {/if}

  {#if crPreview}
    <div class="flex items-center gap-2 mt-2 text-sm text-success">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
      </svg>
      <span>
        CR #{crPreview.cr_number}
        {#if crPreview.title}
          — {crPreview.title}
        {/if}
      </span>
    </div>
    {#if crPreview.scheduled_start_time && crPreview.scheduled_end_time}
      <div class="text-xs opacity-60 mt-1 ml-6">
        🕐 Window: {new Date(crPreview.scheduled_start_time).toLocaleString("vi")} → {new Date(crPreview.scheduled_end_time).toLocaleString("vi")}
      </div>
    {/if}
  {/if}

  {#if error}
    <div class="text-sm text-error mt-2">{error}</div>
  {/if}
</div>
