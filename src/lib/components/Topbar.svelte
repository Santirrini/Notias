<script lang="ts">
  import {
    Sun,
    Moon,
    Search,
    Command,
    Menu,
    FileText,
    Plus,
    ArrowRight,
    Hash,
    Sparkles,
    NotebookText,
    MessageSquare,
    Calendar,
    ListTodo,
    GraduationCap,
    Settings,
    type Icon as IconType,
  } from "@lucide/svelte";
  import { toggleMode, mode } from "mode-watcher";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Kbd } from "$lib/components/ui/kbd/index.js";
  import { goto } from "$app/navigation";
  import { notes } from "$lib/stores/notes.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import { toast } from "svelte-sonner";
  import type { NoteSummary } from "$lib/types";

  let { onopenlist }: { onopenlist?: () => void } = $props();

  let query = $state("");
  let inputEl = $state<HTMLInputElement | null>(null);
  let focused = $state(false);
  let open = $state(false);

  type NavItem = { href: string; label: string; icon: typeof IconType };
  type SearchAction = {
    id: string;
    label: string;
    group: "Notes" | "Navigate" | "Notes actions" | "AI";
    icon: typeof IconType;
    shortcut?: string;
    description?: string;
    run: () => void | Promise<void>;
  };

  const NAV_ITEMS: NavItem[] = [
    { href: "/notes", label: "Notes", icon: NotebookText },
    { href: "/chat", label: "Chat", icon: MessageSquare },
    { href: "/calendar", label: "Calendar", icon: Calendar },
    { href: "/tasks", label: "Tasks", icon: ListTodo },
    { href: "/study", label: "Study", icon: GraduationCap },
    { href: "/settings", label: "Settings", icon: Settings },
  ];

  const noteResults = $state<NoteSummary[]>([]);
  let searching = $state(false);
  let debouncer: ReturnType<typeof setTimeout> | null = null;
  let activeIndex = $state(0);

  function openPalette() {
    window.dispatchEvent(new CustomEvent("notias:open-palette"));
  }

  function focusInput() {
    inputEl?.focus();
    inputEl?.select();
  }

  async function runSearch(q: string) {
    const needle = q.trim();
    if (!needle) {
      noteResults.length = 0;
      searching = false;
      return;
    }
    searching = true;
    try {
      if (notes.list.length === 0) await notes.refresh();
      const lc = needle.toLowerCase();
      noteResults.splice(
        0,
        noteResults.length,
        ...notes.list
          .filter(
            (n) =>
              n.title.toLowerCase().includes(lc) ||
              n.tags.some((t) => t.toLowerCase().includes(lc)),
          )
          .slice(0, 6),
      );
    } finally {
      searching = false;
    }
  }

  function scheduleSearch(q: string) {
    if (debouncer) clearTimeout(debouncer);
    debouncer = setTimeout(() => runSearch(q), 140);
  }

  function onInput(e: Event) {
    query = (e.target as HTMLInputElement).value;
    activeIndex = 0;
    scheduleSearch(query);
  }

  function clearQuery() {
    query = "";
    noteResults.length = 0;
    activeIndex = 0;
    focusInput();
  }

  function navigate(href: string) {
    closeResults();
    goto(href);
  }

  async function quickCreate() {
    try {
      const n = await notes.create("Untitled");
      closeResults();
      goto(`/notes/${n.id}`);
    } catch {
      toast.error("Could not create page");
    }
  }

  function matchScore(haystack: string, needle: string): number {
    if (!needle) return 0;
    const h = haystack.toLowerCase();
    const n = needle.toLowerCase();
    if (h === n) return 100;
    if (h.startsWith(n)) return 60;
    if (h.includes(n)) return 30;
    return 0;
  }

  const filteredNav = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return NAV_ITEMS;
    return NAV_ITEMS.filter((n) => n.label.toLowerCase().includes(q));
  });

  const noteActions: SearchAction[] = $derived.by(() => {
    const out: SearchAction[] = [];
    if (noteResults.length > 0) {
      out.push({
        id: "create-page",
        label: "Create new page",
        group: "Notes actions",
        icon: Plus,
        shortcut: "Ctrl N",
        run: quickCreate,
      });
    } else if (!query.trim()) {
      out.push({
        id: "create-page",
        label: "Create new page",
        group: "Notes actions",
        icon: Plus,
        shortcut: "Ctrl N",
        run: quickCreate,
      });
    }
    return out;
  });

  type ListItem =
    | { kind: "note"; id: string; title: string; tags: string[]; updated: string }
    | { kind: "action"; action: SearchAction }
    | { kind: "nav"; item: NavItem };

  const flatList = $derived.by<ListItem[]>(() => {
    const items: ListItem[] = [];
    for (const n of noteResults) {
      items.push({
        kind: "note",
        id: n.id,
        title: n.title || "Untitled",
        tags: n.tags,
        updated: n.updated,
      });
    }
    for (const a of noteActions) {
      items.push({ kind: "action", action: a });
    }
    for (const n of filteredNav) {
      items.push({ kind: "nav", item: n });
    }
    return items;
  });

  $effect(() => {
    // Keep activeIndex within range when results change
    if (activeIndex >= flatList.length) activeIndex = 0;
  });

  function closeResults() {
    query = "";
    noteResults.length = 0;
    activeIndex = 0;
    inputEl?.blur();
  }

  function activateIndex(i: number) {
    const item = flatList[i];
    if (!item) return;
    if (item.kind === "note") {
      closeResults();
      goto(`/notes/${item.id}`);
    } else if (item.kind === "action") {
      closeResults();
      void item.action.run();
    } else if (item.kind === "nav") {
      navigate(item.item.href);
    }
  }

  function highlight(text: string, q: string): string {
    if (!q) return text;
    const idx = text.toLowerCase().indexOf(q.toLowerCase());
    if (idx < 0) return text;
    return (
      escapeHtml(text.slice(0, idx)) +
      "<mark>" +
      escapeHtml(text.slice(idx, idx + q.length)) +
      "</mark>" +
      escapeHtml(text.slice(idx + q.length))
    );
  }

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function onKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      openPalette();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n") {
      // Let browser fall through; we only intercept when not in editor
      if ((e.target as HTMLElement)?.tagName === "INPUT") return;
      e.preventDefault();
      void quickCreate();
      return;
    }
    // When our input is focused
    if (document.activeElement === inputEl) {
      if (e.key === "Escape") {
        e.preventDefault();
        if (query) clearQuery();
        else inputEl?.blur();
        return;
      }
      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (flatList.length > 0) {
          activeIndex = (activeIndex + 1) % flatList.length;
        }
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        if (flatList.length > 0) {
          activeIndex = (activeIndex - 1 + flatList.length) % flatList.length;
        }
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        activateIndex(activeIndex);
        return;
      }
    }
  }

  function onFocus() {
    focused = true;
    if (query.trim()) open = true;
  }
  function onBlur() {
    // Delay so clicks on items register first
    setTimeout(() => {
      focused = false;
      open = false;
    }, 120);
  }

  $effect(() => {
    open = focused && (query.trim().length > 0 || noteResults.length > 0 || noteActions.length > 0);
  });

  const showHint = $derived(!query.trim() && focused && !open);
