<script lang="ts">
  import Sidebar from "$lib/components/Sidebar.svelte";
  let { children } = $props();
</script>

<div class="app">
  <Sidebar />
  <section class="content">
    {@render children()}
  </section>
</div>

<style>
  .app { display: flex; min-height: 100vh; }
  .content { flex: 1; padding: 1.5rem; }
</style>
