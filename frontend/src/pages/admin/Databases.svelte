<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";
  import { toastStore } from "../../stores/toast.js";
  import Skeleton from "../../components/Skeleton.svelte";
  import ConfirmModal from "../../components/ConfirmModal.svelte";

  let databases = [];
  let loading = true;
  let error = null;
  let showForm = false;
  let testingNew = false;
  let testNewResult = null;

  // Per-database test status
  let testingId = null;

  // Form fields
  let form = { name: "", db_type: "postgresql", host: "", port: 5432, database_name: "", schema_name: "", username: "", password: "", timeout_secs: 30 };

  // Edit state
  let editingDb = null;
  let editForm = { name: "", host: "", port: 5432, database_name: "", schema_name: "", timeout_secs: 30 };
  let editSaving = false;
  let editError = null;

  // Confirm modal
  let confirmOpen = false;
  let confirmAction = null;
  let confirmTitle = "";
  let confirmMessage = "";
  let confirmVariant = "warning";
  let confirmLoading = false;

  // DB type metadata
  const dbTypeInfo = {
    postgresql: { label: "PostgreSQL", color: "text-blue-500", icon: "🐘", defaultPort: 5432 },
    mysql: { label: "MySQL", color: "text-orange-500", icon: "🐬", defaultPort: 3306 },
    oracle: { label: "Oracle", color: "text-red-500", icon: "🔴", defaultPort: 1521 },
    mongodb: { label: "MongoDB", color: "text-green-600", icon: "🍃", defaultPort: 27017 },
  };

  onMount(loadDatabases);

  /**
   * Tải danh sách database connections từ API.
   * Cập nhật state `databases`, `loading`, và `error`.
   */
  async function loadDatabases() {
    loading = true;
    error = null;
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
   * Validate input cơ bản, sau khi tạo thành công reset form và reload danh sách.
   */
  async function handleCreate() {
    if (!form.name.trim() || !form.host.trim() || !form.database_name.trim()) {
      toastStore.warning("Vui lòng điền đầy đủ: Tên, Host, Database name.");
      return;
    }
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
      toastStore.success("Đã tạo database connection mới.");
      loadDatabases();
    } catch (e) {
      toastStore.error(e.error || "Không thể tạo.");
    }
  }

  /**
   * Kiểm tra kết nối cho một database đã lưu và hiển thị kết quả qua toast.
   * @param {object} db - Database connection object cần test.
   */
  async function handleTest(db) {
    testingId = db.id;
    try {
      const result = await api.databases.test(db.id);
      if (result.status === "ok") {
        toastStore.success(`${db.name}: Kết nối thành công!`);
      } else {
        toastStore.error(`${db.name}: ${result.message}`);
      }
    } catch (e) {
      toastStore.error(`${db.name}: ${e.error || "Không thể kết nối."}`);
    } finally {
      testingId = null;
    }
  }

  /**
   * Xử lý bật/tắt trạng thái active của database.
   * Nếu đang active, hiển thị confirm modal trước khi disable.
   * @param {object} db - Database connection object cần toggle.
   */
  function handleToggle(db) {
    if (db.is_active) {
      confirmTitle = "Disable Database";
      confirmMessage = `Disable "${db.name}"? Makers sẽ không thể tạo request mới trên database này.`;
      confirmVariant = "warning";
      confirmAction = () => doToggle(db);
      confirmOpen = true;
    } else {
      doToggle(db);
    }
  }

  /**
   * Thực hiện toggle trạng thái active/inactive cho database connection.
   * Được gọi trực tiếp (enable) hoặc qua confirm modal (disable).
   * @param {object} db - Database connection object cần toggle.
   */
  async function doToggle(db) {
    if (confirmOpen) confirmLoading = true;
    try {
      await api.databases.update(db.id, { is_active: !db.is_active });
      confirmOpen = false;
      toastStore.success(db.is_active ? `${db.name} đã bị disable.` : `${db.name} đã được enable.`);
      loadDatabases();
    } catch (e) {
      toastStore.error(e.error || "Lỗi cập nhật trạng thái.");
    } finally {
      confirmLoading = false;
    }
  }

  /**
   * Mở modal chỉnh sửa database connection và populate form với dữ liệu hiện tại.
   * @param {object} db - Database connection object cần chỉnh sửa.
   */
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

  /** Đóng modal chỉnh sửa và reset error state. */
  function closeEdit() {
    editingDb = null;
    editError = null;
  }

  /**
   * Gửi cập nhật thông tin database connection từ edit form.
   * Sau khi thành công, đóng modal và reload danh sách.
   */
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
      toastStore.success("Đã cập nhật thông tin database.");
      loadDatabases();
    } catch (e) {
      editError = e.error || "Không thể cập nhật.";
    } finally {
      editSaving = false;
    }
  }

  // Auto-update port when db_type changes
  $: {
    const info = dbTypeInfo[form.db_type];
    if (info && form.port === 5432 || form.port === 3306 || form.port === 1521 || form.port === 27017) {
      form.port = info.defaultPort;
    }
  }

  /** Handler cho sự kiện confirm từ ConfirmModal — thực thi action đã lưu. */
  function onConfirm() {
    if (confirmAction) confirmAction();
  }

  /** Handler cho sự kiện cancel từ ConfirmModal — đóng modal và reset state. */
  function onCancelModal() {
    confirmOpen = false;
    confirmAction = null;
    confirmLoading = false;
  }
