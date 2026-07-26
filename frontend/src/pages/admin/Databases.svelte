<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";

  let databases = [];
  let loading = true;
  let error = null;
  let showForm = false;
  let testResult = null;
  let testingNew = false;
  let testNewResult = null;

  // Form fields
  let form = { name: "", db_type: "postgresql", host: "", port: 5432, database_name: "", schema_name: "", username: "", password: "", timeout_secs: 30 };

  // Edit state
  let editingDb = null;
  let editForm = { name: "", host: "", port: 5432, database_name: "", schema_name: "", timeout_secs: 30 };
  let editSaving = false;
  let editError = null;

  onMount(loadDatabases);

  /**
   * Tải danh sách database connections từ API.
   * Cập nhật state `databases`, `loading`, và `error`.
   */
  async function loadDatabases() {
    loading = true;
    try {
      databases = await api.databases.list();
    } catch (e) {
      error = e.error || "Không thể tải danh sách.";
    } finally {
      loading = false;
    }
  }

  /**
   * Kiểm tra kết nối cho database mới (chưa lưu) bằng thông tin trong form.
   * Gọi API test-new và cập nhật `testNewResult` với kết quả.
   */
  async function handleTestNew() {
    testingNew = true;
    testNewResult = null;
    try {
      const payload = {
        ...form,
        port: parseInt(form.port),
        timeout_secs: parseInt(form.timeout_secs),
        schema_name: form.schema_name || null,
      };
      testNewResult = await api.databases.testNew(payload);
    } catch (e) {
      testNewResult = { status: "error", message: e.error || "Không thể kết nối." };
    } finally {
      testingNew = false;
    }
  }

  /**
   * Tạo database connection mới từ dữ liệu form.
   * Sau khi tạo thành công, reset form và reload danh sách.
   */
  async function handleCreate() {
    try {
      await api.databases.create({
        ...form,
        port: parseInt(form.port),
        timeout_secs: parseInt(form.timeout_secs),
        schema_name: form.schema_name || null,
      });
      showForm = false;
      testNewResult = null;
      form = { name: "", db_type: "postgresql", host: "", port: 5432, database_name: "", schema_name: "", username: "", password: "", timeout_secs: 30 };
      loadDatabases();
    } catch (e) {
      error = e.error || "Không thể tạo.";
    }
  }

  /**
   * Kiểm tra kết nối cho một database đã lưu.
   * @param {string} id - UUID của database connection cần test.
   */
  async function handleTest(id) {
    testResult = null;
    try {
      testResult = await api.databases.test(id);
    } catch (e) {
      testResult = { status: "error", message: e.error };
    }
  }

  /**
   * Bật/tắt trạng thái active của một database connection.
   * @param {string} id - UUID của database connection.
   * @param {boolean} isActive - Trạng thái hiện tại (sẽ đảo ngược).
   */
  async function handleToggle(id, isActive) {
    try {
      await api.databases.update(id, { is_active: !isActive });
      loadDatabases();
    } catch (e) {
      alert(e.error || "Lỗi.");
    }
  }

  function openEdit(db) {
    editingDb = db;
    editForm = {
      name: db.name,
      host: db.host,
      port: db.port,
      database_name: db.database_name,
      schema_name: db.schema_name || "",
      timeout_secs: db.timeout_secs || 30,
    };
    editError = null;
  }

  function closeEdit() {
    editingDb = null;
    editError = null;
  }

  async function handleEdit() {
    editSaving = true;
    editError = null;
    try {
      const payload = {
        name: editForm.name,
        host: editForm.host,
        port: parseInt(editForm.port),
        database_name: editForm.database_name,
        schema_name: editForm.schema_name || null,
        timeout_secs: parseInt(editForm.timeout_secs),
      };
      await api.databases.update(editingDb.id, payload);
      closeEdit();
      loadDatabases();
    } catch (e) {
      editError = e.error || "Không thể cập nhật.";
    } finally {
      editSaving = false;
    }
  }
</script>

