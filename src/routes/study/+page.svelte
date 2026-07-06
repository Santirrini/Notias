<script lang="ts">
  import { page } from '$app/state';
  import SRSReviewer from '$lib/components/SRSReviewer.svelte';
  import QuizRunner from '$lib/components/QuizRunner.svelte';
  import WeeklyPlan from '$lib/components/WeeklyPlan.svelte';

  let tab = $state<'today' | 'quizzes' | 'plan'>('today');
  let selectedNoteIds = $state<string[]>([]);

  // ponytail: read selectedNote from query string for the Today tab to enable
  // "Generate cards" — passing via URL keeps the SRS component reusable.
  $effect(() => {
    selectedNoteIds = page.url.searchParams.getAll('note');
  });
</script>

<main>
  <nav class="tabs">
    <button class:active={tab === 'today'} onclick={() => tab = 'today'}>Today</button>
    <button class:active={tab === 'quizzes'} onclick={() => tab = 'quizzes'}>Quizzes</button>
    <button class:active={tab === 'plan'} onclick={() => tab = 'plan'}>Plan</button>
  </nav>

  {#if tab === 'today'}
    <SRSReviewer noteId={selectedNoteIds[0]} />
  {:else if tab === 'quizzes'}
    <QuizRunner noteIds={selectedNoteIds} />
  {:else}
    <WeeklyPlan />
  {/if}
</main>

<style>
  main { padding: 1rem; }
  .tabs { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
  .tabs button { padding: 0.5rem 1rem; }
  .tabs button.active { background: var(--accent, #4af); color: white; }
</style>
