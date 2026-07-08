<script lang="ts">
  import { AlertCircle, RefreshCw } from "@lucide/svelte";
  import { rebuildIndex } from "$lib/ipc";

  let { onrebuild }: { onrebuild: () => void } = $props();
  let busy = $state(false);

  async function rebuild() {
    busy = true;
    try {
      await rebuildIndex();
      onrebuild();
    } finally {
      busy = false;
    }
  }
</script>

<aside class="banner" role="alert">
  <div class="msg">
    <AlertCircle size={16} />
    <span><strong>Index out of sync</strong> with disk. Rebuild recommended.</span>
  </div>
  <button onclick={rebuild} disabled={busy}>
    <RefreshCw size={14} class={busy ? "animate-spin" : ""} />
    {busy ? "Rebuilding…" : "Rebuild"}
  </button>
</aside>

<style>
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.625rem 0.875rem;
    margin: 0.75rem 1.25rem 0;
    border: 1px solid var(--color-destructive);
    background: var(--color-destructive-subtle);
    color: var(--color-foreground);
    border-radius: var(--radius-md);
    font-size: 0.875rem;
  }
  .msg {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    color: var(--color-destructive);
  }
  .msg strong {
    color: var(--color-destructive);
  }
  button {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--color-destructive);
    color: var(--color-destructive-foreground);
    border: 0;
    padding: 0.375rem 0.75rem;
    border-radius: var(--radius-sm);
    font-size: 0.8125rem;
    font-weight: 500;
  }
  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
