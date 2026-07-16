<script>
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import api from "../../api/index.js";
  import { authStore } from "../../stores/auth.js";

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

  // Notification
  let notification = null;
  let notificationTimeout = null;

  function showNotification(message, type = "success") {
    if (notificationTimeout) clearTimeout(notificationTimeout);
    notification = { message, type };
    notificationTimeout = setTimeout(() => {
      notification = null;
    }, 3000);
  }

  onMount(async () => {
    try {
      [users, databases] = await Promise.all([api.users.list(), api.databases.list()]);
    } catch (e) {
      error = e.error || "Không thể tải dữ liệu.";
    } finally {
      loading = false;
    }
  });

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
      showNotification("Cập nhật roles thành công!");
    } catch (e) {
      showNotification(e.error || "Lỗi cập nhật roles.", "error");
    } finally {
      savingRoles = false;
    }
  }

  async function handleGrant() {
    if (!grantDbId || !selectedUser) return;
    try {
      await api.users.grantPermission(selectedUser.id, grantDbId);
      grantDbId = "";
      showPermissions(selectedUser);
    } catch (e) {
      alert(e.error || "Lỗi cấp quyền.");
    }
  }

  async function handleRevoke(databaseId) {
    if (!confirm("Thu hồi quyền này?")) return;
    try {
      await api.users.revokePermission(selectedUser.id, databaseId);
      showPermissions(selectedUser);
    } catch (e) {
      alert(e.error || "Lỗi thu hồi.");
    }
  }

  // --- Activate / Deactivate ---

  function handleToggleStatus(user, event) {
    event.stopPropagation();
    const currentUser = get(authStore)?.user;

    // Self-deactivation check
    if (user.is_active && currentUser && currentUser.id === user.id) {
      showNotification("Không thể vô hiệu hóa tài khoản của chính bạn", "error");
      return;
    }

    if (user.is_active) {
      // Deactivate → show confirmation dialog
      deactivateTarget = user;
      document.getElementById("deactivate-modal")?.showModal();
    } else {
      // Activate → do immediately
      doStatusUpdate(user, true);
    }
  }

  async function confirmDeactivate() {
    if (!deactivateTarget) return;
    document.getElementById("deactivate-modal")?.close();
    await doStatusUpdate(deactivateTarget, false);
    deactivateTarget = null;
  }

  function cancelDeactivate() {
    document.getElementById("deactivate-modal")?.close();
    deactivateTarget = null;
  }

  async function doStatusUpdate(user, activate) {
    statusUpdating = true;
    try {
      await api.users.update(user.id, { is_active: activate });
      // Refresh user list
      users = await api.users.list();
      // Update selectedUser reference if same user
      if (selectedUser && selectedUser.id === user.id) {
        selectedUser = users.find((u) => u.id === user.id) || null;
      }
      showNotification(activate ? "Đã kích hoạt user." : "Đã vô hiệu hóa user.");
    } catch (e) {
      showNotification(e.error || "Lỗi cập nhật trạng thái.", "error");
    } finally {
      statusUpdating = false;
    }
  }
</script>

<div class="p-6">
  <h2 class="text-2xl font-bold mb-6">Quản lý Users</h2>

  <!-- Notification toast -->
  {#if notification}
    <div
      class="toast toast-top toast-end z-50"
    >
      <div class="alert {notification.type === 'error' ? 'alert-error' : 'alert-success'}">
        <span>{notification.message}</span>
      </div>
    </div>
  {/if}

  {#if error}
    <div class="alert alert-error mb-4"><span>{error}</span></div>
  {/if}

  {#if loading}
    <div class="flex justify-center py-10"><span class="loading loading-spinner"></span></div>
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

  <!-- Deactivate Confirmation Modal -->
  <dialog id="deactivate-modal" class="modal">
    <div class="modal-box">
      <h3 class="font-bold text-lg">Xác nhận vô hiệu hóa</h3>
      {#if deactivateTarget}
        <p class="py-4">
          Bạn có chắc muốn vô hiệu hóa tài khoản của <strong>{deactivateTarget.display_name}</strong> ({deactivateTarget.email})?
        </p>
      {/if}
      <div class="modal-action">
        <button class="btn btn-ghost" on:click={cancelDeactivate}>Hủy</button>
        <button class="btn btn-warning" on:click={confirmDeactivate}>Vô hiệu hóa</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button on:click={cancelDeactivate}>close</button>
    </form>
  </dialog>
</div>
