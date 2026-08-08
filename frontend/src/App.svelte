<script>
  import Router, { push } from "svelte-spa-router";
  import { authStore } from "./stores/auth.js";
  import { notifications } from "./stores/notifications.js";
  import Navbar from "./components/Navbar.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Toast from "./components/Toast.svelte";

  // Pages
  import Login from "./pages/Login.svelte";
  import AuthCallback from "./pages/AuthCallback.svelte";
  import Dashboard from "./pages/Dashboard.svelte";
  import NewRequest from "./pages/maker/NewRequest.svelte";
  import MyRequests from "./pages/maker/MyRequests.svelte";
  import RequestDetail from "./pages/maker/RequestDetail.svelte";
  import PendingList from "./pages/checker/PendingList.svelte";
  import ReviewDetail from "./pages/checker/ReviewDetail.svelte";
  import AdminDatabases from "./pages/admin/Databases.svelte";
  import AdminUsers from "./pages/admin/Users.svelte";
  import ChangeRequests from "./pages/maker/ChangeRequests.svelte";

  // Data Query pages
  import NewDataQuery from "./pages/data-queries/NewDataQuery.svelte";
  import MyDataQueries from "./pages/data-queries/MyDataQueries.svelte";
  import DataQueryDetail from "./pages/data-queries/DataQueryDetail.svelte";
  import DataQueryApprovals from "./pages/data-queries/DataQueryApprovals.svelte";
  import ApprovalMatrix from "./pages/admin/ApprovalMatrix.svelte";
  import ApproverUnits from "./pages/admin/ApproverUnits.svelte";

  // Manual / Documentation
  import Manual from "./pages/Manual.svelte";

  const routes = {
    "/login": Login,
    "/auth/callback": AuthCallback,
    "/": Dashboard,
    "/requests/new": NewRequest,
    "/requests": MyRequests,
    "/requests/:id": RequestDetail,
    "/change-requests": ChangeRequests,
    "/approvals": PendingList,
    "/approvals/:id": ReviewDetail,
    "/admin/databases": AdminDatabases,
    "/admin/users": AdminUsers,
    // Data Query routes
    "/data-queries/new": NewDataQuery,
    "/data-queries": MyDataQueries,
    "/data-queries/:id": DataQueryDetail,
    "/data-query-approvals": DataQueryApprovals,
    "/admin/approval-matrix": ApprovalMatrix,
    "/admin/approver-units": ApproverUnits,
    // Manual / Documentation
    "/manual": Manual,
  };

  // Auth guard — redirect to login if not authenticated
  function conditionsFailed(event) {
    push("/login");
  }

  // Connect WebSocket khi đã login
  $: if ($authStore?.token) {
    notifications.connect();
  } else {
    notifications.disconnect();
  }

  // Redirect nếu chưa login (trừ /login và /auth/callback)
  $: {
    if (typeof window !== "undefined") {
      const hash = window.location.hash || "#/";
      const path = hash.replace("#", "");
      const isPublicRoute = path === "/login" || path.startsWith("/auth/callback");
      if (!$authStore?.token && !isPublicRoute) {
        push("/login");
      }
    }
  }
</script>

{#if $authStore?.token}
  <Navbar />
  <div class="flex">
    <Sidebar />
    <main class="flex-1 min-h-screen bg-base-200">
      <Router {routes} on:conditionsFailed={conditionsFailed} />
    </main>
  </div>
{:else}
  <Router {routes} on:conditionsFailed={conditionsFailed} />
{/if}

<!-- Toast always rendered (global) -->
<Toast />
