<script lang="ts">
  import { page } from "$app/stores";
  import { notes } from "$lib/stores/notes.svelte";
  import Milkdown from "$lib/editor/Milkdown.svelte";
  import { onMount } from "svelte";

  const id = $derived($page.params.id);
  let title = $state("");
  let body = $state("");
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    if (!id) return;
    await notes.load(id);
    if (notes.current) {
      title = notes.current.title;
      body = notes.current.body;
    }
  });

  function scheduleSave() {
    if (!id) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => notes.save(id, { title, body }), 1500);
  }
</script>

{#if notes.current}
  <input bind:value={title} oninput={scheduleSave} placeholder="Title" />
  <Milkdown initial={body} onChange={(v) => { body = v; scheduleSave(); }} />
{:else}
  <p>Loading…</p>
{/if}