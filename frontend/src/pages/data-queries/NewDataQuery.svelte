<script>
  import { onMount } from "svelte";
  import { push } from "svelte-spa-router";
  import api from "../../api/index.js";
  import SqlEditor from "../../components/SqlEditor.svelte";
  import { toastStore } from "../../stores/toast.js";

  let title = "";
  let databaseId = "";
  let schemaName = "";
  let queryMode = "BASIC";
  let ticketId = "";
  let expiresAt = "";
  let assignedUserIds = [];
  let databases = [];
  let users = [];
  let loading = false;
  let loadingDbs = true;
  let loadingUsers = true;

  // Basic mode state
  let tableName = "";
  let availableColumns = [];
  let selectedColumns = [];
  let filters = [];
  let queryLimit = 1000;
  let generatedSql = "";

  // Advanced mode state
  let sqlContent = "";
  let validationResult = null;
  let validating = false;

  // User search
  let userSearch = "";

  onMount(async () => {
    try {
      databases = await api.databases.myPermissions();
    } catch (e) {
      toastStore.error("Không thể tải danh sách databases.");
    } finally {
      loadingDbs = false;
    }

    try {
      users = await api.users.list();
    } catch (e) {
      // Non-admin might not access user list — use empty
      users = [];
    } finally {
      loadingUsers = false;
    }
  });

  // Auto-generate SQL in Basic mode
  $: if (queryMode === "BASIC" && tableName && selectedColumns.length > 0) {
    generateSql();
  }

  /**
   * Generates a SELECT SQL statement from Basic mode inputs (table, columns, filters, limit).
   * Updates the `generatedSql` reactive variable with the resulting query string.
   */
  function generateSql() {
    const cols = selectedColumns.join(", ");
    let sql = `SELECT ${cols}\nFROM ${schemaName ? schemaName + "." : ""}${tableName}`;

    if (filters.length > 0) {
      const conditions = filters
        .filter((f) => f.column && f.operator && f.value)
        .map((f) => `${f.column} ${f.operator} '${f.value}'`);
      if (conditions.length > 0) {
        sql += `\nWHERE ${conditions.join("\n  AND ")}`;
      }
    }

    sql += `\nLIMIT ${queryLimit}`;
    generatedSql = sql;
  }

  /**
   * Appends a new empty filter entry to the filters list.
   */
  function addFilter() {
    filters = [...filters, { column: "", operator: "=", value: "" }];
  }

  /**
   * Removes a filter entry at the specified index.
   * @param {number} index - The zero-based index of the filter to remove.
   */
  function removeFilter(index) {
    filters = filters.filter((_, i) => i !== index);
  }

  /**
   * Toggles a column's inclusion in the selectedColumns list.
   * Removes the column if already selected, adds it otherwise.
   * @param {string} col - The column name to toggle.
   */
  function toggleColumn(col) {
    if (selectedColumns.includes(col)) {
      selectedColumns = selectedColumns.filter((c) => c !== col);
    } else {
      selectedColumns = [...selectedColumns, col];
    }
  }

  /**
   * Adds a user to the assigned users list (users permitted to execute the query).
   * Clears the user search input after selection.
   * @param {string} userId - The UUID of the user to assign.
   */
  function addAssignedUser(userId) {
    if (!assignedUserIds.includes(userId)) {
      assignedUserIds = [...assignedUserIds, userId];
    }
    userSearch = "";
  }

  /**
   * Removes a user from the assigned users list.
   * @param {string} userId - The UUID of the user to remove.
   */
  function removeAssignedUser(userId) {
    assignedUserIds = assignedUserIds.filter((id) => id !== userId);
  }

  $: filteredUsers = users.filter(
    (u) =>
      !assignedUserIds.includes(u.id) &&
      (u.display_name || u.email || "").toLowerCase().includes(userSearch.toLowerCase())
  );

  /**
   * Handles form submission — creates a Data Query and optionally submits it for approval.
   * Validates required fields (title, database, ticket) and SQL content before calling the API.
   * On success, navigates to the data queries list page.
   * @param {boolean} submit - If true, submits the query for approval after creation;
   *   if false, saves as draft only.
   */
  async function handleSubmit(submit) {
    if (!title.trim() || !databaseId || !ticketId.trim()) {
      toastStore.warning("Vui lòng điền đầy đủ: Tiêu đề, Database, và Ticket ID.");
      return;
    }

    const finalSql = queryMode === "BASIC" ? generatedSql : sqlContent;
    if (!finalSql.trim()) {
      toastStore.warning("SQL query không được để trống.");
      return;
    }

    loading = true;
    try {
      const payload = {
        title: title.trim(),
        database_id: databaseId,
        schema_name: schemaName.trim(),
        query_mode: queryMode,
        sql_query: finalSql.trim(),
        ticket_id: ticketId.trim(),
        assigned_user_ids: assignedUserIds,
      };

      if (expiresAt) {
        payload.expires_at = new Date(expiresAt).toISOString();
      }

      // Basic mode extras
      if (queryMode === "BASIC") {
        payload.table_name = tableName;
        payload.columns = selectedColumns;
        payload.filters = filters.filter((f) => f.column && f.operator && f.value);
        payload.limit = queryLimit;
      }

      const result = await api.dataQueries.create(payload);

      if (submit && result?.id) {
        await api.dataQueries.submit(result.id);
        toastStore.success("Data Query đã được tạo và gửi phê duyệt.");
      } else {
        toastStore.success("Đã lưu bản nháp Data Query.");
      }
      push("/data-queries");
    } catch (e) {
      toastStore.error(e.error || "Không thể tạo Data Query.");
    } finally {
      loading = false;
    }
  }