</script>

<svelte:window onkeydown={onKey} />

<header class="topbar">
  {#if onopenlist}
    <button type="button" class="list-toggle" onclick={onopenlist} aria-label="Toggle pages list">
      <Menu size={18} />
    </button>
  {/if}

  <div class="search" class:open>
    <span class="search-icon"><Search size={14} /></span>
    <input
      bind:this={inputEl}
      type="search"
      placeholder="Search notes, jump anywhere, run commands…"
      value={query}
      oninput={onInput}
      onfocus={onFocus}
      onblur={onBlur}
      aria-label="Search notes"
      autocomplete="off"
      spellcheck="false"
    />
    {#if query}
      <button type="button" class="clear" onclick={clearQuery} aria-label="Clear search">
        <span class="kbd-clear">Esc</span>
      </button>
    {:else}
      <button type="button" class="kbd-btn" onclick={openPalette} aria-label="Open command palette">
        <Kbd><Command size={10} /> K</Kbd>
      </button>
    {/if}

    {#if open && (query.trim() || noteResults.length > 0 || noteActions.length > 0)}
      <div class="dropdown" role="listbox" aria-label="Search results">
        {#if noteResults.length > 0}
          <section>
            <h3>
              <FileText size={11} /> Pages
              <span class="count">{noteResults.length}</span>
            </h3>
            <ul>
              {#each noteResults as n, i (n.id)}
                <li>
                  <button
                    type="button"
                    class="result"
                    class:active={activeIndex === i}
                    onclick={() => activateIndex(i)}
                    onmouseenter={() => (activeIndex = i)}
                  >
                    <span class="result-icon"><FileText size={14} /></span>
                    <span class="result-body">
                      <span class="title">{@html highlight(n.title || "Untitled", query)}</span>
                      <span class="meta">
                        {#if n.tags.length}
                          {#each n.tags.slice(0, 3) as t}
                            <span class="tag-chip"><Hash size={9} />{t}</span>
                          {/each}
                        {/if}
                        <span class="time">· {new Date(n.updated).toLocaleDateString()}</span>
                      </span>
                    </span>
                    <ArrowRight size={12} class="arrow" />
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if noteActions.length > 0}
          <section>
            <h3>
              <Sparkles size={11} /> Actions
            </h3>
            <ul>
              {#each noteActions as a, i (a.id)}
                {@const idx = noteResults.length + i}
                <li>
                  <button
                    type="button"
                    class="result"
                    class:active={activeIndex === idx}
                    onclick={() => activateIndex(idx)}
                    onmouseenter={() => (activeIndex = idx)}
                  >
                    <span class="result-icon"><a.icon size={14} /></span>
                    <span class="result-body">
                      <span class="title">{a.label}</span>
                      {#if a.description}<span class="meta">{a.description}</span>{/if}
                    </span>
                    {#if a.shortcut}<Kbd>{a.shortcut}</Kbd>{/if}
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if filteredNav.length > 0}
          <section>
            <h3>
              <ArrowRight size={11} /> Navigate
            </h3>
            <ul>
              {#each filteredNav as nav, i (nav.href)}
                {@const idx = noteResults.length + noteActions.length + i}
                <li>
                  <button
                    type="button"
                    class="result"
                    class:active={activeIndex === idx}
                    onclick={() => activateIndex(idx)}
                    onmouseenter={() => (activeIndex = idx)}
                  >
                    <span class="result-icon"><nav.icon size={14} /></span>
                    <span class="result-body">
                      <span class="title">Go to {nav.label}</span>
                    </span>
                    <span class="hint-kbd">/{nav.href.slice(1)}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if query.trim() && noteResults.length === 0 && filteredNav.length === 0 && !searching}
          <div class="empty">
            <p>No matches for <strong>"{query}"</strong></p>
            <button type="button" class="empty-cta" onclick={quickCreate}>
              <Plus size={12} /> Create page
            </button>
          </div>
        {/if}
      </div>
    {/if}

    {#if showHint}
      <div class="dropdown hint-dropdown">
        <p class="hint-line"><Kbd>↑</Kbd><Kbd>↓</Kbd> to navigate, <Kbd>Enter</Kbd> to open, <Kbd>Esc</Kbd> to clear</p>
        <p class="hint-line">Try: <em>meeting</em>, <em>Go to</em>, <em>Toggle theme</em></p>
      </div>
    {/if}
  </div>

  <div class="actions">
    <Button
      variant="ghost"
      size="icon"
      onclick={toggleMode}
      aria-label="Toggle theme"
      title={mode.current === "dark" ? "Switch to light" : "Switch to dark"}
    >
      {#if mode.current === "dark"}
        <Sun size={16} />
      {:else}
        <Moon size={16} />
      {/if}
    </Button>
  </div>
</header>

<style>
  .topbar {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-background);
    min-height: 52px;
    z-index: 25;
  }

  .search {
    position: relative;
    flex: 1;
    min-width: 0;
    max-width: 720px;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 0.375rem 0 0.75rem;
    background: var(--color-muted);
    border: 1px solid transparent;
    border-radius: var(--radius-md);
    transition:
      border-color 120ms ease,
      background 120ms ease,
      box-shadow 120ms ease;
  }
  .search:focus-within,
  .search.open {
    background: var(--color-background);
    border-color: var(--color-ring);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-ring) 18%, transparent);
  }
  .search-icon {
    display: inline-flex;
    color: var(--color-muted-foreground);
    flex-shrink: 0;
  }
  input {
    flex: 1;
    min-width: 0;
    height: 32px;
    background: transparent;
    border: 0;
    outline: 0;
    color: var(--color-foreground);
    font-size: 0.875rem;
    font-family: inherit;
    padding: 0;
  }
  input::placeholder {
    color: var(--color-muted-foreground);
  }
  input::-webkit-search-cancel-button {
    display: none;
  }

  .clear {
    display: inline-flex;
    align-items: center;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--color-muted-foreground);
    flex-shrink: 0;
  }

  .kbd-btn {
    display: inline-flex;
    align-items: center;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--color-muted-foreground);
    flex-shrink: 0;
  }
  .kbd-clear {
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    color: var(--color-muted-foreground);
    border: 1px solid var(--color-border);
    border-radius: 3px;
    padding: 1px 5px;
    background: var(--color-background);
  }

  .dropdown {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    right: 0;
    background: var(--color-popover);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.18);
    z-index: 30;
    max-height: 60vh;
    overflow-y: auto;
    padding: 0.375rem;
    animation: drop-in 120ms ease;
  }
  .hint-dropdown {
    padding: 0.625rem 0.875rem;
  }
  @keyframes drop-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  section {
    padding: 0.25rem 0;
  }
  section + section {
    border-top: 1px solid var(--color-border);
    margin-top: 0.25rem;
    padding-top: 0.375rem;
  }
  h3 {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.6875rem;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-muted-foreground);
    margin: 0.25rem 0.5rem 0.25rem;
  }
  h3 .count {
    margin-left: auto;
    background: var(--color-muted);
    padding: 1px 6px;
    border-radius: 999px;
    font-size: 0.6875rem;
    letter-spacing: 0;
    text-transform: none;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .result {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 0.625rem;
    width: 100%;
    border: 0;
    background: transparent;
    border-radius: var(--radius-sm);
    color: var(--color-foreground);
    text-align: left;
    font-size: 0.875rem;
    transition: background 80ms ease;
  }
  .result.active,
  .result:hover {
    background: var(--color-muted);
  }
  .result.active {
    box-shadow: inset 2px 0 0 var(--color-accent);
  }
  .result-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    color: var(--color-muted-foreground);
    flex-shrink: 0;
  }
  .result.active .result-icon {
    background: var(--color-accent-subtle);
    color: var(--color-accent);
  }
  .result-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .result .title {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .result :global(mark) {
    background: color-mix(in srgb, var(--color-accent) 22%, transparent);
    color: var(--color-accent);
    padding: 0 2px;
    border-radius: 2px;
  }
  .result .meta {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.7rem;
    color: var(--color-muted-foreground);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    background: var(--color-muted);
    padding: 0 5px;
    border-radius: 999px;
    font-size: 0.65rem;
  }
  .hint-kbd {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--color-muted-foreground);
    background: var(--color-muted);
    padding: 1px 6px;
    border-radius: 3px;
  }

  :global(.result .arrow) {
    color: var(--color-muted-foreground);
    opacity: 0;
    transition: opacity 80ms ease;
  }
  .result.active :global(.arrow) {
    opacity: 1;
    color: var(--color-accent);
  }

  .empty {
    padding: 1rem 0.5rem 0.625rem;
    color: var(--color-muted-foreground);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.625rem;
  }
  .empty p {
    margin: 0;
    font-size: 0.875rem;
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
  }
  .empty-cta:hover {
    background: color-mix(in srgb, var(--color-accent) 18%, transparent);
  }

  .hint-line {
    margin: 0;
    font-size: 0.75rem;
    color: var(--color-muted-foreground);
  }
  .hint-line + .hint-line {
    margin-top: 0.25rem;
  }
  .hint-line em {
    background: var(--color-muted);
    padding: 1px 5px;
    border-radius: 3px;
    font-style: normal;
    font-size: 0.7rem;
    margin: 0 1px;
  }

  .actions {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    margin-left: auto;
    flex-shrink: 0;
  }

  .list-toggle {
    display: none;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: transparent;
    border: 0;
    color: var(--color-muted-foreground);
    border-radius: var(--radius-sm);
    flex-shrink: 0;
    transition: background 120ms ease, color 120ms ease;
  }
  .list-toggle:hover {
    background: var(--color-muted);
    color: var(--color-foreground);
  }

  @media (max-width: 799px) {
    .list-toggle {
      display: inline-flex;
    }
  }
  @media (max-width: 600px) {
    .search input::placeholder {
      color: transparent;
    }
    .kbd-btn {
      display: none;
    }
  }
</style>