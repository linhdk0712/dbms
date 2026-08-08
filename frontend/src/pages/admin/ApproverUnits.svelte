<script>
  import { onMount } from "svelte";
  import api from "../../api/index.js";
  import { toastStore } from "../../stores/toast.js";

  let units = [];
  let loading = true;
  let showModal = false;
  let saving = false;
  let editingUnit = null;

  // Form
  let formName = "";
  let formDescription = "";

  onMount(async () => {
    await loadUnits();
  });

  /**
   * Fetches the list of approver units from the API and updates local state.
   * @returns {Promise<void>}
   */
  async function loadUnits() {
    loading = true;
    try {
      units = await api.approverUnits.list();
    } catch (e) {
      toastStore.error("Không thể tải danh sách approver units.");
      units = [];
    } finally {
      loading = false;
    }
  }

  /**
   * Opens the modal in "create" mode, resetting form fields.
   */
  function openCreate() {
    editingUnit = null;
    formName = "";
    formDescription = "";
    showModal = true;
  }

  /**
   * Opens the modal in "edit" mode, pre-filling form fields with the unit's data.
   * @param {object} unit - The approver unit to edit.
   */
  function openEdit(unit) {
    editingUnit = unit;
    formName = unit.name;
    formDescription = unit.description || "";
    showModal = true;
  }

  /**
   * Validates form input and creates or updates an approver unit via the API.
   * Closes the modal and reloads the list on success.
   * @returns {Promise<void>}
   */
  async function handleSave() {
    if (!formName.trim()) {
      toastStore.warning("Tên unit không được để trống.");
      return;
    }
    saving = true;
    try {
      if (editingUnit) {
        await api.approverUnits.update(editingUnit.id, {
          name: formName.trim(),
          description: formDescription.trim() || null,
        });
        toastStore.success("Đã cập nhật unit.");
      } else {
        await api.approverUnits.create(formName.trim(), formDescription.trim() || null);
        toastStore.success("Đã tạo unit mới.");
      }
      showModal = false;
      await loadUnits();
    } catch (e) {
      toastStore.error(e.error || "Không thể lưu unit.");
    } finally {
      saving = false;
    }
  }

  /**
   * Toggles the is_active status of an approver unit.
   * @param {object} unit - The approver unit to activate or deactivate.
   * @returns {Promise<void>}
   */
  async function handleToggleActive(unit) {
    try {
      await api.approverUnits.update(unit.id, { is_active: !unit.is_active });
      await loadUnits();
      toastStore.success(unit.is_active ? "Đã vô hiệu hóa unit." : "Đã kích hoạt unit.");
    } catch (e) {
      toastStore.error(e.error || "Lỗi cập nhật trạng thái.");
    }
  }

  /**
   * Deletes an approver unit after user confirmation.
   * @param {object} unit - The approver unit to delete.
   * @returns {Promise<void>}
   */
  async function handleDelete(unit) {
    if (!confirm(`Xóa unit "${unit.name}"? Thao tác này không thể hoàn tác.`)) return;
    try {
      await api.approverUnits.delete(unit.id);
      toastStore.success("Đã xóa unit.");
      await loadUnits();
    } catch (e) {
      toastStore.error(e.error || "Không thể xóa unit (có thể đang được sử dụng).");
    }
  }
</script>

<div class="p-6 max-w-3xl mx-auto">
  <div class="flex items-center justify-between mb-6">
    <div>
      <h2 class="text-2xl font-bold">Quản lý Approver Units</h2>
      <p class="text-sm text-base-content/60 mt-0.5">Định nghĩa các đơn vị phê duyệt trong hệ thống</p>
    </div>
    <button class="btn btn-primary btn-sm" on:click={openCreate}>
      <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
      </svg>
      Thêm Unit
    </button>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg"></span>
    </div>
  {:else if units.length === 0}
    <div class="text-center py-12 text-base-content/50">
      <p>Chưa có approver unit nào.</p>
      <button class="btn btn-primary btn-sm mt-4" on:click={openCreate}>Tạo unit đầu tiên</button>
    </div>
  {:else}
    <div class="space-y-3">
      {#each units as unit}
        <div class="card bg-base-100 shadow">
          <div class="card-body py-4 px-5">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div>
                  <p class="font-semibold">{unit.name}</p>
                  {#if unit.description}
                    <p class="text-sm text-base-content/60">{unit.description}</p>
                  {/if}
                </div>
                <span class="badge badge-sm" class:badge-success={unit.is_active} class:badge-ghost={!unit.is_active}>
                  {unit.is_active ? "Active" : "Inactive"}
                </span>
              </div>
              <div class="flex items-center gap-1">
                <button class="btn btn-ghost btn-xs" on:click={() => openEdit(unit)}>
                  Sửa
                </button>
                <button
                  class="btn btn-ghost btn-xs"
                  class:text-warning={unit.is_active}
                  class:text-success={!unit.is_active}
                  on:click={() => handleToggleActive(unit)}
                >
                  {unit.is_active ? "Tắt" : "Bật"}
                </button>
                <button class="btn btn-ghost btn-xs text-error" on:click={() => handleDelete(unit)}>
                  Xóa
                </button>
              </div>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Create/Edit Modal -->
{#if showModal}
  <div class="modal modal-open">
    <div class="modal-box max-w-sm">
      <h3 class="font-bold text-lg">{editingUnit ? "Sửa Approver Unit" : "Tạo Approver Unit mới"}</h3>

      <div class="space-y-4 mt-4">
        <div class="form-control">
          <label class="label" for="unit-name"><span class="label-text">Tên Unit</span></label>
          <input
            id="unit-name"
            type="text"
            class="input input-bordered w-full"
            placeholder="VD: ITO, Security Team, DBA..."
            bind:value={formName}
          />
        </div>

        <div class="form-control">
          <label class="label" for="unit-desc"><span class="label-text">Mô tả (tùy chọn)</span></label>
          <input
            id="unit-desc"
            type="text"
            class="input input-bordered w-full"
            placeholder="VD: IT Operations Team"
            bind:value={formDescription}
          />
        </div>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" on:click={() => (showModal = false)}>Hủy</button>
        <button class="btn btn-primary" on:click={handleSave} disabled={saving}>
          {#if saving}<span class="loading loading-spinner loading-xs"></span>{/if}
          {editingUnit ? "Cập nhật" : "Tạo"}
        </button>
      </div>
    </div>
    <div class="modal-backdrop" on:click={() => (showModal = false)} on:keydown={() => {}} role="presentation"></div>
  </div>
{/if}
