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

  onMount(loadDatabases);

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

  async function handleTest(id) {
    testResult = null;
    try {
      testResult = await api.databases.test(id);
    } catch (e) {
      testResult = { status: "error", message: e.error };
    }
  }

  async function handleToggle(id, isActive) {
    try {
      await api.databases.update(id, { is_active: !isActive });
      loadDatabases();
    } catch (e) {
      alert(e.error || "Lỗi.");
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
</div>
