<script lang="ts">
  import { study } from '$lib/stores/study.svelte';
  import { Check, X, Plus, Minus } from '@lucide/svelte';

  let { noteIds }: { noteIds: string[] } = $props();
  let noteIdInput = $state('');

  function addId() {
    if (!noteIdInput.trim()) return;
    noteIds = [...noteIds, noteIdInput.trim()];
    noteIdInput = '';
  }

  function removeId(id: string) {
    noteIds = noteIds.filter((n) => n !== id);
  }

  async function start() {
    if (!noteIds.length) return;
    await study.startQuiz(noteIds, 5);
  }
</script>

<section>
  <h2>Quizzes</h2>

  {#if !study.quiz}
    <div class="setup">
      <p>Paste note IDs to draw questions from:</p>
      <div class="id-row">
        <input
          bind:value={noteIdInput}
          placeholder="Note ID (ULID)"
          onkeydown={(e) => { if (e.key === 'Enter') addId(); }}
        />
        <button onclick={addId}>
          <Plus size={14} /> Note
        </button>
      </div>
      {#if noteIds.length > 0}
        <ul>
          {#each noteIds as id}
            <li>{id} <button onclick={() => removeId(id)} aria-label="Remove"><X size={14} /></button></li>
          {/each}
        </ul>
      {/if}
      <button onclick={start} disabled={!noteIds.length || study.busy}>
        {study.busy ? 'Generating…' : 'Generate quiz'}
      </button>
    </div>
    {#if study.error}
      <p class="err">{study.error}</p>
    {/if}
  {:else if !study.result}
    <div class="qs">
      {#each study.quiz.questions as q, i}
        <div class="q">
          <p><strong>{i + 1}.</strong> {q.question}</p>
          {#if q.type === 'mc'}
            {#each q.choices as c, j}
              <label>
                <input
                  type="radio"
                  name={`q${i}`}
                  value={j}
                  onchange={() => study.setAnswer(i, String(j))}
                />
                {c}
              </label>
            {/each}
          {:else}
            <input
              placeholder="Your answer"
              oninput={(e) => study.setAnswer(i, (e.target as HTMLInputElement).value)}
            />
          {/if}
        </div>
      {/each}
      <button onclick={() => study.submit()} disabled={study.busy}>
        {study.busy ? 'Grading…' : 'Submit'}
      </button>
    </div>
  {:else}
    <div class="result">
      <h3>Score: {study.result.score} / {study.result.total}</h3>
      {#each study.result.perQuestion as r}
        <div class={r.correct ? 'ok' : 'no'}>
          <strong>Q{r.index + 1}</strong>:
          {#if r.correct}<Check size={14} class="text-success" />
          {:else}<X size={14} class="text-destructive" />{/if}
          expected <em>{r.expected}</em>, you wrote <em>{r.given || '∅'}</em>
          <p>{r.rationale}</p>
        </div>
      {/each}
      <button onclick={() => study.reset()}>New quiz</button>
    </div>
  {/if}
</section>

<style>
  section { padding: 1rem; }
  .setup, .qs, .result { display: flex; flex-direction: column; gap: 0.75rem; }
  .id-row { display: flex; gap: 0.5rem; }
  .id-row input { flex: 1; padding: 0.4rem 0.6rem; }
  .ok { border-left: 3px solid #4f4; padding-left: 0.5rem; }
  .no { border-left: 3px solid #f44; padding-left: 0.5rem; }
  .err { color: #f44; }
  ul { list-style: none; padding: 0; }
  li { display: flex; gap: 0.5rem; align-items: center; padding: 0.2rem 0; }
  li button { margin-left: auto; }
  label { display: block; padding: 0.2rem 0; }
</style>
