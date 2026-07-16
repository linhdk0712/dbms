<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import SqlEditor from "../../components/SqlEditor.svelte";

  let description = "";
  let sqlContent = "";
  let databaseId = "";
  let databases = [];
  let loading = false;
  let error = null;
  let selectedDbType = "sql";

  onMount(async () => {
    try {
      databases = await api.databases.myPermissions();
    } catch (e) {
      error = "Không thể tải danh sách databases.";
    }
  });

  // Detect language dựa trên db_type đã chọn
  $: {
    const selectedDb = databases.find((db) => db.id === databaseId);
    selectedDbType = selectedDb?.db_type === "mongodb" ? "json" : "sql";
  }

  async function handleSubmit(submit) {
    if (!description.trim() || !sqlContent.trim() || !databaseId) {
      error = "Vui lòng điền đầy đủ thông tin.";
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
      });
      push("/requests");
    } catch (e) {
      error = e.error || "Không thể tạo request.";
    } finally {
      loading = false;
    }
  }
</script>

<div class="p-6 max-w-4xl mx-auto">
  <h2 class="text-2xl font-bold mb-6">Tạo SQL Request mới</h2>

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
      {loading ? "Đang lưu..." : "Lưu nháp"}
    </button>
    <button
      class="btn btn-primary"
      on:click={() => handleSubmit(true)}
      disabled={loading}
    >
      {loading ? "Đang gửi..." : "Gửi yêu cầu"}
    </button>
  </div>
</div>
