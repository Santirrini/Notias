<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  import { onMount } from "svelte";
  import { recoveryRequired, rebuildIndex } from "$lib/ipc";

  let { children } = $props();
  let recovery = $state(false);

  onMount(async () => {
    recovery = await recoveryRequired();
  });

  async function rebuild() {
    await rebuildIndex();
    recovery = false;
  }
</script>

<div class="app">
  <Sidebar />
  <section class="content">
    {#if recovery}
      <div class="banner">
        Index out of sync with disk.
        <button onclick={rebuild}>Rebuild</button>
      </div>
    {/if}
    {@render children()}
  </section>
</div>

<style>
  .app { display: flex; min-height: 100vh; }
  .content { flex: 1; padding: 1.5rem; }
  .banner { background: #fee; border: 1px solid #c00; padding: 0.75rem; margin-bottom: 1rem; display: flex; justify-content: space-between; align-items: center; }
  .banner button { background: #c00; color: white; border: 0; padding: 0.25rem 0.75rem; border-radius: 3px; cursor: pointer; }
</style>