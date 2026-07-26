<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import SqlEditor from "../../components/SqlEditor.svelte";
  import CrInput from "../../components/CrInput.svelte";
  import { toastStore } from "../../stores/toast.js";

  let description = "";
  let sqlContent = "";
  let databaseId = "";
  let crUrl = "";
  let databases = [];
  let loading = false;
  let loadingDbs = true;
  let error = null;
  let selectedDbType = "sql";

  onMount(async () => {
    try {
      databases = await api.databases.myPermissions();
    } catch (e) {
      toastStore.error("Không thể tải danh sách databases.");
    } finally {
      loadingDbs = false;
    }
  });

  // Detect language dựa trên db_type đã chọn
  $: {
    const selectedDb = databases.find((db) => db.id === databaseId);
    selectedDbType = selectedDb?.db_type === "mongodb" ? "json" : "sql";
  }

  /**
   * Tạo SQL request mới và điều hướng về danh sách requests.
   * Validate input trước khi gọi API — hiển thị toast nếu thiếu thông tin.
   * @param {boolean} submit - true để tạo và gửi luôn (PENDING), false để lưu nháp (DRAFT)
   */
  async function handleSubmit(submit) {
    if (!description.trim() || !sqlContent.trim() || !databaseId) {
      toastStore.warning("Vui lòng điền đầy đủ thông tin.");
      return;
    }

    loading = true;
    error = null;

    try {
      await api.requests.create({
        description: description.trim(),
        sql_content: sqlContent.trim(),
        database_id: databaseId,
        submit,
        cr_url: crUrl.trim() || undefined,
      });
      toastStore.success(submit ? "Request đã được tạo và gửi." : "Đã lưu bản nháp.");
      push("/requests");
    } catch (e) {
      error = e.error || "Không thể tạo request.";
      toastStore.error(error);
    } finally {
      loading = false;
    }
  }
</script>

<div class="p-6 max-w-4xl mx-auto">
  <div class="mb-6">
    <div class="flex items-center gap-3">
      <a href="#/requests" class="btn btn-ghost btn-sm btn-circle">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </a>
      <div>
        <h2 class="text-2xl font-bold">Tạo SQL Request mới</h2>
        <p class="text-sm text-base-content/60 mt-0.5">Soạn câu lệnh SQL và gửi yêu cầu thực thi</p>
      </div>
    </div>
  </div>

  {#if error}
    <div class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}

  <div class="form-control mb-4">
    <label class="label" for="description"><span class="label-text">Mô tả</span></label>
    <input
      id="description"
      type="text"
      class="input input-bordered w-full"
      placeholder="Mô tả ngắn gọn mục đích..."
      bind:value={description}
    />
  </div>

  <div class="form-control mb-4">
    <label class="label" for="database"><span class="label-text">Database</span></label>
    <select id="database" class="select select-bordered w-full" bind:value={databaseId}>
      <option value="" disabled>Chọn database...</option>
      {#each databases as db}
        <option value={db.id}>{db.name} ({db.db_type} - {db.host})</option>
      {/each}
    </select>
  </div>

  <div class="mb-4">
    <CrInput bind:value={crUrl} />
  </div>

  <div class="form-control mb-6">
    <label class="label" for="sql">
      <span class="label-text">
        {selectedDbType === "json" ? "MongoDB Commands (JSON)" : "SQL Content"}
      </span>
    </label>
    <SqlEditor
      bind:value={sqlContent}
      language={selectedDbType}
      placeholder={selectedDbType === "json" ? '{"insert": "collection", "documents": [...]}' : "INSERT INTO ..."}
      minHeight={256}
    />
  </div>

  <div class="flex gap-3">
    <button
      class="btn btn-outline"
      on:click={() => handleSubmit(false)}
      disabled={loading}
    >
      {#if loading}<span class="loading loading-spinner loading-sm"></span>{/if}
      Lưu nháp
    </button>
    <button
      class="btn btn-primary"
      on:click={() => handleSubmit(true)}
      disabled={loading}
    >
      {#if loading}<span class="loading loading-spinner loading-sm"></span>{/if}
      Gửi yêu cầu
    </button>
  </div>
</div>