</script>

<div class="p-6 max-w-5xl mx-auto">
  <div class="mb-6">
    <div class="flex items-center gap-3">
      <a href="#/data-queries" class="btn btn-ghost btn-sm btn-circle">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </a>
      <div>
        <h2 class="text-2xl font-bold">Tạo Data Query mới</h2>
        <p class="text-sm text-base-content/60 mt-0.5">Truy vấn dữ liệu từ database production với kiểm soát phê duyệt</p>
      </div>
    </div>
  </div>

  <!-- Mode Toggle -->
  <div class="tabs tabs-boxed mb-6 w-fit">
    <button class="tab" class:tab-active={queryMode === "BASIC"} on:click={() => (queryMode = "BASIC")}>
      Basic Mode
    </button>
    <button class="tab" class:tab-active={queryMode === "ADVANCED"} on:click={() => (queryMode = "ADVANCED")}>
      Advanced Mode
    </button>
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <!-- Left: Main form -->
    <div class="lg:col-span-2 space-y-4">
      <div class="form-control">
        <label class="label" for="title"><span class="label-text">Tiêu đề</span></label>
        <input id="title" type="text" class="input input-bordered w-full" placeholder="Mô tả mục đích query..." bind:value={title} />
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="form-control">
          <label class="label" for="database"><span class="label-text">Database</span></label>
          <select id="database" class="select select-bordered w-full" bind:value={databaseId}>
            <option value="" disabled>Chọn database...</option>
            {#each databases as db}
              <option value={db.id}>{db.name} ({db.db_type})</option>
            {/each}
          </select>
        </div>

        <div class="form-control">
          <label class="label" for="schema"><span class="label-text">Schema</span></label>
          <input id="schema" type="text" class="input input-bordered w-full" placeholder="e.g. customer" bind:value={schemaName} />
        </div>
      </div>

      {#if queryMode === "BASIC"}
        <!-- Basic Mode: Table & Column Selection -->
        <div class="form-control">
          <label class="label" for="table"><span class="label-text">Table</span></label>
          <input id="table" type="text" class="input input-bordered w-full" placeholder="e.g. transactions" bind:value={tableName} />
        </div>

        <div class="form-control">
          <label class="label"><span class="label-text">Columns (nhập tên, cách nhau bởi dấu phẩy)</span></label>
          <input
            type="text"
            class="input input-bordered w-full"
            placeholder="id, amount, status, created_at"
            on:change={(e) => {
              selectedColumns = e.target.value.split(",").map((c) => c.trim()).filter(Boolean);
            }}
            value={selectedColumns.join(", ")}
          />
        </div>

        <!-- Filters -->
        <div class="form-control">
          <label class="label">
            <span class="label-text">Filters</span>
            <button class="btn btn-xs btn-ghost" on:click={addFilter}>+ Thêm filter</button>
          </label>
          {#each filters as filter, i}
            <div class="flex gap-2 mb-2 items-center">
              <input type="text" class="input input-bordered input-sm flex-1" placeholder="column" bind:value={filter.column} />
              <select class="select select-bordered select-sm w-20" bind:value={filter.operator}>
                <option>=</option>
                <option>!=</option>
                <option>&gt;</option>
                <option>&lt;</option>
                <option>&gt;=</option>
                <option>&lt;=</option>
                <option>LIKE</option>
                <option>IN</option>
              </select>
              <input type="text" class="input input-bordered input-sm flex-1" placeholder="value" bind:value={filter.value} />
              <button class="btn btn-ghost btn-sm btn-circle" on:click={() => removeFilter(i)}>✕</button>
            </div>
          {/each}
        </div>

        <div class="form-control">
          <label class="label" for="limit"><span class="label-text">Limit (max 10,000)</span></label>
          <input id="limit" type="number" class="input input-bordered w-32" min="1" max="10000" bind:value={queryLimit} />
        </div>

        <!-- Generated SQL Preview -->
        {#if generatedSql}
          <div class="form-control">
            <label class="label"><span class="label-text">Generated SQL (read-only)</span></label>
            <SqlEditor value={generatedSql} readonly={true} language="sql" minHeight={120} />
          </div>
        {/if}
      {:else}
        <!-- Advanced Mode: Free-form SQL -->
        <div class="form-control">
          <label class="label"><span class="label-text">SQL Query (SELECT only)</span></label>
          <SqlEditor bind:value={sqlContent} language="sql" placeholder="SELECT ... FROM ... WHERE ... LIMIT 1000" minHeight={200} />
        </div>
      {/if}
    </div>

    <!-- Right: Metadata -->
    <div class="space-y-4">
      <div class="card bg-base-100 border border-base-300 p-4 space-y-4">
        <h3 class="font-semibold text-sm uppercase text-base-content/70">Thông tin bổ sung</h3>

        <div class="form-control">
          <label class="label" for="ticket"><span class="label-text">Ticket ID (ServiceDesk)</span></label>
          <input id="ticket" type="text" class="input input-bordered input-sm w-full" placeholder="INC-12345" bind:value={ticketId} />
        </div>

        <div class="form-control">
          <label class="label" for="expires"><span class="label-text">Hết hạn (tùy chọn)</span></label>
          <input id="expires" type="date" class="input input-bordered input-sm w-full" bind:value={expiresAt} />
        </div>

        <!-- Assigned Users -->
        <div class="form-control">
          <label class="label"><span class="label-text">Assign Users (được phép run)</span></label>
          <div class="flex flex-wrap gap-1 mb-2">
            {#each assignedUserIds as userId}
              {@const user = users.find((u) => u.id === userId)}
              <span class="badge badge-primary gap-1">
                {user?.display_name || user?.email || userId}
                <button class="btn btn-ghost btn-xs p-0" on:click={() => removeAssignedUser(userId)}>✕</button>
              </span>
            {/each}
          </div>
          <input
            type="text"
            class="input input-bordered input-sm w-full"
            placeholder="Tìm user..."
            bind:value={userSearch}
          />
          {#if userSearch && filteredUsers.length > 0}
            <ul class="menu bg-base-100 border border-base-300 rounded-box mt-1 max-h-32 overflow-y-auto p-1">
              {#each filteredUsers.slice(0, 5) as u}
                <li>
                  <button class="text-sm" on:click={() => addAssignedUser(u.id)}>
                    {u.display_name || u.email}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>

      <!-- Actions -->
      <div class="flex flex-col gap-2">
        <button class="btn btn-primary w-full" on:click={() => handleSubmit(true)} disabled={loading}>
          {#if loading}<span class="loading loading-spinner loading-sm"></span>{/if}
          Gửi phê duyệt
        </button>
        <button class="btn btn-outline w-full" on:click={() => handleSubmit(false)} disabled={loading}>
          Lưu nháp
        </button>
      </div>
    </div>
  </div>
</div>
