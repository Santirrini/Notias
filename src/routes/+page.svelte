<script lang="ts">
  import { onMount } from "svelte";
  import { ping } from "$lib/ipc";

  let result = $state<string | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      result = await ping();
    } catch (e) {
      error = (e as { message: string }).message;
    }
  });
</script>

<main>
  <h1>Notias</h1>
  {#if error}<p class="err">{error}</p>
  {:else if result}<p>Backend says: <code>{result}</code></p>
  {:else}<p>Loading…</p>{/if}
</main>

<style>
  main { padding: 2rem; font-family: ui-sans-serif, system-ui, sans-serif; }
  .err { color: #c00; }
  code { background: #f4f4f4; padding: 0 0.3em; border-radius: 3px; }
</style>