<div class="p-6">
  <div class="flex justify-between items-center mb-6">
    <h2 class="text-2xl font-bold">Quản lý Databases</h2>
    <button class="btn btn-primary btn-sm" on:click={() => (showForm = !showForm)}>
      {showForm ? "Đóng" : "+ Thêm mới"}
    </button>
  </div>

  {#if error}
    <div class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}

  {#if testResult}
    <div class="alert {testResult.status === 'ok' ? 'alert-success' : 'alert-error'} mb-4">
      <span>{testResult.message}</span>
    </div>
  {/if}

  {#if showForm}
    <div class="card bg-base-100 shadow mb-6">
      <div class="card-body">
        <h3 class="card-title text-lg">Thêm Database Connection</h3>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
          <input class="input input-bordered" placeholder="Tên (VD: ORACLE_PROD)" bind:value={form.name} />
          <select class="select select-bordered" bind:value={form.db_type}>
            <option value="postgresql">PostgreSQL</option>
            <option value="mysql">MySQL</option>
            <option value="oracle">Oracle</option>
            <option value="mongodb">MongoDB</option>
          </select>
          <input class="input input-bordered" placeholder="Host" bind:value={form.host} />
          <input class="input input-bordered" type="number" placeholder="Port" bind:value={form.port} />
          <input class="input input-bordered" placeholder="Database name" bind:value={form.database_name} />
          <input class="input input-bordered" placeholder="Schema (optional)" bind:value={form.schema_name} />
          <input class="input input-bordered" placeholder="Username" bind:value={form.username} />
          <input class="input input-bordered" type="password" placeholder="Password" bind:value={form.password} />
          <input class="input input-bordered" type="number" placeholder="Timeout (s)" bind:value={form.timeout_secs} />
        </div>
        <div class="card-actions mt-4">
          <button class="btn btn-primary" on:click={handleCreate}>Tạo</button>
          <button class="btn btn-outline btn-info" on:click={handleTestNew} disabled={testingNew || !form.host || !form.database_name || !form.username || !form.password}>
            {#if testingNew}
              <span class="loading loading-spinner loading-xs"></span>
              Đang kiểm tra...
            {:else}
              Kiểm tra kết nối
            {/if}
          </button>
          <button class="btn btn-ghost" on:click={() => (showForm = false)}>Hủy</button>
        </div>
        {#if testNewResult}
          <div class="alert {testNewResult.status === 'ok' ? 'alert-success' : 'alert-error'} mt-3">
            <span>{testNewResult.status === 'ok' ? '✓ Kết nối thành công!' : `✗ ${testNewResult.message}`}</span>
          </div>
        {/if}
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="flex justify-center py-10"><span class="loading loading-spinner"></span></div>
  {:else}
    <div class="overflow-x-auto">
      <table class="table w-full">
        <thead>
          <tr>
            <th>Tên</th>
            <th>Loại</th>
            <th>Host</th>
            <th>Trạng thái</th>
            <th>Hành động</th>
          </tr>
        </thead>
        <tbody>
          {#each databases as db}
            <tr>
              <td class="font-medium">{db.name}</td>
              <td><span class="badge badge-sm badge-outline">{db.db_type}</span></td>
              <td class="text-sm">{db.host}:{db.port}</td>
              <td>
                {#if db.is_active}
                  <span class="badge badge-success badge-sm">Active</span>
                {:else}
                  <span class="badge badge-ghost badge-sm">Inactive</span>
                {/if}
              </td>
              <td class="flex gap-1">
                <button class="btn btn-xs btn-outline btn-info" on:click={() => openEdit(db)}>Edit</button>
                <button class="btn btn-xs btn-outline" on:click={() => handleTest(db.id)}>Test</button>
                <button class="btn btn-xs btn-ghost" on:click={() => handleToggle(db.id, db.is_active)}>
                  {db.is_active ? "Disable" : "Enable"}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  <!-- Edit Modal -->
  {#if editingDb}
    <div class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">Chỉnh sửa: {editingDb.name}</h3>
        {#if editError}
          <div class="alert alert-error mb-3"><span>{editError}</span></div>
        {/if}
        <div class="grid grid-cols-1 gap-3">
          <div class="form-control">
            <label class="label"><span class="label-text">Tên</span></label>
            <input class="input input-bordered" bind:value={editForm.name} />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">Loại</span></label>
            <input class="input input-bordered" value={editingDb.db_type} disabled />
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div class="form-control">
              <label class="label"><span class="label-text">Host</span></label>
              <input class="input input-bordered" bind:value={editForm.host} />
            </div>
            <div class="form-control">
              <label class="label"><span class="label-text">Port</span></label>
              <input class="input input-bordered" type="number" bind:value={editForm.port} />
            </div>
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">Database name</span></label>
            <input class="input input-bordered" bind:value={editForm.database_name} />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">Schema (optional)</span></label>
            <input class="input input-bordered" bind:value={editForm.schema_name} />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">Timeout (giây)</span></label>
            <input class="input input-bordered" type="number" bind:value={editForm.timeout_secs} />
          </div>
        </div>
        <div class="modal-action">
          <button class="btn btn-primary" on:click={handleEdit} disabled={editSaving}>
            {#if editSaving}
              <span class="loading loading-spinner loading-xs"></span> Đang lưu...
            {:else}
              Lưu thay đổi
            {/if}
          </button>
          <button class="btn btn-ghost" on:click={closeEdit}>Hủy</button>
        </div>
      </div>
      <div class="modal-backdrop" on:click={closeEdit}></div>
    </div>
  {/if}
</div>
