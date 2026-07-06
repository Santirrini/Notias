<script lang="ts">
  import { onMount } from 'svelte';
  import { srs } from '$lib/stores/srs.svelte';
  import { generateCards } from '$lib/ipc';
  import type { DraftCard } from '$lib/types';

  let { noteId }: { noteId?: string } = $props();
  let generating = $state(false);
  let drafts = $state<DraftCard[]>([]);

  onMount(() => srs.loadQueue());

  async function gen() {
    if (!noteId) return;
    generating = true;
    try {
      drafts = await generateCards(noteId, 5);
    } catch (e) {
      srs.error = (e as { message: string }).message;
    } finally {
      generating = false;
    }
  }

  async function saveDraft(d: DraftCard, idx: number) {
    if (!noteId) return;
    try {
      await srs.saveBatch(noteId, [d]);
      drafts.splice(idx, 1);
    } catch (e) {
      srs.error = (e as { message: string }).message;
    }
  }
</script>

<section>
  <header>
    <h2>Today</h2>
    <span>{srs.queue.length} cards due</span>
    {#if srs.busy}<small> · loading…</small>{/if}
    {#if noteId}
      <button onclick={gen} disabled={generating}>+ Generate 5 cards</button>
    {/if}
  </header>

  {#if srs.error}
    <p class="err">{srs.error}</p>
  {/if}

  {#if drafts.length > 0}
    <article class="drafts">
      <h3>Drafts</h3>
      {#each drafts as d, i}
        <div>
          <strong>{d.front}</strong> → {d.back}
          <button onclick={() => saveDraft(d, i)}>Save</button>
        </div>
      {/each}
    </article>
  {/if}

  {#if srs.current}
    <div class="card">
      {#if srs.showBack}
        <p class="front">{srs.current.front}</p>
        <hr>
        <p class="back">{srs.current.back}</p>
      {:else}
        <p class="front">{srs.current.front}</p>
        <button onclick={() => srs.reveal()}>Show answer</button>
      {/if}
    </div>

    {#if srs.showBack}
      <div class="grade">
        <button onclick={() => srs.grade(1)}>Again</button>
        <button onclick={() => srs.grade(3)}>Hard</button>
        <button onclick={() => srs.grade(4)}>Good</button>
        <button onclick={() => srs.grade(5)}>Easy</button>
      </div>
    {:else}
      <button onclick={() => srs.skip()}>Skip</button>
    {/if}
  {:else}
    <p class="empty">Nothing due. 🎉</p>
  {/if}
</section>

<style>
  section { padding: 1rem; }
  header { display: flex; gap: 1rem; align-items: baseline; }
  .card { padding: 2rem; background: var(--bg-elevated, #1a1a1a); border-radius: 0.5rem; margin: 1rem 0; text-align: center; min-height: 8rem; }
  .front { font-size: 1.15rem; }
  .back { opacity: 0.85; }
  .grade { display: flex; gap: 0.5rem; justify-content: center; }
  .grade button { padding: 0.5rem 1.25rem; font-size: 1rem; }
  .drafts { margin: 1rem 0; padding: 0.5rem; background: #111; border-radius: 0.5rem; }
  .drafts div { display: flex; gap: 0.5rem; align-items: center; padding: 0.4rem 0; }
  .drafts button { margin-left: auto; }
  .empty { opacity: 0.7; }
  .err { color: #f44; }
</style>