</script>

<div class="p-6">
  <!-- Header -->
  <div class="flex justify-between items-center mb-6">
    <div>
      <h2 class="text-2xl font-bold">Quản lý Databases</h2>
      <p class="text-sm text-base-content/60 mt-1">Quản lý kết nối đến các database đích</p>
    </div>
    <button
      class="btn btn-primary btn-sm gap-1"
      on:click={() => (showForm = !showForm)}
    >
      {#if showForm}
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
        Đóng
      {:else}
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        Thêm mới
      {/if}
    </button>
  </div>

  <!-- Create Form -->
  {#if showForm}
    <div class="card bg-base-100 shadow-sm border border-base-300 mb-6">
      <div class="card-body">
        <h3 class="text-lg font-semibold mb-4 flex items-center gap-2">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
          </svg>
          Thêm Database Connection
        </h3>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label" for="new-name"><span class="label-text font-medium">Tên hiển thị</span></label>
            <input id="new-name" class="input input-bordered" placeholder="VD: ORACLE_PROD" bind:value={form.name} />
          </div>
          <div class="form-control">
            <label class="label" for="new-type"><span class="label-text font-medium">Loại database</span></label>
            <select id="new-type" class="select select-bordered" bind:value={form.db_type}>
              {#each Object.entries(dbTypeInfo) as [key, info]}
                <option value={key}>{info.icon} {info.label}</option>
              {/each}
            </select>
          </div>
          <div class="form-control">
            <label class="label" for="new-host"><span class="label-text font-medium">Host</span></label>
            <input id="new-host" class="input input-bordered" placeholder="hostname hoặc IP" bind:value={form.host} />
          </div>
          <div class="form-control">
            <label class="label" for="new-port"><span class="label-text font-medium">Port</span></label>
            <input id="new-port" class="input input-bordered" type="number" bind:value={form.port} />
          </div>
          <div class="form-control">
            <label class="label" for="new-dbname"><span class="label-text font-medium">Database name</span></label>
            <input id="new-dbname" class="input input-bordered" placeholder="tên database" bind:value={form.database_name} />
          </div>
          <div class="form-control">
            <label class="label" for="new-schema"><span class="label-text font-medium">Schema <span class="opacity-50">(optional)</span></span></label>
            <input id="new-schema" class="input input-bordered" placeholder="schema name" bind:value={form.schema_name} />
          </div>
          <div class="form-control">
            <label class="label" for="new-user"><span class="label-text font-medium">Username</span></label>
            <input id="new-user" class="input input-bordered" placeholder="database user" bind:value={form.username} />
          </div>
          <div class="form-control">
            <label class="label" for="new-pass"><span class="label-text font-medium">Password</span></label>
            <input id="new-pass" class="input input-bordered" type="password" placeholder="••••••••" bind:value={form.password} />
          </div>
          <div class="form-control">
            <label class="label" for="new-timeout"><span class="label-text font-medium">Timeout (giây)</span></label>
            <input id="new-timeout" class="input input-bordered" type="number" bind:value={form.timeout_secs} />
          </div>
        </div>

        <!-- Test result -->
        {#if testNewResult}
          <div class="alert {testNewResult.status === 'ok' ? 'alert-success' : 'alert-error'} mt-4">
            <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-5 w-5" fill="none" viewBox="0 0 24 24">
              {#if testNewResult.status === 'ok'}
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              {:else}
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
              {/if}
            </svg>
            <span>{testNewResult.status === 'ok' ? 'Kết nối thành công!' : testNewResult.message}</span>
          </div>
        {/if}

        <div class="divider my-2"></div>

        <div class="flex flex-wrap gap-2">
          <button class="btn btn-primary" on:click={handleCreate}>
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
            Tạo
          </button>
          <button
            class="btn btn-outline btn-info"
            on:click={handleTestNew}
            disabled={testingNew || !form.host || !form.database_name || !form.username || !form.password}
          >
            {#if testingNew}
              <span class="loading loading-spinner loading-xs"></span>
            {:else}
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
            {/if}
            Kiểm tra kết nối
          </button>
          <button class="btn btn-ghost" on:click={() => { showForm = false; testNewResult = null; }}>Hủy</button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Database List -->
  {#if loading}
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      {#each Array(4) as _}
        <Skeleton type="card" />
      {/each}
    </div>
  {:else if error}
    <div class="alert alert-error">
      <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>{error}</span>
      <button class="btn btn-sm btn-ghost" on:click={loadDatabases}>Thử lại</button>
    </div>
  {:else if databases.length === 0}
    <div class="text-center py-16">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-16 w-16 mx-auto text-base-content/20 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
      </svg>
      <p class="text-base-content/50 mb-4">Chưa có database nào được cấu hình.</p>
      <button class="btn btn-primary btn-sm" on:click={() => (showForm = true)}>Thêm database đầu tiên</button>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      {#each databases as db}
        {@const info = dbTypeInfo[db.db_type] || { label: db.db_type, color: "text-base-content", icon: "💾", defaultPort: 0 }}
        <div class="card bg-base-100 shadow-sm border border-base-300 hover:shadow-md transition-shadow {!db.is_active ? 'opacity-60' : ''}">
          <div class="card-body p-5">
            <!-- Header row -->
            <div class="flex items-start justify-between">
              <div class="flex items-center gap-3">
                <span class="text-2xl">{info.icon}</span>
                <div>
                  <h3 class="font-semibold text-base">{db.name}</h3>
                  <span class="text-xs {info.color} font-medium">{info.label}</span>
                </div>
              </div>
              <div class="flex items-center gap-1.5">
                {#if db.is_active}
                  <span class="badge badge-success badge-sm gap-1">
                    <span class="w-1.5 h-1.5 rounded-full bg-current"></span>
                    Active
                  </span>
                {:else}
                  <span class="badge badge-ghost badge-sm">Inactive</span>
                {/if}
              </div>
            </div>

            <!-- Connection info -->
            <div class="mt-3 bg-base-200 rounded-lg p-3">
              <div class="flex items-center gap-2 text-sm font-mono text-base-content/70">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
                </svg>
                <span>{db.host}:{db.port}</span>
              </div>
              <div class="flex items-center gap-2 text-sm font-mono text-base-content/70 mt-1">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7" />
                </svg>
                <span>{db.database_name}{db.schema_name ? ` / ${db.schema_name}` : ''}</span>
              </div>
              {#if db.timeout_secs}
                <div class="flex items-center gap-2 text-sm text-base-content/50 mt-1">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  <span>Timeout: {db.timeout_secs}s</span>
                </div>
              {/if}
            </div>

            <!-- Actions -->
            <div class="flex gap-2 mt-3 pt-3 border-t border-base-200">
              <button
                class="btn btn-sm btn-ghost gap-1"
                on:click={() => handleTest(db)}
                disabled={testingId === db.id}
              >
                {#if testingId === db.id}
                  <span class="loading loading-spinner loading-xs"></span>
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                  </svg>
                {/if}
                Test
              </button>
              <button class="btn btn-sm btn-ghost gap-1" on:click={() => openEdit(db)}>
                <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                </svg>
                Edit
              </button>
              <button
                class="btn btn-sm btn-ghost gap-1 {db.is_active ? 'text-warning' : 'text-success'}"
                on:click={() => handleToggle(db)}
              >
                {#if db.is_active}
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
                  </svg>
                  Disable
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  Enable
                {/if}
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Edit Modal -->
  {#if editingDb}
    {@const info = dbTypeInfo[editingDb.db_type] || { label: editingDb.db_type, icon: "💾" }}
    <div class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4 flex items-center gap-2">
          <span>{info.icon}</span>
          Chỉnh sửa: {editingDb.name}
        </h3>
        {#if editError}
          <div class="alert alert-error mb-3"><span>{editError}</span></div>
        {/if}
        <div class="grid grid-cols-1 gap-3">
          <div class="form-control">
            <label class="label" for="edit-name"><span class="label-text">Tên hiển thị</span></label>
            <input id="edit-name" class="input input-bordered" bind:value={editForm.name} />
          </div>
          <div class="form-control">
            <label class="label" for="edit-type"><span class="label-text">Loại</span></label>
            <input id="edit-type" class="input input-bordered bg-base-200" value="{info.icon} {info.label}" disabled />
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div class="form-control">
              <label class="label" for="edit-host"><span class="label-text">Host</span></label>
              <input id="edit-host" class="input input-bordered" bind:value={editForm.host} />
            </div>
            <div class="form-control">
              <label class="label" for="edit-port"><span class="label-text">Port</span></label>
              <input id="edit-port" class="input input-bordered" type="number" bind:value={editForm.port} />
            </div>
          </div>
          <div class="form-control">
            <label class="label" for="edit-dbname"><span class="label-text">Database name</span></label>
            <input id="edit-dbname" class="input input-bordered" bind:value={editForm.database_name} />
          </div>
          <div class="form-control">
            <label class="label" for="edit-schema"><span class="label-text">Schema (optional)</span></label>
            <input id="edit-schema" class="input input-bordered" bind:value={editForm.schema_name} />
          </div>
          <div class="form-control">
            <label class="label" for="edit-timeout"><span class="label-text">Timeout (giây)</span></label>
            <input id="edit-timeout" class="input input-bordered" type="number" bind:value={editForm.timeout_secs} />
          </div>
        </div>
        <div class="modal-action">
          <button class="btn btn-ghost" on:click={closeEdit}>Hủy</button>
          <button class="btn btn-primary" on:click={handleEdit} disabled={editSaving}>
            {#if editSaving}
              <span class="loading loading-spinner loading-xs"></span>
            {/if}
            Lưu thay đổi
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button on:click={closeEdit}>close</button>
      </form>
    </div>
  {/if}
</div>

<!-- Confirm Modal -->
<ConfirmModal
  bind:open={confirmOpen}
  title={confirmTitle}
  message={confirmMessage}
  variant={confirmVariant}
  loading={confirmLoading}
  on:confirm={onConfirm}
  on:cancel={onCancelModal}
/>
