<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { notes } from "$lib/stores/notes.svelte";
  import NoteCanvas from "$lib/components/NoteCanvas.svelte";

  const id = $derived(page.params.id ?? "");

  onMount(async () => {
    if (!id) return;
    await notes.load(id);
  });
</script>

{#if notes.current && id === notes.current.id}
  <NoteCanvas
    {id}
    initialTitle={notes.current.title}
    initialBody={notes.current.body}
  />
{:else}
  <p class="hint">Loading…</p>
{/if}

<style>
  .hint {
    padding: 2rem;
    color: var(--color-muted-foreground);
  }
</style>
