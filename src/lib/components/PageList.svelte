<script lang="ts">
  import { goto } from "$app/navigation";
  import { browser } from "$app/environment";
  import { page } from "$app/state";
  import { toast } from "svelte-sonner";
  import { notes, makeLocalId } from "$lib/stores/notes.svelte";
  import { sections } from "$lib/stores/sections.svelte";
  import type { NoteSummary } from "$lib/types";
  import {
    Plus,
    Search,
    FileText,
    Trash2,
    ChevronRight,
    Inbox,
    AlertTriangle,
    X,
    NotebookPen,
    ArrowDownAZ,
    ArrowDownNarrowWide,
    Clock,
  } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { onMount } from "svelte";
  import { tick } from "svelte";
  import { m, formatDate } from "$lib/i18n";

  const FAKE_KEY = "notias.localNotes.v1";
  const SORT_KEY = "notias.pageList.sort.v1";

  type SortMode = "updated" | "created" | "title";
  const SORTS: { key: SortMode; label: string; icon: typeof Clock }[] = [
    { key: "updated", label: "Last updated", icon: Clock },
    { key: "created", label: "Created", icon: ArrowDownNarrowWide },
    { key: "title", label: "Title (A→Z)", icon: ArrowDownAZ },
  ];

  let items = $state<NoteSummary[]>([]);
  let loading = $state(true);
  let filter = $state("");
  let backendOk = $state(true);
  let debouncer: ReturnType<typeof setTimeout> | null = null;
  let sortMode = $state<SortMode>("updated");

  /** Load notes — try IPC first, fall back to localStorage. */
  async function refresh() {
    loading = true;
    await notes.refresh();
    // The store already routes offline vs error internally; we mirror the
    // items locally so sections/search keep working.
    items = notes.list;
    backendOk = !notes.lastError;
    seedSections();
    persistLocal();
    loading = false;
  }

  function loadLocal(): NoteSummary[] {
    if (!browser) return [];
    try {
      const raw = localStorage.getItem(FAKE_KEY);
      if (!raw) return seedDemo();
      const parsed = JSON.parse(raw) as NoteSummary[];
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }

  function persistLocal() {
    if (!browser) return;
    try {
      localStorage.setItem(FAKE_KEY, JSON.stringify(items));
    } catch {
      /* quota */
    }
  }

  /** Seed a couple of demo notes so the UI shows content immediately in dev. */
  function seedDemo(): NoteSummary[] {
    const now = new Date().toISOString();
    const demo: NoteSummary[] = [
      {
        id: "demo-welcome",
        title: "Welcome to Notias",
        tags: ["intro"],
        updated: now,
      },
      {
        id: "demo-onenote",
        title: "Why this UI feels like OneNote",
        tags: ["design"],
        updated: new Date(Date.now() - 86_400_000).toISOString(),
      },
      {
        id: "demo-tips",
        title: "Tips & shortcuts",
        tags: ["intro"],
        updated: new Date(Date.now() - 2 * 86_400_000).toISOString(),
      },
    ];
    if (browser) {
      try {
        localStorage.setItem(FAKE_KEY, JSON.stringify(demo));
      } catch {
        /* ignore */
      }
    }
    return demo;
  }

  function seedSections() {
    sections.hydrate();
    const uniqueTags = Array.from(
      new Set(items.flatMap((n) => n.tags))
    ).filter(Boolean);
    sections.seedFromTags(uniqueTags);
  }

  function sectionOf(n: NoteSummary): string {
    return sections.sectionFor(n.id, n.tags);
  }

  const visibleNotes = $derived.by(() => {
    const sectionName = sections.state.active ?? "General";
    const inSection = items.filter((n) => sectionOf(n) === sectionName);
    const needle = filter.trim().toLowerCase();
    const filtered = !needle
      ? inSection
      : inSection.filter(
          (n) =>
            n.title.toLowerCase().includes(needle) ||
            n.tags.some((t) => t.toLowerCase().includes(needle)),
        );
    const sorted = [...filtered].sort((a, b) => {
      if (sortMode === "title") return (a.title || "").localeCompare(b.title || "");
      return new Date(b.updated).getTime() - new Date(a.updated).getTime();
    });
    return sorted;
  });

  const sectionMeta = $derived.by(() => {
    const sectionName = sections.state.active ?? "General";
    const def = sections.state.sections.find((s) => s.name === sectionName);
    return {
      name: sectionName,
      color: def?.color ?? "#8a8886",
      count: items.filter((n) => sectionOf(n) === sectionName).length,
    };
  });

  function relative(date: string): string {
    const d = new Date(date);
    const now = Date.now();
    const diffMs = now - d.getTime();
    const day = 86_400_000;
    if (diffMs < day) return m.dates_today();
    if (diffMs < 2 * day) return m.dates_yesterday();
    if (diffMs < 7 * day) {
      const days = Math.floor(diffMs / day);
      return m.dates_days_ago_other({ days });
    }
    return formatDate(d);
  }

  /** Create a note. Tries the IPC; if it fails, uses a local id and persists. */
  async function add() {
    const created = await notes.create("Untitled");
    if (created) {
      await refresh();
      await tick();
      goto(`/notes/${created.id}`);
      return;
    }
    // Real backend error (not offline) — surface it.
    if (notes.lastError) {
      toast.error(notes.lastError);
      return;
    }
    // Offline: store created a local mirror note already; route to it.
    const id = makeLocalId();
    const note: NoteSummary = {
      id,
      title: "Untitled",
      tags: [],
      updated: new Date().toISOString(),
    };
    items = [note, ...items];
    persistLocal();
    await tick();
    goto(`/notes/${id}`);
    toast.warning("Created locally", {
      description:
        "Backend Tauri no reachable; this page lives in localStorage until backend is up.",
    });
  }

  async function remove(id: string, e?: MouseEvent) {
    e?.preventDefault();
    e?.stopPropagation();
    await notes.remove(id);
    items = items.filter((n) => n.id !== id);
    persistLocal();
    if (notes.lastError) {
      toast.error(notes.lastError);
      notes.lastError = null;
    }
    if (page.params.id === id) {
      goto("/notes");
    }
  }

  function onInput(e: Event) {
    filter = (e.target as HTMLInputElement).value;
  }

  function clearFilter() {
    filter = "";
  }

  function setSort(m: SortMode) {
    sortMode = m;
    if (browser) {
      try {
        localStorage.setItem(SORT_KEY, m);
      } catch {
        /* ignore */
      }
    }
  }

  onMount(() => {
    if (browser) {
      const saved = localStorage.getItem(SORT_KEY) as SortMode | null;
      if (saved && SORTS.some((s) => s.key === saved)) sortMode = saved;
    }
    refresh();
  });
</script>

<section class="page-list" aria-label="Pages in current section">
  <header>
    <div class="title-row">
      <span class="dot" style:background={sectionMeta.color} aria-hidden="true"></span>
      <h2 class="section-title">{sectionMeta.name}</h2>
      <span class="count">{sectionMeta.count}</span>
      {#if !backendOk}
        <span class="offline" title="Backend Tauri no reachable">
          <AlertTriangle size={12} />
        </span>
      {/if}
    </div>

    <div class="actions">
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label="Sort"
              title="Sort"
              {...props}
            >
              {#if sortMode === "title"}
                <ArrowDownAZ size={14} />
              {:else if sortMode === "created"}
                <ArrowDownNarrowWide size={14} />
              {:else}
                <Clock size={14} />
              {/if}
            </Button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="end" sideOffset={4} class="w-48">
          <DropdownMenu.Label>Sort by</DropdownMenu.Label>
          {#each SORTS as s}
            <DropdownMenu.Item onclick={() => setSort(s.key)}>
              <s.icon size={12} />
              <span>{s.label}</span>
              {#if sortMode === s.key}
                <span class="check">✓</span>
              {/if}
            </DropdownMenu.Item>
          {/each}
        </DropdownMenu.Content>
      </DropdownMenu.Root>
      <Button size="sm" onclick={add} aria-label="New page">
        <Plus size={14} /> New
      </Button>
    </div>
  </header>

  <div class="search-bar">
    <span class="search-icon"><Search size={14} /></span>
    <Input
      type="search"
      placeholder="Filter pages…"
      value={filter}
      oninput={onInput}
      aria-label="Filter pages"
      autocomplete="off"
      spellcheck="false"
    />
    {#if filter}
      <button type="button" class="clear" onclick={clearFilter} aria-label="Clear filter">
        <X size={12} />
      </button>
    {/if}
  </div>

  {#if loading}
    <div class="loading">
      <span class="loader"></span>
      Loading pages…
    </div>
  {:else if visibleNotes.length === 0}
    <div class="empty">
      {#if filter}
        <Search size={26} />
        <p>
          No pages match <strong>"{filter}"</strong> in <em>{sectionMeta.name}</em>.
        </p>
        <button type="button" class="empty-cta" onclick={clearFilter}>
          Clear filter
        </button>
      {:else}
        <NotebookPen size={26} />
        <p>
          No pages in <em>{sectionMeta.name}</em> yet.<br />
          Press <kbd>Ctrl</kbd>+<kbd>N</kbd> or click <strong>New</strong>.
        </p>
        <button type="button" class="empty-cta" onclick={add}>
          <Plus size={12} /> New page
        </button>
      {/if}
    </div>
  {:else}
    <ul class="pages">
      {#each visibleNotes as n (n.id)}
        <li class="row">
          <a href={`/notes/${n.id}`} class="link">
            <span class="file-icon"><FileText size={14} /></span>
            <div class="body">
              <span class="title">{n.title || "Untitled"}</span>
              <span class="meta">
                {relative(n.updated)}{#if n.tags.length} · {n.tags.join(", ")}{/if}
                {#if n.id.startsWith("LOCAL-") || n.id.startsWith("demo-")}
                  <span class="local-tag">local</span>
                {/if}
              </span>
            </div>
            <ChevronRight size={14} class="chev" />
          </a>
          <button
            type="button"
            class="del"
            onclick={(e) => remove(n.id, e)}
            aria-label={`Delete ${n.title}`}
            title="Delete"
          >
            <Trash2 size={13} />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .page-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-background);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.75rem 0.875rem 0.5rem;
  }
  .title-row {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }
  .dot {
    width: 9px;
    height: 9px;
    border-radius: 999px;
    flex-shrink: 0;
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-background) 80%, transparent);
  }
  .section-title {
    font-size: 0.95rem;
    font-weight: 600;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-foreground);
  }
  .count {
    font-size: 0.7rem;
    color: var(--color-muted-foreground);
    background: var(--color-muted);
    padding: 1px 7px;
    border-radius: 999px;
    font-variant-numeric: tabular-nums;
  }
  .offline {
    display: inline-flex;
    align-items: center;
    color: var(--color-warning, #ffb900);
  }
  .actions {
    display: inline-flex;
    gap: 0.25rem;
    align-items: center;
  }

  .search-bar {
    position: relative;
    display: flex;
    align-items: center;
    margin: 0 0.75rem 0.5rem;
  }
  .search-icon {
    position: absolute;
    left: 0.625rem;
    color: var(--color-muted-foreground);
    pointer-events: none;
    display: inline-flex;
  }
  .search-bar :global(input) {
    padding-left: 2rem;
    padding-right: 2rem;
    height: 30px;
    font-size: 0.8125rem;
  }
  .clear {
    position: absolute;
    right: 0.5rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    background: transparent;
    border: 0;
    color: var(--color-muted-foreground);
    border-radius: 999px;
    cursor: pointer;
  }
  .clear:hover {
    background: var(--color-muted);
    color: var(--color-foreground);
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 1rem;
    color: var(--color-muted-foreground);
    font-size: 0.875rem;
  }
  .loader {
    width: 12px;
    height: 12px;
    border: 2px solid var(--color-muted);
    border-top-color: var(--color-accent);
    border-radius: 999px;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.625rem;
    padding: 2.5rem 1rem;
    text-align: center;
    color: var(--color-muted-foreground);
  }
  .empty p {
    font-size: 0.875rem;
    max-width: 240px;
    line-height: 1.55;
    margin: 0;
    color: var(--color-foreground);
    opacity: 0.8;
  }
  .empty em {
    font-style: normal;
    background: var(--color-muted);
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 0.8125rem;
  }
  .empty kbd {
    background: var(--color-muted);
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 0.75rem;
    border: 1px solid var(--color-border);
    font-family: var(--font-mono);
  }
  .empty-cta {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    border: 0;
    border-radius: var(--radius-sm);
    padding: 0.375rem 0.625rem;
    font-size: 0.8125rem;
    font-family: inherit;
    cursor: pointer;
    margin-top: 0.25rem;
  }
  .empty-cta:hover {
    background: color-mix(in srgb, var(--color-accent) 18%, transparent);
  }

  .pages {
    list-style: none;
    margin: 0;
    padding: 0.25rem 0.5rem 1rem;
    overflow-y: auto;
    flex: 1;
  }
  .row {
    position: relative;
    border-radius: var(--radius-sm);
    margin-bottom: 1px;
    transition: background 100ms ease;
  }
  .row:hover {
    background: var(--color-muted);
  }
  .row:hover .del {
    opacity: 1;
  }
  .row:hover :global(.chev) {
    opacity: 1;
    transform: translateX(0);
  }
  .link {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 2rem 0.5rem 0.625rem;
    color: var(--color-foreground);
    text-decoration: none;
  }
  .file-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    color: var(--color-muted-foreground);
    flex-shrink: 0;
  }
  .row:hover .file-icon {
    color: var(--color-accent);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .title {
    font-size: 0.875rem;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    font-size: 0.7rem;
    color: var(--color-muted-foreground);
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .local-tag {
    display: inline-block;
    font-size: 0.6rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    background: var(--color-warning, #ffb900);
    color: #1a1a1a;
    padding: 0 5px;
    border-radius: 3px;
    line-height: 1.4;
    text-transform: uppercase;
  }
  :global(.chev) {
    color: var(--color-muted-foreground);
    opacity: 0;
    transform: translateX(-4px);
    transition: opacity 100ms ease, transform 100ms ease;
  }

  .del {
    position: absolute;
    top: 50%;
    right: 0.4rem;
    transform: translateY(-50%);
    width: 24px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 0;
    background: transparent;
    color: var(--color-muted-foreground);
    border-radius: var(--radius-sm);
    opacity: 0;
    cursor: pointer;
    transition: opacity 100ms ease, background 100ms ease, color 100ms ease;
  }
  .del:hover {
    background: var(--color-destructive-subtle);
    color: var(--color-destructive);
  }

  .check {
    margin-left: auto;
    color: var(--color-accent);
    font-weight: 600;
  }
</style>