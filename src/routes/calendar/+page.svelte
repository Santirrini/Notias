<script lang="ts">
  import { onMount } from "svelte";
  import {
    Calendar,
    RefreshCw,
    Plus,
    Sparkles,
    AlertCircle,
    Clock,
    CalendarPlus,
    CalendarDays,
    MapPin,
  } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import PageHeader from "$lib/components/PageHeader.svelte";
  import SectionCard from "$lib/components/SectionCard.svelte";
  import OfflineCallout from "$lib/components/OfflineCallout.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import { calendarList, calendarPull, calendarCreate } from "$lib/ipc";
  import type { CalendarEvent } from "$lib/types";

  let events = $state<CalendarEvent[]>([]);
  let busy = $state(false);
  let status = $state<string | null>(null);
  let err = $state<string | null>(null);
  let offline = $state(false);

  let summary = $state("");
  let description = $state("");
  let date = $state(new Date().toISOString().slice(0, 10));
  let startTime = $state("09:00");
  let endTime = $state("10:00");

  async function refresh() {
    if (typeof calendarList !== "function") {
      offline = true;
      return;
    }
    try {
      events = await calendarList();
      err = null;
      offline = false;
    } catch (e) {
      err = (e as Error).message;
      offline = true;
    }
  }

  async function pull() {
    busy = true;
    err = null;
    status = null;
    try {
      const n = await calendarPull(7);
      status = `Pulled ${n} events`;
      await refresh();
    } catch (e) {
      err = (e as Error).message;
      offline = true;
    } finally {
      busy = false;
    }
  }

  async function create() {
    if (!summary.trim()) return;
    busy = true;
    err = null;
    status = null;
    try {
      const start = `${date}T${startTime}:00`;
      const end = `${date}T${endTime}:00`;
      await calendarCreate(summary, description, start, end);
      summary = "";
      description = "";
      status = "Event created";
      await refresh();
    } catch (e) {
      err = (e as Error).message;
    } finally {
      busy = false;
    }
  }

  function fmt(iso: string): { date: string; time: string } {
    try {
      const d = new Date(iso);
      return {
        date: d.toLocaleDateString(undefined, {
          weekday: "short",
          month: "short",
          day: "numeric",
        }),
        time: d.toLocaleTimeString(undefined, {
          hour: "2-digit",
          minute: "2-digit",
        }),
      };
    } catch {
      return { date: iso, time: "" };
    }
  }

  function duration(startIso: string, endIso: string): string {
    try {
      const ms = new Date(endIso).getTime() - new Date(startIso).getTime();
      const mins = Math.round(ms / 60_000);
      if (mins < 60) return `${mins}m`;
      const h = Math.floor(mins / 60);
      const m = mins % 60;
      return m === 0 ? `${h}h` : `${h}h ${m}m`;
    } catch {
      return "";
    }
  }

  const sortedEvents = $derived(
    [...events].sort(
      (a, b) => new Date(a.starts_at).getTime() - new Date(b.starts_at).getTime(),
    ),
  );

  function isToday(iso: string): boolean {
    const d = new Date(iso);
    const now = new Date();
    return (
      d.getFullYear() === now.getFullYear() &&
      d.getMonth() === now.getMonth() &&
      d.getDate() === now.getDate()
    );
  }

  onMount(refresh);
</script>

