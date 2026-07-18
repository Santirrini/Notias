<script lang="ts">
  import { study } from '$lib/stores/study.svelte';
  import { Check, X, Plus } from '@lucide/svelte';
  import { m, localizeError } from "$lib/i18n";

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
  <h2>{m.study_quizzes_title()}</h2>

  {#if !study.quiz}
    <div class="setup">
      <p>{m.study_quizzes_setup_intro()}</p>
      <div class="id-row">
        <input
          bind:value={noteIdInput}
          placeholder={m.study_quizzes_id_placeholder()}
          onkeydown={(e) => { if (e.key === 'Enter') addId(); }}
        />
        <button onclick={addId}>
          <Plus size={14} /> {m.study_quizzes_add_button()}
        </button>
      </div>
      {#if noteIds.length > 0}
        <ul>
          {#each noteIds as id}
            <li>{id} <button onclick={() => removeId(id)} aria-label={m.study_quizzes_remove_aria()}><X size={14} /></button></li>
          {/each}
        </ul>
      {/if}
      <button onclick={start} disabled={!noteIds.length || study.busy}>
        {study.busy ? m.study_quizzes_generating() : m.study_quizzes_generate()}
      </button>
    </div>
    {#if study.error}
      <p class="err">{localizeError({ message: study.error })}</p>
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
              placeholder={m.study_quizzes_your_answer()}
              oninput={(e) => study.setAnswer(i, (e.target as HTMLInputElement).value)}
            />
          {/if}
        </div>
      {/each}
      <button onclick={() => study.submit()} disabled={study.busy}>
        {study.busy ? m.study_quizzes_grading() : m.study_quizzes_submit()}
      </button>
    </div>
  {:else}
    <div class="result">
      <h3>{m.study_quizzes_score({ score: study.result.score, total: study.result.total })}</h3>
      {#each study.result.perQuestion as r}
        <div class={r.correct ? 'ok' : 'no'}>
          <strong>{m.study_quizzes_question({ index: r.index + 1 })}</strong>:
          {#if r.correct}<Check size={14} class="text-success" />
          {:else}<X size={14} class="text-destructive" />{/if}
          {m.study_quizzes_expected()} <em>{r.expected}</em>, {m.study_quizzes_you_wrote()} <em>{r.given || m.study_quizzes_empty_answer()}</em>
          <p>{r.rationale}</p>
        </div>
      {/each}
      <button onclick={() => study.reset()}>{m.study_quizzes_new()}</button>
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
