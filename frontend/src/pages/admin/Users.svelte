<script>
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import api from "../../api/index.js";
  import { authStore } from "../../stores/auth.js";
  import { toastStore } from "../../stores/toast.js";
  import ConfirmModal from "../../components/ConfirmModal.svelte";
  import Skeleton from "../../components/Skeleton.svelte";

  let users = [];
  let databases = [];
  let loading = true;
  let error = null;

  // Permission modal
  let selectedUser = null;
  let userPermissions = [];
  let grantDbId = "";

  // Role management
  let editRoles = { admin: false, maker: false, checker: false };
  let savingRoles = false;

  // Activate/Deactivate
  let deactivateTarget = null;
  let statusUpdating = false;

  // Confirm modal state
  let confirmOpen = false;
  let confirmAction = null;
  let confirmTitle = "";
  let confirmMessage = "";
  let confirmVariant = "primary";
  let confirmLoading = false;

  onMount(async () => {
    try {
      [users, databases] = await Promise.all([api.users.list(), api.databases.list()]);
    } catch (e) {
      error = e.error || "Không thể tải dữ liệu.";
    } finally {
      loading = false;
    }
  });

  /**
   * Hiển thị panel quyền và roles cho user được chọn.
   * @param {Object} user - User object cần hiển thị thông tin quyền
   */
  async function showPermissions(user) {
    selectedUser = user;
    // Initialize role checkboxes from user's roles
    editRoles = {
      admin: (user.roles || []).includes("ADMIN"),
      maker: (user.roles || []).includes("MAKER"),
      checker: (user.roles || []).includes("CHECKER"),
    };
    try {
      userPermissions = await api.users.getPermissions(user.id);
    } catch (e) {
      userPermissions = [];
    }
  }

  /**
   * Lưu thay đổi roles cho user đang được chọn.
   * Gửi request cập nhật roles lên API và refresh lại danh sách users.
   */
  async function handleSaveRoles() {
    if (!selectedUser) return;
    savingRoles = true;
    try {
      const roles = [];
      if (editRoles.admin) roles.push("ADMIN");
      if (editRoles.maker) roles.push("MAKER");
      if (editRoles.checker) roles.push("CHECKER");

      await api.users.update(selectedUser.id, { roles });

      // Refresh user list
      users = await api.users.list();
      // Update selectedUser reference
      selectedUser = users.find((u) => u.id === selectedUser.id) || null;
      toastStore.success("Cập nhật roles thành công!");
    } catch (e) {
      toastStore.error(e.error || "Lỗi cập nhật roles.");
    } finally {
      savingRoles = false;
    }
  }

  /**
   * Cấp quyền truy cập database cho user đang được chọn.
   * Sử dụng grantDbId để xác định database cần cấp quyền.
   */
  async function handleGrant() {
    if (!grantDbId || !selectedUser) return;
    try {
      await api.users.grantPermission(selectedUser.id, grantDbId);
      grantDbId = "";
      toastStore.success("Đã cấp quyền.");
      showPermissions(selectedUser);
    } catch (e) {
      toastStore.error(e.error || "Lỗi cấp quyền.");
    }
  }

  /**
   * Mở confirm modal để xác nhận thu hồi quyền database.
   * @param {string} databaseId - ID của database cần thu hồi quyền
   */
  async function handleRevoke(databaseId) {
    confirmTitle = "Thu hồi quyền";
    confirmMessage = "Bạn chắc chắn muốn thu hồi quyền database này?";
    confirmVariant = "warning";
    confirmAction = () => doRevoke(databaseId);
    confirmOpen = true;
  }

  /**
   * Thực hiện thu hồi quyền database sau khi user xác nhận.
   * @param {string} databaseId - ID của database cần thu hồi quyền
   */
  async function doRevoke(databaseId) {
    confirmLoading = true;
    try {
      await api.users.revokePermission(selectedUser.id, databaseId);
      confirmOpen = false;
      toastStore.success("Đã thu hồi quyền.");
      showPermissions(selectedUser);
    } catch (e) {
      toastStore.error(e.error || "Lỗi thu hồi.");
    } finally {
      confirmLoading = false;
    }
  }

  // --- Activate / Deactivate ---

  /**
   * Xử lý toggle trạng thái active/inactive của user.
   * Kiểm tra self-deactivation, hiển thị confirm dialog nếu deactivate.
   * @param {Object} user - User object cần thay đổi trạng thái
   * @param {Event} event - DOM event (để stopPropagation)
   */
  function handleToggleStatus(user, event) {
    event.stopPropagation();
    const currentUser = get(authStore)?.user;

    if (user.is_active && currentUser && currentUser.id === user.id) {
      toastStore.error("Không thể vô hiệu hóa tài khoản của chính bạn");
      return;
    }

    if (user.is_active) {
      // Deactivate → show confirmation
      confirmTitle = "Vô hiệu hóa user";
      confirmMessage = `Bạn có chắc muốn vô hiệu hóa tài khoản của ${user.display_name} (${user.email})?`;
      confirmVariant = "warning";
      confirmAction = () => doStatusUpdate(user, false);
      confirmOpen = true;
    } else {
      doStatusUpdate(user, true);
    }
  }

  /**
   * Thực hiện cập nhật trạng thái active/inactive cho user.
   * @param {Object} user - User object cần cập nhật
   * @param {boolean} activate - true để kích hoạt, false để vô hiệu hóa
   */
  async function doStatusUpdate(user, activate) {
    if (confirmOpen) confirmLoading = true;
    statusUpdating = true;
    try {
      await api.users.update(user.id, { is_active: activate });
      users = await api.users.list();
      if (selectedUser && selectedUser.id === user.id) {
        selectedUser = users.find((u) => u.id === user.id) || null;
      }
      confirmOpen = false;
      toastStore.success(activate ? "Đã kích hoạt user." : "Đã vô hiệu hóa user.");
    } catch (e) {
      toastStore.error(e.error || "Lỗi cập nhật trạng thái.");
    } finally {
      statusUpdating = false;
      confirmLoading = false;
    }
  }

  /**
   * Callback khi user nhấn xác nhận trong ConfirmModal.
   * Thực thi action đã được gán trước đó (revoke hoặc deactivate).
   */
  function onConfirm() {
    if (confirmAction) confirmAction();
  }

  /**
   * Callback khi user hủy ConfirmModal. Reset trạng thái modal.
   */
  function onCancelModal() {
    confirmOpen = false;
    confirmAction = null;
    confirmLoading = false;
  }
