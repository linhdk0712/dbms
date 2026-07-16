import { writable, get } from "svelte/store";
import { authStore } from "./auth.js";

/**
 * WebSocket notification store.
 * Auto-connect khi có token, auto-reconnect khi disconnect.
 */
function createNotificationStore() {
  const { subscribe, update } = writable([]);
  let ws = null;
  let reconnectTimeout = null;

  function connect() {
    const auth = get(authStore);
    if (!auth?.token) return;

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/ws?token=${auth.token}`;

    ws = new WebSocket(wsUrl);

    ws.onmessage = (event) => {
      try {
        const notification = JSON.parse(event.data);
        update((list) => [notification, ...list].slice(0, 50)); // Keep last 50
      } catch (e) {
        // Ignore invalid messages
      }
    };

    ws.onclose = () => {
      ws = null;
      // Auto-reconnect after 3 seconds
      reconnectTimeout = setTimeout(connect, 3000);
    };

    ws.onerror = () => {
      ws?.close();
    };
  }

  function disconnect() {
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }
    if (ws) {
      ws.close();
      ws = null;
    }
  }

  function clear() {
    update(() => []);
  }

  return {
    subscribe,
    connect,
    disconnect,
    clear,
  };
}

export const notifications = createNotificationStore();
