<script>
  /** CR number (e.g. "12345") */
  export let crNumber = "";
  /** Full ManageEngine URL */
  export let crUrl = "";
</script>

{#if crNumber}
  <a
    href={crUrl}
    target="_blank"
    rel="noopener noreferrer"
    class="badge badge-info badge-outline gap-1 hover:badge-info transition-colors"
    title="Mở CR trên ManageEngine"
  >
    <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101" />
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.172 13.828a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.102 1.101" />
    </svg>
    CR #{crNumber}
  </a>
{/if}
