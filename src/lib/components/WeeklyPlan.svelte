<script lang="ts">
  import { onMount } from 'svelte';
  import { plan } from '$lib/stores/study.svelte';
  import { m, i18n, localizeError } from "$lib/i18n";

  onMount(() => plan.load());

  let cap = $state(4);

  function weekDays(start: string): string[] {
    const d = new Date(start + 'T00:00:00Z');
    return Array.from({ length: 7 }, (_, i) => {
      const x = new Date(d);
      x.setUTCDate(d.getUTCDate() + i);
      return x.toISOString().split('T')[0];
    });
  }

  function dayName(iso: string): string {
    const d = new Date(iso + 'T12:00:00Z');
    return d.toLocaleDateString(i18n.locale, { weekday: 'short', day: 'numeric' });
  }

  function blocksFor(day: string) {
    return plan.plan?.blocks.filter((b) => b.day === day) ?? [];
  }

  function totalMinutes(day: string): number {
    return blocksFor(day).reduce((s, b) => s + b.minutes, 0);
  }

  async function generate() {
    await plan.generate(cap);
    if (plan.plan) await plan.persist();
  }

  function moveBlock(i: number, toDay: string) {
    plan.moveBlock(i, toDay);
  }

  let blockIndex = $derived.by(() => {
    const idx: Record<string, number[]> = {};
    plan.plan?.blocks.forEach((b, i) => {
      (idx[b.day] ??= []).push(i);
    });
    return idx;
  });
</script>

<section>
  <header>
    <h2>{m.study_plan_title({ weekStart: plan.weekStart })}</h2>
    <label>{m.study_plan_cap({ cap })}
      <input type="number" min="1" max="12" bind:value={cap} />
    </label>
    <button onclick={generate} disabled={plan.busy}>
      {plan.busy ? m.study_plan_generating() : m.study_plan_regenerate()}
    </button>
    {#if plan.plan}
      <button onclick={() => plan.persist()}>{m.study_plan_save()}</button>
    {/if}
  </header>

  {#if plan.error}<p class="err">{localizeError({ message: plan.error })}</p>{/if}

  {#if plan.plan}
    <div class="grid">
      {#each weekDays(plan.weekStart) as day}
        <article>
          <h4>{dayName(day)}</h4>
          <small>{m.study_plan_minutes({ minutes: totalMinutes(day) })}</small>
          {#each blocksFor(day) as _b, _j}
            {#if blockIndex[day]}
              {@const i = blockIndex[day][_j]}
              <div class="block">
                <strong>{plan.plan?.blocks[i].start}</strong> ·
                {plan.plan?.blocks[i].minutes}m ·
                {plan.plan?.blocks[i].kind}
                <p>{plan.plan?.blocks[i].rationale}</p>
                <select
                  value={day}
                  onchange={(e) => moveBlock(i, (e.target as HTMLSelectElement).value)}
                  aria-label={m.study_plan_move_aria()}
                >
                  {#each weekDays(plan.weekStart) as d}<option value={d}>{dayName(d)}</option>{/each}
                </select>
              </div>
            {/if}
          {/each}
        </article>
      {/each}
    </div>
  {:else}
    <p class="empty">{m.study_plan_empty()}</p>
  {/if}
</section>

<style>
  section { padding: 1rem; }
  header { display: flex; gap: 1rem; align-items: baseline; flex-wrap: wrap; }
  .grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 0.5rem; margin-top: 1rem; }
  article { background: var(--bg-elevated, #1a1a1a); padding: 0.5rem; border-radius: 0.5rem; min-height: 8rem; }
  article h4 { margin: 0 0 0.2rem; }
  .block { background: #222; padding: 0.4rem 0.5rem; margin: 0.4rem 0; border-radius: 0.3rem; font-size: 0.85rem; }
  .block select { margin-top: 0.3rem; width: 100%; }
  .err { color: #f44; }
  .empty { opacity: 0.7; }
</style>
