<script lang="ts">
  import { onMount } from 'svelte';
  import { calendarList, calendarPull, calendarCreate } from '$lib/ipc';
  import type { CalendarEvent } from '$lib/types';

  let events = $state<CalendarEvent[]>([]);
  let busy = $state(false);
  let status = $state<string | null>(null);
  let err = $state<string | null>(null);

  let summary = $state('');
  let description = $state('');
  let date = $state(new Date().toISOString().slice(0, 10));
  let startTime = $state('09:00');
  let endTime = $state('10:00');

  async function refresh() {
    try { events = await calendarList(); } catch (e) { err = (e as Error).message; }
  }

  async function pull() {
    busy = true; err = null; status = null;
    try {
      const n = await calendarPull(7);
      status = `Pulled ${n} events`;
      await refresh();
    } catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  async function create() {
    if (!summary.trim()) return;
    busy = true; err = null; status = null;
    try {
      const start = `${date}T${startTime}:00`;
      const end = `${date}T${endTime}:00`;
      await calendarCreate(summary, description, start, end);
      summary = ''; description = '';
      status = 'Created';
      await refresh();
    } catch (e) { err = (e as Error).message; }
    finally { busy = false; }
  }

  onMount(refresh);
</script>

<h1>Calendar</h1>

<section>
  <div class="row">
    <button onclick={pull} disabled={busy}>Sync now (next 7 days)</button>
    {#if status}<span class="ok">{status}</span>{/if}
    {#if err}<span class="err">{err}</span>{/if}
  </div>
</section>

<section>
  <h2>Create event</h2>
  <form onsubmit={(e) => { e.preventDefault(); create(); }}>
    <label><span>Summary</span><input bind:value={summary} required /></label>
    <label><span>Description</span><input bind:value={description} /></label>
    <label><span>Date</span><input type="date" bind:value={date} /></label>
    <label><span>Start</span><input type="time" bind:value={startTime} /></label>
    <label><span>End</span><input type="time" bind:value={endTime} /></label>
    <button type="submit" disabled={busy || !summary.trim()}>Create</button>
  </form>
</section>

<section>
  <h2>Events</h2>
  {#if events.length === 0}<p class="muted">No events. Click "Sync now" to pull from Google.</p>{/if}
  <ul class="evs">
    {#each events as ev (ev.gcal_id)}
      <li>
        <strong>{ev.summary}</strong>
        <span class="when">{ev.starts_at} → {ev.ends_at}</span>
        {#if ev.description}<p class="desc">{ev.description}</p>{/if}
        <span class="src">{ev.source}</span>
      </li>
    {/each}
  </ul>
</section>

<style>
  h1 { margin-top: 1rem; }
  section { margin: 1.25rem 0; }
  .row { display: flex; gap: .5rem; align-items: center; flex-wrap: wrap; }
  button { padding: .35rem .7rem; border: 1px solid #ccc; border-radius: 4px; background: #fff; cursor: pointer; }
  button:disabled { opacity: .5; cursor: default; }
  form { display: grid; gap: .5rem; max-width: 400px; }
  form label { display: grid; grid-template-columns: 100px 1fr; gap: .5rem; align-items: center; }
  form input { padding: .35rem; border: 1px solid #ccc; border-radius: 4px; }
  .ok { color: #178217; }
  .err { color: #c00; }
  .muted { color: #666; }
  .evs { list-style: none; padding: 0; }
  .evs li { border: 1px solid #eee; border-radius: 6px; padding: .5rem; margin-bottom: .5rem; }
  .evs .when { color: #666; font-size: .85em; margin-left: .5rem; }
  .evs .desc { margin: .25rem 0; color: #555; font-size: .9em; }
  .evs .src { display: inline-block; padding: 1px 6px; background: #f0f0f0; border-radius: 8px; font-size: .7em; color: #666; }
</style>