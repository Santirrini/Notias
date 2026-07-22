<script lang="ts">
  import { onDestroy } from "svelte";
  import {
    Cloud,
    Loader2,
    CloudOff,
    Hash,
    MoreHorizontal,
    Tag,
    Plus,
    Trash2,
    X as XIcon,
    Check,
    CircleAlert,
  } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { notes } from "$lib/stores/notes.svelte";
  import { backend } from "$lib/stores/backend.svelte";
  import { toast } from "svelte-sonner";
  import { goto } from "$app/navigation";
  import Milkdown, {
    type MilkdownHandle,
  } from "$lib/editor/Milkdown.svelte";
  import EditorToolbar from "$lib/components/EditorToolbar.svelte";
  import EditorStatus from "$lib/components/EditorStatus.svelte";
  import SlashMenu from "$lib/components/SlashMenu.svelte";
  import AudioRecorder from "$lib/components/AudioRecorder.svelte";
  import { paperPrefs } from "$lib/stores/paper.svelte";
  import {
    resolvePaper,
    toEditorStyle,
    type PaperPrefs,
    type ResolvedPaper,
  } from "$lib/editor/paper";
  import { formatInteger } from "$lib/i18n";
  // Lazy import so React + Excalidraw (~1MB) are not pulled into the
  // initial bundle. The modal is only fetched when the user first opens it.
  type ExcalidrawModalComponent = typeof import("./drawing/ExcalidrawModal.svelte").default;
  let ExcalidrawModal = $state<ExcalidrawModalComponent | null>(null);
  let drawingModalOpen = $state(false);
  let editingDrawingId = $state<string | null>(null);

  type SaveState = "idle" | "saving" | "saved" | "dirty" | "error";

  let {
    id,
    initialTitle = "",
    initialBody = "",
  }: {
    id: string;
    initialTitle?: string;
    initialBody?: string;
  } = $props();

  let title = $state(initialTitle);
  // svelte-ignore state_referenced_locally
  let body = $state(initialBody);
  let saveState: SaveState = $state("idle");
  let lastSavedAt = $state<Date | null>(null);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  let editorHandle = $state<MilkdownHandle | null>(null);
  let tagInputOpen = $state(false);
  let tagInput = $state("");
  let tagInputRef = $state<HTMLElement | null>(null);
  let audioOpen = $state(false);
  let titleFocused = $state(false);

  const words = $derived(
    body.trim() ? body.trim().split(/\s+/).filter(Boolean).length : 0,
  );
  const chars = $derived(body.length);

  const currentTags = $derived(notes.list.find((n) => n.id === id)?.tags ?? []);

  // Paper / typography resolution. The resolver applies the precedence:
  // frontmatter override → global default → hardcoded fallback, and
  // produces both a CSS variable map and the class name to put on
  // `.editor-host` (e.g. `paper-ruled`).
  const resolvedPaper: ResolvedPaper = $derived(
    resolvePaper(notes.current?.frontmatter, paperPrefs.state)
  );
  const paperStyle = $derived(toEditorStyle(resolvedPaper));
  const paperOverrides = $derived(resolvedPaper.overrides);

  async function setPaperField<K extends keyof PaperPrefs>(
    key: K,
    value: PaperPrefs[K],
  ) {
    // Discrete change — no debounce. Persist immediately so a reload
    // restores the override and the IPC round-trip stays in sync.
    await notes.save(id, { [key]: value } as Record<string, string>);
  }

  async function clearPaperOverrides() {
    // Clear every key the note currently overrides by sending `""` (the
    // Rust side maps that to `None`, removing the key from frontmatter).
    const clear: Record<string, string> = {};
    for (const k of paperOverrides) clear[k as string] = "";
    if (Object.keys(clear).length === 0) return;
    await notes.save(id, clear);
  }

  /** Serialize a CSS variable map to an inline style string. */
  function cssVars(vars: Record<string, string>): string {
    return Object.entries(vars)
      .map(([k, v]) => `${k}:${v}`)
      .join(";");
  }

  // ── Drawing modal ─────────────────────────────────────────────────────
  //
  // Lazy-load the React + Excalidraw bundle the first time the user opens
  // the modal. Subsequent opens reuse the cached module.

  async function openDrawingNew() {
    if (!ExcalidrawModal) {
      const mod = await import("./drawing/ExcalidrawModal.svelte");
      ExcalidrawModal = mod.default;
    }
    editingDrawingId = null;
    drawingModalOpen = true;
  }

  function openDrawingEdit(relSvgPath: string) {
    if (!ExcalidrawModal) {
      void (async () => {
        const mod = await import("./drawing/ExcalidrawModal.svelte");
        ExcalidrawModal = mod.default;
        editingDrawingId = relSvgPath;
        drawingModalOpen = true;
      })();
      return;
    }
    editingDrawingId = relSvgPath;
    drawingModalOpen = true;
  }

  function closeDrawingModal() {
    drawingModalOpen = false;
    editingDrawingId = null;
  }

  async function onDrawingSaved(relSvgPath: string, wasEditing: boolean) {
    if (wasEditing) {
      // The image is already embedded in the body — no markdown change needed.
      // Re-rendering is a no-op since src didn't change. Toast so the user
      // gets feedback that the save succeeded.
      toast.success(m.drawing_save());
      return;
    }
    // New drawing — embed at the cursor. Insert blank lines around so the
    // image lands on its own paragraph.
    const markdown = `\n\n![](${relSvgPath})\n\n`;
    editorHandle?.insertTextAtCursor(markdown);
  }

  /** Click delegation on the editor host: detect clicks on <img> tags that
   *  reference an Excalidraw SVG and open the modal in edit mode. */
  function onEditorHostClick(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    if (!target) return;
    if (target.tagName !== "IMG") return;
    const img = target as HTMLImageElement;
    const src = img.getAttribute("src") ?? img.src;
    // Match `attachments/{noteId}/{drawingId}.svg` — the exact rel returned
    // by the backend's save_drawing command.
    const match = /^attachments\/[^/]+\/[^/]+\.svg$/.exec(src);
    if (!match) return;
    openDrawingEdit(src);
  }

  function scheduleSave() {
    saveState = "dirty";
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(flush, 700);
  }

  async function flush() {
    if (saveTimer) {
      clearTimeout(saveTimer);
      saveTimer = null;
    }
    saveState = "saving";
    try {
      await notes.save(id, { title, body });
      saveState = "saved";
      lastSavedAt = new Date();
    } catch {
      saveState = "error";
    }
  }

  function onTitle(e: Event) {
    title = (e.target as HTMLInputElement).value;
    scheduleSave();
  }

  function onBody(v: string) {
    body = v;
    scheduleSave();
  }

  onDestroy(() => {
    if (saveState === "dirty" || saveState === "saving") {
      void notes.save(id, { title, body });
    }
  });

  function startTagInput() {
    tagInputOpen = true;
    tagInput = "";
    requestAnimationFrame(() => tagInputRef?.focus());
  }

  function cancelTagInput() {
    tagInputOpen = false;
    tagInput = "";
  }

  function addTag() {
    const t = tagInput.trim().replace(/^#/, "").toLowerCase();
    if (!t) {
      tagInputOpen = false;
      return;
    }
    const note = notes.list.find((n) => n.id === id);
    const current = note?.tags ?? [];
    if (current.includes(t)) {
      toast.info("Already tagged", { description: `#${t}` });
      tagInputOpen = false;
      tagInput = "";
      return;
    }
    notes.save(id, { title }).then(() => {
      try {
        const KEY = "notias.localNotes.v1";
        const raw = localStorage.getItem(KEY);
        if (!raw) return;
        const list = JSON.parse(raw) as { id: string; tags: string[] }[];
        const idx = list.findIndex((n) => n.id === id);
        if (idx < 0) return;
        list[idx].tags = [...(list[idx].tags ?? []), t];
        localStorage.setItem(KEY, JSON.stringify(list));
      } catch {
        /* ignore */
      }
      tagInput = "";
      tagInputOpen = false;
      toast.success("Tag added", { description: `#${t}` });
      notes.refresh();
    });
  }

  function removeTag(t: string) {
    try {
      const KEY = "notias.localNotes.v1";
      const raw = localStorage.getItem(KEY);
      if (!raw) return;
      const list = JSON.parse(raw) as { id: string; tags: string[] }[];
      const idx = list.findIndex((n) => n.id === id);
      if (idx < 0) return;
      list[idx].tags = (list[idx].tags ?? []).filter((x) => x !== t);
      localStorage.setItem(KEY, JSON.stringify(list));
      toast.success("Tag removed", { description: `#${t}` });
      notes.refresh();
    } catch {
      /* ignore */
    }
  }

  async function duplicate() {
    const n = await notes.create(title || "Untitled copy");
    if (n) {
      await notes.save(n.id, { body });
      goto(`/notes/${n.id}`);
      toast.success("Duplicated");
      return;
    }
    if (notes.lastError) {
      toast.error(notes.lastError);
    } else {
      toast.error("Could not duplicate page");
    }
  }

  async function remove() {
    await notes.remove(id);
    if (notes.lastError) {
      toast.error(notes.lastError);
      notes.lastError = null;
    }
    goto("/notes");
  }
</script>

<article class="canvas">
  <header class="head">
    <input
      class="title"
      class:focused={titleFocused}
      type="text"
      placeholder="Untitled — click to add a title"
      value={title}
      oninput={onTitle}
      onfocus={() => (titleFocused = true)}
      onblur={() => (titleFocused = false)}
      aria-label="Note title"
      spellcheck="false"
    />

    <div class="head-actions">
      <div class="save-state" data-state={saveState} title={label(saveState, lastSavedAt)}>
        {#if saveState === "saving"}
          <Loader2 size={12} class="spin" />
          <span>Saving…</span>
        {:else if saveState === "error"}
          <CloudOff size={12} />
          <span>Save failed</span>
        {:else if saveState === "dirty"}
          <CircleAlert size={12} />
          <span>Unsaved</span>
        {:else}
          <Check size={12} />
          <span>Saved</span>
        {/if}
      </div>

      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          {#snippet child({ props })}
            <Button variant="ghost" size="icon-sm" aria-label="More" {...props}>
              <MoreHorizontal size={14} />
            </Button>
          {/snippet}
        </DropdownMenu.Trigger>
        <DropdownMenu.Content align="end" sideOffset={4} class="w-52">
          <DropdownMenu.Item onclick={duplicate}>
            <Plus size={12} /> Duplicate
          </DropdownMenu.Item>
          <DropdownMenu.Item disabled>
            <Tag size={12} /> Export Markdown
          </DropdownMenu.Item>
          <DropdownMenu.Separator />
          <DropdownMenu.Item variant="destructive" onclick={remove}>
            <Trash2 size={12} />
            Delete note
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </div>
  </header>

  <div class="meta">
    <div class="left">
      <span class="meta-pill">{formatInteger(words)} {words === 1 ? m.note_canvas_word_one() : m.note_canvas_word_other()}</span>
      <span class="meta-pill">{formatInteger(chars)} {m.note_canvas_chars()}</span>
      <span class="meta-pill">{readingTime(words)} {m.note_canvas_read()}</span>
      {#if !backend.available}
        <span class="meta-pill warn"><CloudOff size={10} /> {m.note_canvas_offline()}</span>
      {/if}
    </div>
    <div class="right">
      <EditorStatus
        state={saveState}
        {lastSavedAt}
        wordCount={words}
        charCount={chars}
      />
    </div>
  </div>

  <div class="tags">
    {#each currentTags as t (t)}
      <span class="tag">
        <Hash size={10} />
        <span>{t}</span>
        <button
          type="button"
          class="tag-x"
          onclick={() => removeTag(t)}
          aria-label={m.note_canvas_remove_tag({ tag: t })}
        >
          <XIcon size={9} />
        </button>
      </span>
    {/each}
    {#if tagInputOpen}
      <Input
        bind:ref={tagInputRef}
        class="tag-input"
        placeholder={m.note_canvas_add_tag()}
        bind:value={tagInput}
        onkeydown={(e) => {
          if (e.key === "Enter") addTag();
          if (e.key === "Escape") cancelTagInput();
        }}
        onblur={() => {
          if (tagInput.trim()) addTag();
          else cancelTagInput();
        }}
      />
    {:else}
      <button
        type="button"
        class="add-tag"
        onclick={startTagInput}
        aria-label={m.note_canvas_add_tag_aria()}
      >
        <Tag size={11} /> add tag
      </button>
    {/if}
  </div>

  <div class="editor-row">
    <EditorToolbar
      handle={editorHandle}
      body={body}
      title={title}
      onRecordClick={() => (audioOpen = !audioOpen)}
      onOpenDrawing={openDrawingNew}
      paperOverrides={paperOverrides}
      onPaperChange={(patch) => {
        for (const [k, v] of Object.entries(patch)) {
          void setPaperField(k as keyof PaperPrefs, v as PaperPrefs[keyof PaperPrefs]);
        }
      }}
      onPaperClear={clearPaperOverrides}
    />
  </div>

  {#if audioOpen}
    <div class="audio-row">
      <AudioRecorder
        onInsert={(text) => {
          editorHandle?.insertTextAtCursor("\n\n" + text + "\n\n");
          audioOpen = false;
        }}
      />
    </div>
  {/if}

  <div
    class="editor-host {paperStyle.classes}"
    style={cssVars(paperStyle.style)}
    onclick={onEditorHostClick}
    role="presentation"
  >
    <Milkdown
      initial={initialBody}
      bind:handle={editorHandle}
      onChange={onBody}
    />
  </div>

  <SlashMenu handle={editorHandle} body={body} title={title} />
</article>

{#if ExcalidrawModal && drawingModalOpen}
  <ExcalidrawModal
    open={drawingModalOpen}
    noteId={id}
    drawingId={editingDrawingId}
    onSaved={(rel) =>
      onDrawingSaved(rel, editingDrawingId !== null)}
    onClose={closeDrawingModal}
  />
{/if}

<!-- Icons used in dropdown items above -->

<script module lang="ts">
  import { m, i18n } from "$lib/i18n";
  function label(s: "idle" | "saving" | "saved" | "dirty" | "error", last: Date | null): string {
    if (s === "saving") return m.editor_status_saving();
    if (s === "dirty") return m.editor_status_dirty();
    if (s === "error") return m.editor_status_error();
    if (s === "saved" && last) return m.editor_status_saved_at({
      time: last.toLocaleTimeString(i18n.locale),
    });
    return m.editor_status_saved();
  }
  function readingTime(words: number): string {
    const min = Math.max(1, Math.round(words / 200));
    return m.editor_details_minutes({ minutes: min });
  }
</script>

<style>
  .canvas {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1.5rem 3rem 4rem;
    width: 100%;
    max-width: var(--editor-page-width, 880px);
    margin: 0 auto;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--color-border);
  }
  .title {
    flex: 1;
    background: transparent;
    border: 0;
    outline: 0;
    font-size: 2rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    line-height: 1.15;
    color: var(--color-foreground);
    font-family: inherit;
    padding: 0.25rem 0;
    border-radius: var(--radius-sm);
    transition: background 120ms ease;
  }
  .title.focused {
    background: color-mix(in srgb, var(--color-accent) 4%, transparent);
  }
  .title::placeholder {
    color: var(--color-subtle-foreground);
  }

  .head-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding-top: 0.5rem;
  }

  .save-state {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0.625rem;
    font-size: 0.75rem;
    border-radius: 999px;
    background: var(--color-muted);
    color: var(--color-muted-foreground);
    transition: background 120ms ease, color 120ms ease;
  }
  .save-state[data-state="saving"] {
    background: color-mix(in srgb, var(--color-warning) 14%, transparent);
    color: var(--color-warning, #ffb900);
  }
  .save-state[data-state="saved"] {
    background: color-mix(in srgb, var(--color-success) 14%, transparent);
    color: var(--color-success, #107c10);
  }
  .save-state[data-state="dirty"] {
    background: color-mix(in srgb, var(--color-warning) 14%, transparent);
    color: var(--color-warning, #ffb900);
  }
  .save-state[data-state="error"] {
    background: var(--color-destructive-subtle);
    color: var(--color-destructive);
  }
  :global(.save-state .spin) {
    animation: nc-spin 1s linear infinite;
  }
  @keyframes nc-spin {
    to { transform: rotate(360deg); }
  }

  .meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding-bottom: 0.5rem;
    font-size: 0.75rem;
    color: var(--color-muted-foreground);
  }
  .left,
  .right {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .meta-pill {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: var(--color-muted);
    padding: 0.125rem 0.5rem;
    border-radius: 999px;
    font-size: 0.7rem;
    color: var(--color-muted-foreground);
    font-variant-numeric: tabular-nums;
  }
  .meta-pill.warn {
    background: color-mix(in srgb, var(--color-warning) 14%, transparent);
    color: var(--color-warning, #ffb900);
  }

  .tags {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
    padding-bottom: 0.25rem;
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    padding: 0.125rem 0.5rem 0.125rem 0.5rem;
    border-radius: 999px;
    font-size: 0.7rem;
    font-weight: 500;
    line-height: 1.4;
  }
  .tag-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border: 0;
    background: transparent;
    color: var(--color-accent);
    opacity: 0.6;
    border-radius: 999px;
    cursor: pointer;
    transition: opacity 100ms ease, background 100ms ease;
  }
  .tag-x:hover {
    opacity: 1;
    background: color-mix(in srgb, var(--color-accent) 18%, transparent);
  }

  .add-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    background: transparent;
    border: 1px dashed var(--color-border);
    color: var(--color-muted-foreground);
    font-size: 0.7rem;
    cursor: pointer;
    padding: 0.125rem 0.5rem;
    border-radius: 999px;
    transition: color 100ms ease, border-color 100ms ease, background 100ms ease;
    line-height: 1.4;
  }
  .add-tag:hover {
    color: var(--color-foreground);
    border-color: var(--color-accent);
    background: var(--color-accent-subtle);
  }

  :global(.tag-input) {
    height: 1.625rem;
    width: 8.5rem;
    font-size: 0.7rem;
    padding: 0 0.5rem;
    border-radius: 999px;
  }

  .editor-row {
    padding-top: 0.25rem;
  }
  .audio-row {
    padding: 0.5rem 0;
    border-top: 1px dashed var(--color-border);
  }
  .editor-host {
    padding: 1rem 1.5rem 2rem;
    border-radius: var(--radius-md);
    transition: background-color 200ms ease;
  }
</style>