<PageHeader title="Calendar" description="Pull events from Google or add them locally.">
  {#snippet icon()}<Calendar size={22} />{/snippet}
  <Button onclick={pull} disabled={busy || offline}>
    <RefreshCw size={14} class={busy ? "animate-spin" : ""} />
    {busy ? "Syncing…" : "Sync now"}
  </Button>
</PageHeader>

<div class="page-pad">
  {#if offline}
    <OfflineCallout
      variant="warning"
      title="Backend not reachable"
      description="Calendar needs the Tauri backend. Run via `pnpm tauri dev` or check the connection."
    />
  {/if}

  {#if status}
    <p class="ok-msg"><Sparkles size={12} /> {status}</p>
  {/if}
  {#if err && !offline}
    <p class="err-msg"><AlertCircle size={12} /> {err}</p>
  {/if}

  <div class="grid">
    <SectionCard title="Create event" description="Events created here stay in your local mirror.">
      {#snippet icon()}<CalendarPlus size={14} />{/snippet}
      <form onsubmit={(e) => { e.preventDefault(); create(); }}>
        <div class="field">
          <Label for="cal-summary">Summary</Label>
          <Input id="cal-summary" bind:value={summary} required placeholder="Team sync" />
        </div>
        <div class="field">
          <Label for="cal-desc">Description</Label>
          <Input id="cal-desc" bind:value={description} placeholder="Optional" />
        </div>
        <div class="field-row">
          <div class="field">
            <Label for="cal-date">Date</Label>
            <Input id="cal-date" type="date" bind:value={date} />
          </div>
          <div class="field">
            <Label for="cal-start">Start</Label>
            <Input id="cal-start" type="time" bind:value={startTime} />
          </div>
          <div class="field">
            <Label for="cal-end">End</Label>
            <Input id="cal-end" type="time" bind:value={endTime} />
          </div>
        </div>
        <Button type="submit" disabled={busy || !summary.trim() || offline}>
          <Plus size={14} /> Create
        </Button>
      </form>
    </SectionCard>

    <SectionCard title="Events" description="{sortedEvents.length} in your mirror">
      {#snippet icon()}<CalendarDays size={14} />{/snippet}
      {#if sortedEvents.length === 0}
        <div class="empty">
          <span class="empty-icon"><Calendar size={28} /></span>
          <p>
            {#if offline}
              Backend not reachable — events will appear here once Tauri is up.
            {:else}
              No events yet. Click <strong>Sync now</strong> to pull from Google, or create one.
            {/if}
          </p>
        </div>
      {:else}
        <ul class="evs">
          {#each sortedEvents as ev (ev.gcal_id)}
            {@const s = fmt(ev.starts_at)}
            {@const e = fmt(ev.ends_at)}
            {@const today = isToday(ev.starts_at)}
            <li class="ev" class:today>
              <div class="ev-time">
                <span class="time-main">{s.time}</span>
                <span class="time-dur">{duration(ev.starts_at, ev.ends_at)}</span>
              </div>
              <div class="ev-body">
                <div class="ev-head">
                  <strong>{ev.summary}</strong>
                  {#if today}<Badge class="today-badge">Today</Badge>{/if}
                  <Badge variant="outline" class="src-badge">{ev.source}</Badge>
                </div>
                <p class="when">
                  <Calendar size={11} /> {s.date}
                  <span class="dot">·</span>
                  <Clock size={11} /> {s.time} → {e.time}
                </p>
                {#if ev.description}<p class="desc">{ev.description}</p>{/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </SectionCard>
  </div>
</div>

<style>
  .page-pad {
    padding: 1rem 1.5rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-width: 980px;
  }
  .grid {
    display: grid;
    grid-template-columns: minmax(280px, 360px) 1fr;
    gap: 1rem;
    align-items: start;
  }
  @media (max-width: 900px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }

  .ok-msg,
  .err-msg {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0;
    font-size: 0.8125rem;
    padding: 0.4rem 0.625rem;
    border-radius: var(--radius-sm);
  }
  .ok-msg {
    background: var(--color-success-subtle);
    color: var(--color-success);
  }
  .err-msg {
    background: var(--color-destructive-subtle);
    color: var(--color-destructive);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .field-row {
    display: grid;
    grid-template-columns: 2fr 1fr 1fr;
    gap: 0.5rem;
  }
  @media (max-width: 600px) {
    .field-row {
      grid-template-columns: 1fr;
    }
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.625rem;
    padding: 2rem 1rem;
    text-align: center;
    color: var(--color-muted-foreground);
  }
  .empty-icon {
    width: 48px;
    height: 48px;
    border-radius: 999px;
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .empty p {
    margin: 0;
    font-size: 0.875rem;
    max-width: 280px;
    line-height: 1.5;
  }

  .evs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .ev {
    display: flex;
    gap: 0.875rem;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.625rem 0.875rem;
    background: var(--color-background);
    transition: border-color 120ms ease, box-shadow 120ms ease;
  }
  .ev:hover {
    border-color: color-mix(in srgb, var(--color-accent) 35%, var(--color-border));
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
  }
  .ev.today {
    border-left: 3px solid var(--color-accent);
  }
  .ev-time {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 64px;
    padding: 0.375rem;
    background: var(--color-muted);
    border-radius: var(--radius-sm);
    text-align: center;
  }
  .time-main {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 0.9375rem;
    color: var(--color-foreground);
  }
  .time-dur {
    font-size: 0.65rem;
    color: var(--color-muted-foreground);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-top: 2px;
  }
  .ev-body {
    flex: 1;
    min-width: 0;
  }
  .ev-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .ev-head strong {
    font-size: 0.9375rem;
    color: var(--color-foreground);
  }
  :global(.today-badge) {
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  :global(.src-badge) {
    margin-left: auto;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .when {
    margin: 0.25rem 0 0;
    color: var(--color-muted-foreground);
    font-size: 0.75rem;
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    flex-wrap: wrap;
  }
  .when .dot {
    opacity: 0.5;
  }
  .desc {
    margin: 0.375rem 0 0;
    color: var(--color-foreground);
    font-size: 0.875rem;
    opacity: 0.9;
    line-height: 1.45;
  }

  :global(.animate-spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>