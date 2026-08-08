<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";
  import { toastStore } from "../../stores/toast.js";

  let rules = [];
  let databases = [];
  let loading = true;
  let showCreateModal = false;
  let editingRule = null;
  let saving = false;

  // New rule form
  let newRule = {
    database_id: "",
    schema_name: "",
    table_name: "",
    levels: [{ level_order: 1, approver_unit: "ITO", required: true }],
  };

  const approverUnits = [];
  let approverUnitsLoaded = false;

  onMount(async () => {
    await Promise.all([loadRules(), loadDatabases(), loadApproverUnits()]);
  });

  /**
   * Tải danh sách approver units từ API.
   * @returns {Promise<void>}
   */
  async function loadApproverUnits() {
    try {
      const result = await api.approverUnits.list();
      approverUnits.length = 0;
      (result || []).filter(u => u.is_active).forEach(u => approverUnits.push(u.name));
      approverUnitsLoaded = true;
    } catch (e) {
      // Fallback nếu không load được
      approverUnits.push("ITO", "ITS", "COMPLIANCE");
      approverUnitsLoaded = true;
    }
  }

  /**
   * Tải danh sách approval rules từ API và cập nhật state.
   * @returns {Promise<void>}
   */
  async function loadRules() {
    loading = true;
    try {
      const result = await api.dataQueryRules.list();
      rules = Array.isArray(result) ? result : [];
    } catch (e) {
      toastStore.error("Không thể tải approval rules.");
      rules = [];
    } finally {
      loading = false;
    }
  }

  /**
   * Tải danh sách databases từ API để hiển thị trong dropdown.
   * @returns {Promise<void>}
   */
  async function loadDatabases() {
    try {
      databases = await api.databases.list();
    } catch (e) {
      databases = [];
    }
  }

  /**
   * Mở modal tạo approval rule mới với form trống.
   */
  function openCreate() {
    newRule = {
      database_id: "",
      schema_name: "",
      table_name: "",
      levels: [{ level_order: 1, approver_unit: "ITO", required: true }],
    };
    editingRule = null;
    showCreateModal = true;
  }

  /**
   * Mở modal chỉnh sửa một approval rule đã tồn tại.
   * @param {object} rule - Rule object cần chỉnh sửa
   */
  function openEdit(rule) {
    editingRule = rule;
    newRule = {
      database_id: rule.database_id,
      schema_name: rule.schema_name || "",
      table_name: rule.table_name || "",
      levels: rule.levels?.length
        ? [...rule.levels]
        : [{ level_order: 1, approver_unit: "ITO", required: true }],
    };
    showCreateModal = true;
  }

  /**
   * Thêm một approval level mới vào danh sách levels của rule đang tạo/sửa.
   * Tự động chọn approver unit chưa được sử dụng.
   */
  function addLevel() {
    const nextOrder = newRule.levels.length + 1;
    const usedUnits = newRule.levels.map((l) => l.approver_unit);
    const availableUnit = approverUnits.find((u) => !usedUnits.includes(u)) || "ITO";
    newRule.levels = [
      ...newRule.levels,
      { level_order: nextOrder, approver_unit: availableUnit, required: true },
    ];
  }

  /**
   * Xóa một approval level theo index và re-index lại level_order.
   * @param {number} index - Vị trí của level cần xóa trong mảng
   */
  function removeLevel(index) {
    newRule.levels = newRule.levels
      .filter((_, i) => i !== index)
      .map((l, i) => ({ ...l, level_order: i + 1 }));
  }

  /**
   * Validate và lưu approval rule (tạo mới hoặc cập nhật).
   * Hiển thị toast thông báo kết quả và reload danh sách rules.
   * @returns {Promise<void>}
   */
  async function handleSave() {
    if (!newRule.database_id) {
      toastStore.warning("Vui lòng chọn database.");
      return;
    }
    if (newRule.levels.length === 0) {
      toastStore.warning("Cần ít nhất 1 level phê duyệt.");
      return;
    }

    saving = true;
    try {
      const payload = {
        database_id: newRule.database_id,
        schema_name: newRule.schema_name.trim() || null,
        table_name: newRule.table_name.trim() || null,
        levels: newRule.levels,
      };

      if (editingRule) {
        await api.dataQueryRules.update(editingRule.id, payload);
        toastStore.success("Đã cập nhật approval rule.");
      } else {
        await api.dataQueryRules.create(payload);
        toastStore.success("Đã tạo approval rule mới.");
      }
      showCreateModal = false;
      await loadRules();
    } catch (e) {
      toastStore.error(e.error || "Không thể lưu rule.");
    } finally {
      saving = false;
    }
  }

  /**
   * Xóa một approval rule sau khi user xác nhận.
   * @param {string} ruleId - UUID của rule cần xóa
   * @returns {Promise<void>}
   */
  async function handleDelete(ruleId) {
    if (!confirm("Bạn chắc chắn muốn xóa rule này?")) return;
    try {
      await api.dataQueryRules.delete(ruleId);
      toastStore.success("Đã xóa rule.");
      await loadRules();
    } catch (e) {
      toastStore.error(e.error || "Không thể xóa rule.");
    }
  }

  /**
   * Tra cứu tên database theo ID từ danh sách đã load.
   * @param {string} dbId - UUID của database connection
   * @returns {string} Tên database hoặc ID nếu không tìm thấy
   */
  function getDbName(dbId) {
    const db = databases.find((d) => d.id === dbId);
    return db?.name || dbId;
  }
