<script lang="ts">
  import { notes } from "$lib/stores/notes.svelte";
  import { onMount } from "svelte";

  let query = $state("");

  onMount(() => notes.refresh());

  async function create() {
    const t = prompt("Note title?");
    if (!t) return;
    await notes.create(t);
  }

  const filtered = $derived(
    query
      ? notes.list.filter(n => n.title.toLowerCase().includes(query.toLowerCase()))
      : notes.list
  );
</script>

<div>
  <header>
    <input bind:value={query} placeholder="Search…" />
    <button onclick={create}>+ New</button>
  </header>
  <ul>
    {#each filtered as n}
      <li><a href={`/notes/${n.id}`}>{n.title}</a></li>
    {/each}
  </ul>
</div>

<style>
  header { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  input { flex: 1; }
  ul { list-style: none; padding: 0; }
  li { padding: 0.25rem 0; }
</style>
