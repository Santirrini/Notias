<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { toggleMode } from "mode-watcher";
  import { toast } from "svelte-sonner";
  import {
    Plus,
    Sun,
    Moon,
    RefreshCw,
    Download,
    Upload,
    Sparkles,
    FileText,
    ArrowRight,
    NotebookText,
    MessageSquare,
    Calendar,
    ListTodo,
    GraduationCap,
    Settings,
    type Icon as IconType,
  } from "@lucide/svelte";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { Kbd } from "$lib/components/ui/kbd/index.js";
  import { notes } from "$lib/stores/notes.svelte";
  import { rebuildIndex, syncExportZip, syncImportZip } from "$lib/ipc";
  import type { NoteSummary } from "$lib/types";
  import { m, localizeError, isOffline, i18n } from "$lib/i18n";

  type NavItem = {
    href: string;
    labelKey: "nav_notes" | "nav_chat" | "nav_calendar" | "nav_tasks" | "nav_study" | "nav_settings";
    icon: typeof IconType;
  };
  type Action = {
    id: string;
    /** Resolved label (already localized) — needed because the Action
     * objects are created at module init time and can't call reactive
     * m.*() then; we resolve per-render in the markup and at the
     * filter step at call time. */
    label: string;
    group: "Navigate" | "Notes" | "Theme" | "Sync" | "AI";
    icon: typeof IconType;
    shortcut?: string;
    description?: string;
    run: () => Promise<void> | void;
  };

  const NAV_ITEMS: NavItem[] = [
    { href: "/notes", labelKey: "nav_notes", icon: NotebookText },
    { href: "/chat", labelKey: "nav_chat", icon: MessageSquare },
    { href: "/calendar", labelKey: "nav_calendar", icon: Calendar },
    { href: "/tasks", labelKey: "nav_tasks", icon: ListTodo },
    { href: "/study", labelKey: "nav_study", icon: GraduationCap },
    { href: "/settings", labelKey: "nav_settings", icon: Settings },
  ];

  function navLabel(item: NavItem): string {
    return m[item.labelKey]();
  }

  let open = $state(false);
  let query = $state("");
  let inputEl = $state<HTMLInputElement | null>(null);
  let noteResults = $state<NoteSummary[]>([]);
  let activeIndex = $state(0);
  let debouncer: ReturnType<typeof setTimeout> | null = null;

  function openIt() {
    open = true;
    query = "";
    noteResults = [];
    activeIndex = 0;
    queueMicrotask(() => inputEl?.focus());
  }
  function closeIt() {
    open = false;
    query = "";
    noteResults = [];
    activeIndex = 0;
  }

  async function newPage() {
    const n = await notes.create(m.topbar_untitled());
    if (n) goto(`/notes/${n.id}`);
    else toast.error(notes.lastError ? localizeError({ message: notes.lastError }) : m.topbar_could_not_create());
  }

  function jump(href: string) {
    goto(href);
  }

  /** Build an Action with already-localized strings. Called inside `$derived`
   * to stay in sync with `i18n.locale`. */
  const baseActions: Action[] = $derived.by(() => [
    {
      id: "new-page",
      label: m.palette_action_create_page(),
      icon: Plus,
      shortcut: "Ctrl N",
      group: "Notes",
      run: newPage,
    },
    ...NAV_ITEMS.map<Action>((j) => ({
      id: `jump:${j.href}`,
      label: m.palette_action_go_to({ label: navLabel(j) }),
      icon: j.icon,
      group: "Navigate",
      run: () => jump(j.href),
    })),
    {
      id: "theme:toggle",
      label: m.palette_action_toggle_theme(),
      icon: Sun,
      shortcut: "Ctrl Shift L",
      group: "Theme",
      run: () => toggleMode(),
    },
    {
      id: "theme:dark",
      label: m.palette_action_switch_dark(),
      icon: Moon,
      group: "Theme",
      run: () => {
        if (typeof document !== "undefined") {
          document.documentElement.classList.add("dark");
          try {
            localStorage.setItem("mode-watcher-mode", "dark");
          } catch {
            /* ignore */
          }
        }
      },
    },
    {
      id: "theme:light",
      label: m.palette_action_switch_light(),
      icon: Sun,
      group: "Theme",
      run: () => {
        if (typeof document !== "undefined") {
          document.documentElement.classList.remove("dark");
          try {
            localStorage.setItem("mode-watcher-mode", "light");
          } catch {
            /* ignore */
          }
        }
      },
    },
    {
      id: "rebuild-index",
      label: m.palette_action_rebuild_index(),
      icon: RefreshCw,
      description: m.palette_action_rebuild_index_desc(),
      group: "Sync",
      run: async () => {
        const r = await rebuildIndex();
        if (r.ok) toast.success(m.palette_toast_index_rebuilt());
        else toast.error(localizeError({ code: r.code, message: r.error }));
      },
    },
    {
      id: "sync:export",
      label: m.palette_action_export_zip(),
      icon: Download,
      group: "Sync",
      run: async () => {
        const r = await syncExportZip();
        if (r.ok)
          toast.success(m.palette_action_export_done(), { description: m.palette_action_export_desc() });
        else toast.error(localizeError({ code: r.code, message: r.error }));
      },
    },
    {
      id: "sync:import",
      label: m.palette_action_import_zip(),
      icon: Upload,
      group: "Sync",
      run: async () => {
        const input = document.createElement("input");
        input.type = "file";
        input.accept = ".zip";
        input.onchange = async () => {
          const f = input.files?.[0];
          if (!f) return;
          try {
            const buf = new Uint8Array(await f.arrayBuffer());
            const r = await syncImportZip(Array.from(buf));
            if (r.ok) toast.success(m.palette_action_import_done({ count: r.value }));
            else toast.error(localizeError({ code: r.code, message: r.error }));
          } catch (e) {
            toast.error(
              isOffline({ error: (e as Error).message })
                ? m.err_offline()
                : m.err_generic({ message: (e as Error).message }),
            );
          }
        };
        input.click();
      },
    },
    {
      id: "ai:summarize-page",
      label: m.palette_action_summarize_page(),
      icon: Sparkles,
      description: m.palette_action_summarize_page_desc(),
      group: "AI",
      run: () => goto("/settings"),
    },
    {
      id: "ai:settings",
      label: m.palette_action_open_ai_settings(),
      icon: Settings,
      group: "AI",
      run: () => goto("/settings"),
    },
  ]);

  const filteredActions = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return baseActions;
    return baseActions.filter(
      (a) =>
        a.label.toLowerCase().includes(needle) ||
        (a.description ?? "").toLowerCase().includes(needle),
    );
  });

  async function refetchNotes(q: string) {
    if (!q.trim()) {
      noteResults = [];
      return;
    }
    try {
      if (notes.list.length === 0) await notes.refresh();
      const needle = q.toLowerCase();
      noteResults = notes.list
        .filter(
          (n) =>
            n.title.toLowerCase().includes(needle) ||
            n.tags.some((t) => t.toLowerCase().includes(needle)),
        )
        .slice(0, 8);
    } catch {
      noteResults = [];
    }
  }

  type ListItem =
    | { kind: "note"; note: NoteSummary }
    | { kind: "action"; action: Action };

  const flatList = $derived.by<ListItem[]>(() => {
    const items: ListItem[] = [];
    for (const n of noteResults) items.push({ kind: "note", note: n });
    for (const a of filteredActions) items.push({ kind: "action", action: a });
    return items;
  });

  $effect(() => {
    if (activeIndex >= flatList.length) activeIndex = 0;
  });

  async function activateIndex(i: number) {
    const item = flatList[i];
    if (!item) return;
    closeIt();
    if (item.kind === "note") {
      goto(`/notes/${item.note.id}`);
    } else {
      await item.action.run();
    }
  }

  function pickNote(n: NoteSummary) {
    closeIt();
    goto(`/notes/${n.id}`);
  }

  function onKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      if (open) closeIt();
      else openIt();
      return;
    }
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      closeIt();
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (flatList.length > 0) activeIndex = (activeIndex + 1) % flatList.length;
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (flatList.length > 0)
        activeIndex = (activeIndex - 1 + flatList.length) % flatList.length;
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      void activateIndex(activeIndex);
      return;
    }
  }

  function onInput(v: string) {
    query = v;
    activeIndex = 0;
    if (debouncer) clearTimeout(debouncer);
    debouncer = setTimeout(() => refetchNotes(v), 120);
  }

  function highlight(text: string, q: string): string {
    if (!q) return text;
    const idx = text.toLowerCase().indexOf(q.toLowerCase());
    if (idx < 0) return escapeHtml(text);
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

  onMount(() => {
    const handler = () => openIt();
    window.addEventListener("notias:open-palette", handler);
    return () => window.removeEventListener("notias:open-palette", handler);
  });

  const groupedActions = $derived.by(() => {
    const groups: Record<string, Action[]> = {};
    for (const a of filteredActions) {
      if (!groups[a.group]) groups[a.group] = [];
      groups[a.group].push(a);
    }
    return groups;
  });

  // Stable render order; group names are localized at the markup layer.
  const groupOrder = ["Navigate", "Notes", "Theme", "Sync", "AI"] as const;

  /** Localized label for a palette group key (matches Action.group values). */
  function groupLabel(g: (typeof groupOrder)[number]): string {
    switch (g) {
      case "Navigate":
        return m.palette_group_navigate();
      case "Notes":
        return m.palette_group_notes();
      case "Theme":
        return m.palette_group_theme();
      case "Sync":
        return m.palette_group_sync();
      case "AI":
        return m.palette_group_ai();
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<Dialog.Root open={open} onOpenChange={(v) => (open = v)}>
  <Dialog.Content class="palette">
    <Dialog.Title class="sr-only">{m.palette_aria()}</Dialog.Title>
    <Dialog.Description class="sr-only">{m.palette_description()}</Dialog.Description>

    <div class="input-wrap">
      <span class="palette-icon"><Sparkles size={14} /></span>
      <input
        bind:this={inputEl}
        bind:value={query}
        oninput={(e) => onInput((e.target as HTMLInputElement).value)}
        placeholder={m.palette_placeholder()}
        aria-label={m.palette_aria()}
        autocomplete="off"
        spellcheck="false"
      />
      <span class="palette-shortcuts">
        <Kbd>↑</Kbd><Kbd>↓</Kbd>
        <span class="sep">·</span>
        <Kbd>↵</Kbd>
      </span>
    </div>

    <div class="results">
      {#if !query.trim()}
        <p class="hint">
          {m.palette_hint_try({
            example1: m.palette_hint_example_newpage(),
            example2: m.palette_hint_example_theme(),
            example3: m.palette_hint_example_settings(),
          })}
        </p>
      {/if}

      {#if noteResults.length > 0}
        <section>
          <h3>
            <FileText size={11} /> {m.palette_section_pages()}
            <span class="count">{noteResults.length}</span>
          </h3>
          <ul>
            {#each noteResults as n, i (n.id)}
              <li>
                <button
                  type="button"
                  class="result"
                  class:active={activeIndex === i}
                  onclick={() => pickNote(n)}
                  onmouseenter={() => (activeIndex = i)}
                >
                  <span class="result-icon"><FileText size={14} /></span>
                  <span class="result-body">
                    <span class="title">{@html highlight(n.title || m.topbar_untitled(), query)}</span>
                    <span class="meta">
                      {n.tags.join(", ") || m.topbar_no_tags()} · {new Date(n.updated).toLocaleDateString(i18n.locale)}
                    </span>
                  </span>
                  <ArrowRight size={12} class="arrow" />
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#each groupOrder as group}
        {@const items = groupedActions[group] ?? []}
        {#if items.length > 0}
          <section>
            <h3>{groupLabel(group)}</h3>
            <ul>
              {#each items as a (a.id)}
                {@const idx = noteResults.length + filteredActions.indexOf(a)}
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
      {/each}

      {#if query.trim() && filteredActions.length === 0 && noteResults.length === 0}
        <div class="empty">
          <p>{@html m.palette_no_matches({ query: `"${escapeHtml(query)}"` })}</p>
          <button type="button" class="empty-cta" onclick={() => { closeIt(); newPage(); }}>
            <Plus size={12} /> {m.topbar_create_page()}
          </button>
        </div>
      {/if}
    </div>
  </Dialog.Content>
</Dialog.Root>

<style>
  :global(.palette) {
    max-width: 36rem;
    padding: 0 !important;
    overflow: hidden;
  }

  .input-wrap {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 0.875rem;
    border-bottom: 1px solid var(--color-border);
  }
  .palette-icon {
    color: var(--color-muted-foreground);
    display: inline-flex;
  }
  .input-wrap input {
    flex: 1;
    height: 32px;
    background: transparent;
    border: 0;
    outline: 0;
    color: var(--color-foreground);
    font-size: 0.9375rem;
    font-family: inherit;
    padding: 0;
  }
  .input-wrap input::placeholder {
    color: var(--color-muted-foreground);
  }
  .palette-shortcuts {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--color-muted-foreground);
  }
  .palette-shortcuts .sep {
    font-size: 0.75rem;
    margin: 0 2px;
    opacity: 0.5;
  }

  .results {
    max-height: 60vh;
    overflow-y: auto;
    padding: 0.375rem;
  }
  .hint {
    margin: 0;
    padding: 0.5rem 0.875rem 0.875rem;
    font-size: 0.8125rem;
    color: var(--color-muted-foreground);
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
    margin: 0.25rem 0.5rem;
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
    font-family: inherit;
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
    width: 26px;
    height: 26px;
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
  .title {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.title mark) {
    background: color-mix(in srgb, var(--color-accent) 22%, transparent);
    color: var(--color-accent);
    padding: 0 2px;
    border-radius: 2px;
  }
  .meta {
    font-size: 0.7rem;
    color: var(--color-muted-foreground);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    padding: 1.5rem 1rem;
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
</style>