</script>

<div class="p-6">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-2xl font-bold">Ma trận Phê duyệt</h2>
      <p class="text-sm text-base-content/60 mt-0.5">Cấu hình approval rules cho Data Query theo database/schema/table</p>
    </div>
    <button class="btn btn-primary btn-sm" on:click={openCreate}>
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
      </svg>
      Thêm Rule
    </button>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if rules.length === 0}
    <div class="text-center py-12 text-base-content/50">
      <p>Chưa có approval rule nào. Tạo rule để cấu hình flow phê duyệt cho Data Query.</p>
    </div>
  {:else}
    <div class="overflow-x-auto">
      <table class="table">
        <thead>
          <tr>
            <th>Database</th>
            <th>Schema</th>
            <th>Table</th>
            <th>Approval Chain</th>
            <th>Trạng thái</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each rules as rule}
            <tr>
              <td class="font-medium">{getDbName(rule.database_id)}</td>
              <td>
                {#if rule.schema_name}
                  {rule.schema_name}
                {:else}
                  <span class="text-base-content/40">*</span>
                {/if}
              </td>
              <td>
                {#if rule.table_name}
                  {rule.table_name}
                {:else}
                  <span class="text-base-content/40">*</span>
                {/if}
              </td>
              <td>
                <div class="flex gap-1">
                  {#each rule.levels || [] as level}
                    <span class="badge badge-sm badge-outline">
                      L{level.level_order}: {level.approver_unit}
                    </span>
                  {/each}
                </div>
              </td>
              <td>
                <span class="badge badge-sm" class:badge-success={rule.is_active} class:badge-ghost={!rule.is_active}>
                  {rule.is_active ? "Active" : "Inactive"}
                </span>
              </td>
              <td>
                <div class="flex gap-1">
                  <button class="btn btn-ghost btn-xs" on:click={() => openEdit(rule)}>Sửa</button>
                  <button class="btn btn-ghost btn-xs text-error" on:click={() => handleDelete(rule.id)}>Xóa</button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Create/Edit Modal -->
{#if showCreateModal}
  <div class="modal modal-open">
    <div class="modal-box max-w-lg">
      <h3 class="font-bold text-lg">{editingRule ? "Sửa Approval Rule" : "Tạo Approval Rule mới"}</h3>

      <div class="space-y-4 mt-4">
        <div class="form-control">
          <label class="label"><span class="label-text">Database</span></label>
          <select class="select select-bordered w-full" bind:value={newRule.database_id}>
            <option value="" disabled>Chọn database...</option>
            {#each databases as db}
              <option value={db.id}>{db.name} ({db.db_type})</option>
            {/each}
          </select>
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label"><span class="label-text">Schema (để trống = tất cả)</span></label>
            <input type="text" class="input input-bordered input-sm" placeholder="*" bind:value={newRule.schema_name} />
          </div>
          <div class="form-control">
            <label class="label"><span class="label-text">Table (để trống = tất cả)</span></label>
            <input type="text" class="input input-bordered input-sm" placeholder="*" bind:value={newRule.table_name} />
          </div>
        </div>

        <div class="form-control">
          <label class="label">
            <span class="label-text">Approval Levels</span>
            {#if newRule.levels.length < approverUnits.length}
              <button class="btn btn-xs btn-ghost" on:click={addLevel}>+ Thêm level</button>
            {/if}
          </label>
          <div class="space-y-2">
            {#each newRule.levels as level, i}
              <div class="flex items-center gap-2 bg-base-200 p-2 rounded">
                <span class="text-sm font-mono w-8">L{i + 1}</span>
                <select class="select select-bordered select-sm flex-1" bind:value={level.approver_unit}>
                  {#each approverUnits as unit}
                    <option value={unit}>{unit}</option>
                  {/each}
                </select>
                <label class="label cursor-pointer gap-1">
                  <input type="checkbox" class="checkbox checkbox-xs" bind:checked={level.required} />
                  <span class="label-text text-xs">Bắt buộc</span>
                </label>
                {#if newRule.levels.length > 1}
                  <button class="btn btn-ghost btn-xs btn-circle" on:click={() => removeLevel(i)}>✕</button>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" on:click={() => (showCreateModal = false)}>Hủy</button>
        <button class="btn btn-primary" on:click={handleSave} disabled={saving}>
          {#if saving}<span class="loading loading-spinner loading-xs"></span>{/if}
          {editingRule ? "Cập nhật" : "Tạo"}
        </button>
      </div>
    </div>
    <div class="modal-backdrop" on:click={() => (showCreateModal = false)} on:keydown={() => {}} role="presentation"></div>
  </div>
{/if}
