<script>
  import { authStore } from "../stores/auth.js";

  $: roles = $authStore?.user?.roles || [];
  $: isMaker = roles.includes("MAKER");
  $: isChecker = roles.includes("CHECKER");
  $: isAdmin = roles.includes("ADMIN");
</script>

<aside class="w-64 bg-base-100 border-r border-base-200 min-h-screen p-4 hidden lg:block">
  <nav>
    <ul class="menu">
      <li><a href="#/">Dashboard</a></li>

      {#if isMaker}
        <li class="menu-title mt-4"><span>Maker</span></li>
        <li><a href="#/requests/new">Tạo request mới</a></li>
        <li><a href="#/requests">Requests của tôi</a></li>
        <li><a href="#/change-requests">Change Requests</a></li>
      {/if}

      {#if isChecker}
        <li class="menu-title mt-4"><span>Checker</span></li>
        <li><a href="#/approvals">Chờ duyệt</a></li>
      {/if}

      {#if isAdmin}
        <li class="menu-title mt-4"><span>Admin</span></li>
        <li><a href="#/admin/databases">Databases</a></li>
        <li><a href="#/admin/users">Users</a></li>
      {/if}
    </ul>
  </nav>
</aside>
