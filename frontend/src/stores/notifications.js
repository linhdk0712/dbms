import { writable } from "svelte/store";

/**
 * WebSocket notification store.
 * DISABLED — WebSocket tính năng đã tắt.
 */
function createNotificationStore() {
  const { subscribe, update } = writable([]);

  function connect() {
    // WebSocket disabled — no-op
  }

  function disconnect() {
    // WebSocket disabled — no-op
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
