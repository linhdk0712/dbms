<script>
  import { authStore } from "../stores/auth.js";
  import { push } from "svelte-spa-router";
  import { toastStore } from "../stores/toast.js";
  import api from "../api/index.js";

  // Theme: "light" | "dark" | "system"
  let themeMode = "system";
  let isDark = false;

  // Khôi phục theme từ localStorage
  if (typeof localStorage !== "undefined") {
    themeMode = localStorage.getItem("secs_theme_mode") || "system";
    applyTheme();
  }

  function applyTheme() {
    if (themeMode === "system") {
      isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    } else {
      isDark = themeMode === "dark";
    }
    document.documentElement.setAttribute("data-theme", isDark ? "dark" : "light");
  }

  function cycleTheme() {
    // Cycle: light → dark → system → light ...
    if (themeMode === "light") themeMode = "dark";
    else if (themeMode === "dark") themeMode = "system";
    else themeMode = "light";

    localStorage.setItem("secs_theme_mode", themeMode);
    applyTheme();
  }

  // Listen for OS theme changes when in system mode
  if (typeof window !== "undefined") {
    window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
      if (themeMode === "system") applyTheme();
    });
  }

  async function handleLogout() {
    try {
      await api.auth.logout();
    } catch (_) {
      // Ignore errors — logout client-side anyway
    }
    authStore.logout();
    toastStore.info("Đã đăng xuất.");
    push("/login");
  }
</script>

<div class="navbar bg-base-100 border-b border-base-300 sticky top-0 z-40">
  <div class="flex-1">
    <a href="#/" class="btn btn-ghost gap-2 text-xl font-bold tracking-tight">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-primary" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4" />
      </svg>
      <span>SECS</span>
    </a>
  </div>

  {#if $authStore}
    <div class="flex-none gap-1">
      <!-- Theme toggle (3 states: light/dark/system) -->
      <div class="tooltip tooltip-bottom" data-tip="Theme: {themeMode}">
        <button class="btn btn-ghost btn-circle btn-sm" on:click={cycleTheme} aria-label="Chuyển theme ({themeMode})">
          {#if themeMode === "light"}
            <svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
            </svg>
          {:else if themeMode === "dark"}
            <svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
            </svg>
          {:else}
            <svg class="w-5 h-5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
            </svg>
          {/if}
        </button>
      </div>

      <!-- User dropdown -->
      <div class="dropdown dropdown-end">
        <button class="btn btn-ghost btn-circle avatar placeholder" aria-label="Menu người dùng">
          <div class="bg-primary text-primary-content rounded-full w-9">
            <span class="text-sm font-medium">{$authStore.user?.display_name?.charAt(0)?.toUpperCase() || "U"}</span>
          </div>
        </button>
        <!-- svelte-ignore a11y-no-noninteractive-tabindex -->
        <ul tabindex="0" class="dropdown-content menu bg-base-100 rounded-box z-50 w-56 p-2 shadow-lg border border-base-300">
          <li class="menu-title px-3 pt-1">
            <span class="text-sm font-semibold">{$authStore.user?.display_name || "User"}</span>
          </li>
          <li class="px-3 pb-1">
            <span class="text-xs opacity-60 pointer-events-none">{$authStore.user?.email}</span>
          </li>
          {#if $authStore.user?.roles}
            <li class="px-3 pb-2">
              <span class="flex gap-1 pointer-events-none">
                {#each $authStore.user.roles as role}
                  <span class="badge badge-xs badge-outline">{role}</span>
                {/each}
              </span>
            </li>
          {/if}
          <div class="divider my-0"></div>
          <li>
            <button on:click={handleLogout} class="text-error">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
              </svg>
              Đăng xuất
            </button>
          </li>
        </ul>
      </div>
    </div>
  {/if}
</div>