</script>

<div class="p-6">
  <div class="mb-6">
    <h2 class="text-2xl font-bold">Quản lý Users</h2>
    <p class="text-sm text-base-content/60 mt-1">Phân quyền role và database cho từng user</p>
  </div>

  <!-- Notification toast đã chuyển sang global toastStore -->

  {#if error}
    <div class="alert alert-error mb-4">
      <span>{error}</span>
      <button class="btn btn-sm btn-ghost" on:click={() => location.reload()}>Thử lại</button>
    </div>
  {/if}

  {#if loading}
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <div class="space-y-2">
        {#each Array(5) as _}
          <Skeleton type="card" />
        {/each}
      </div>
      <Skeleton type="card" />
    </div>
  {:else}
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- User list -->
      <div>
        <h3 class="text-lg font-semibold mb-3">Danh sách Users</h3>
        <div class="space-y-2">
          {#each users as user}
            <button
              class="card bg-base-100 shadow w-full text-left hover:shadow-md transition-shadow"
              class:ring-2={selectedUser?.id === user.id}
              class:ring-primary={selectedUser?.id === user.id}
              on:click={() => showPermissions(user)}
            >
              <div class="card-body py-3 px-4">
                <div class="flex justify-between items-center">
                  <div>
                    <p class="font-medium">{user.display_name}</p>
                    <p class="text-sm opacity-60">{user.email}</p>
                  </div>
                  <div class="flex items-center gap-2">
                    <!-- Role badges -->
                    {#each user.roles || [] as role}
                      <span class="badge badge-sm badge-outline">{role}</span>
                    {/each}
                    <!-- Active status badge -->
                    {#if user.is_active}
                      <span class="badge badge-success badge-xs">Active</span>
                    {:else}
                      <span class="badge badge-ghost badge-xs">Inactive</span>
                    {/if}
                    <!-- Activate/Deactivate toggle -->
                    <button
                      class="btn btn-xs {user.is_active ? 'btn-warning' : 'btn-success'}"
                      on:click={(e) => handleToggleStatus(user, e)}
                      disabled={statusUpdating}
                      title={user.is_active ? 'Vô hiệu hóa' : 'Kích hoạt'}
                    >
                      {user.is_active ? '✕' : '✓'}
                    </button>
                  </div>
                </div>
              </div>
            </button>
          {/each}
        </div>
      </div>

      <!-- Right panel -->
      <div>
        {#if selectedUser}
          <!-- Role Management Panel -->
          <div class="mb-6">
            <h3 class="text-lg font-semibold mb-3">
              Roles: {selectedUser.display_name}
            </h3>
            <div class="bg-base-200 rounded-lg p-4">
              <div class="flex flex-col gap-3">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm checkbox-primary"
                    bind:checked={editRoles.admin}
                  />
                  <span class="label-text font-medium">ADMIN</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm checkbox-primary"
                    bind:checked={editRoles.maker}
                  />
                  <span class="label-text font-medium">MAKER</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    class="checkbox checkbox-sm checkbox-primary"
                    bind:checked={editRoles.checker}
                  />
                  <span class="label-text font-medium">CHECKER</span>
                </label>
              </div>
              <button
                class="btn btn-primary btn-sm mt-4"
                on:click={handleSaveRoles}
                disabled={savingRoles}
              >
                {#if savingRoles}
                  <span class="loading loading-spinner loading-xs"></span>
                {/if}
                Lưu Roles
              </button>
            </div>
          </div>

          <!-- Permissions panel -->
          <h3 class="text-lg font-semibold mb-3">
            Quyền Database: {selectedUser.display_name}
          </h3>

          <!-- Grant form -->
          <div class="flex gap-2 mb-4">
            <select class="select select-bordered select-sm flex-1" bind:value={grantDbId}>
              <option value="" disabled>Chọn database...</option>
              {#each databases as db}
                <option value={db.id}>{db.name}</option>
              {/each}
            </select>
            <button class="btn btn-primary btn-sm" on:click={handleGrant} disabled={!grantDbId}>
              Cấp quyền
            </button>
          </div>

          <!-- Current permissions -->
          {#if userPermissions.length === 0}
            <p class="text-sm opacity-60">Chưa có quyền nào.</p>
          {:else}
            <div class="space-y-2">
              {#each userPermissions as perm}
                <div class="flex justify-between items-center bg-base-200 rounded-lg px-3 py-2">
                  <span class="text-sm">
                    {databases.find((d) => d.id === perm.database_id)?.name || perm.database_id}
                  </span>
                  <button class="btn btn-xs btn-ghost text-error" on:click={() => handleRevoke(perm.database_id)}>
                    Thu hồi
                  </button>
                </div>
              {/each}
            </div>
          {/if}
        {:else}
          <div class="text-center py-10 opacity-50">Chọn user để xem quyền</div>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Confirm Modal (replaces old deactivate-modal) -->
  <ConfirmModal
    bind:open={confirmOpen}
    title={confirmTitle}
    message={confirmMessage}
    variant={confirmVariant}
    loading={confirmLoading}
    on:confirm={onConfirm}
    on:cancel={onCancelModal}
  />
</div>
