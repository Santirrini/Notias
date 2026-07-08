<script lang="ts" module>
  import type { Editor } from "@milkdown/core";
  import type { EditorView } from "prosemirror-view";

  /**
   * Public handle exposed to the parent (NoteCanvas) via `bind:handle`.
   * Every UI surface that needs to drive the editor — toolbar, slash menu,
   * ghost text, audio insert, external focus — talks to the editor through
   * this single interface.
   */
  export interface MilkdownHandle {
    focus: () => void;
    getMarkdown: () => string;
    setMarkdown: (md: string) => void;
    insertTextAtCursor: (text: string) => void;
    /** Delete N characters immediately before the cursor. */
    deleteLastChars: (n: number) => void;
    runCommand: (
      id: string,
      ctx: { body: string; title: string },
    ) => void | Promise<void>;
    triggerSuggest: () => void | Promise<void>;
    acceptSuggestion: () => void;
    dismissSuggestion: () => void;
    /** Subscribe to selection / text changes for ghost text + slash menu. */
    onStateChange: (cb: (s: EditorStateSnapshot) => void) => () => void;
  }

  export interface EditorStateSnapshot {
    text: string;
    cursorLine: string;
    rect: { top: number; left: number; bottom: number; right: number } | null;
    /** True when the current line is empty (or only whitespace). Useful for slash menu. */
    lineIsEmpty: boolean;
  }
</script>

<script lang="ts">
  import { onMount, onDestroy, untrack } from "svelte";
  import {
    Editor as MilkdownEditor,
    defaultValueCtx,
    editorViewCtx,
    editorViewOptionsCtx,
    rootCtx,
  } from "@milkdown/core";
  import { commonmark } from "@milkdown/preset-commonmark";
  import { nord } from "@milkdown/theme-nord";
  import "@milkdown/theme-nord/style.css";
  import { Sparkles, X } from "@lucide/svelte";
  import { InlineAi, type InlineSuggestion } from "$lib/editor/inline-ai";
  import { runCommandById } from "$lib/editor/commands";
  import {
    insertTextAtCursor as insertTextAtCursorUtil,
    serialize as serializeUtil,
    setMarkdown as setMarkdownUtil,
    coordsAtCursor as coordsAtCursorUtil,
    focusView as focusViewUtil,
  } from "$lib/editor/serialize";

  let {
    initial = "",
    onChange,
    handle = $bindable<MilkdownHandle | null>(null),
  }: {
    initial?: string;
    onChange?: (v: string) => void;
    handle?: MilkdownHandle | null;
  } = $props();

  let host: HTMLDivElement;
  let editor: MilkdownEditor | null = null;
  let view: EditorView | null = null;
  // svelte-ignore state_referenced_locally
  let lastEmitted = initial;
  let idleTimer: ReturnType<typeof setTimeout> | null = null;
  let subscribers: ((s: EditorStateSnapshot) => void)[] = [];

  let ghost: InlineSuggestion | null = $state(null);
  let ghostRect: { top: number; left: number; bottom: number } | null = $state(
    null,
  );

  const IDLE_MS = 1500;

  function snapshot(): EditorStateSnapshot {
    const text = view ? serializeUtil(editor, view) : "";
    const fromPos = view?.state.selection.$from;
    const lineText = fromPos
      ? (view?.state.doc.textBetween(fromPos.start(), fromPos.end(), "\n", "\n") ?? "")
      : "";
    const lineIsEmpty = lineText.trim() === "";
    const rect = coordsAtCursorUtil(view);
    return { text, cursorLine: lineText, rect, lineIsEmpty };
  }

  function notify() {
    if (!view) return;
    const snap = snapshot();
    ghostRect = snap.rect;
    for (const cb of subscribers) cb(snap);
  }

  function getText() {
    return view ? serializeUtil(editor, view) : "";
  }

  const ai = new InlineAi(
    () => getText(),
    (suggestion) => {
      ghost = suggestion;
      notify();
    },
    () => {
      // silent: backend store surfaces the offline state globally.
    },
  );

  function scheduleIdle() {
    if (idleTimer) clearTimeout(idleTimer);
    idleTimer = setTimeout(() => {
      ai.trigger();
    }, IDLE_MS);
  }

  function onTransaction() {
    if (!view || !editor) return;
    // Native transaction path. Re-apply state to the view (required because
    // dispatchTransaction replaces the default), serialize, dedupe, and
    // notify subscribers.
    const md = serializeUtil(editor, view);
    if (md !== lastEmitted) {
      lastEmitted = md;
      onChange?.(md);
    }
    // Auto-dismiss ghost text on every edit (the user is moving on).
    if (ghost) ai.dismiss();
    scheduleIdle();
    notify();
  }

  function acceptGhost() {
    if (!ghost) return;
    insertTextAtCursorUtil(view, ghost.text);
    ai.dismiss();
  }

  function dismissGhost() {
    ai.dismiss();
  }

  onMount(async () => {
    editor = await MilkdownEditor.make()
      .config((ctx) => {
        ctx.set(rootCtx, host);
        ctx.set(defaultValueCtx, initial);
        // Native transaction listener — replaces the 700ms setInterval we had
        // before. Every keystroke, selection change, or paste flows through
        // here, so onChange is now instantaneous.
        ctx.set(editorViewOptionsCtx, {
          dispatchTransaction(tr) {
            const v = ctx.get(editorViewCtx);
            v.updateState(v.state.apply(tr));
            onTransaction();
          },
        });
      })
      .use(commonmark)
      // ponytail: nord's published .d.ts returns void, but Editor.use wants a
      // CtxRunner. Runtime works; cast is the one-line patch.
      .use(nord as Parameters<Editor["use"]>[0])
      .create();

    view = editor.action((ctx) => ctx.get(editorViewCtx));

    // Build the public handle now that both editor and view are live.
    handle = {
      focus: () => focusViewUtil(view),
      getMarkdown: () => serializeUtil(editor, view),
      setMarkdown: (md: string) => {
        setMarkdownUtil(editor, view, md);
        lastEmitted = serializeUtil(editor, view);
        onChange?.(lastEmitted);
        notify();
      },
      insertTextAtCursor: (text: string) => insertTextAtCursorUtil(view, text),
      deleteLastChars: (n: number) => {
        if (n <= 0 || !view) return;
        const v = view;
        const { state, dispatch } = v;
        const from = state.selection.from;
        const to = Math.max(0, from - n);
        const tr = state.tr.insertText("", to, from);
        dispatch(tr);
        v.focus();
      },
      runCommand: (id, ctx) =>
        runCommandById(id, { editor, view, body: ctx.body, title: ctx.title }),
      triggerSuggest: () => ai.force(),
      acceptSuggestion: () => acceptGhost(),
      dismissSuggestion: () => dismissGhost(),
      onStateChange: (cb) => {
        subscribers.push(cb);
        // Fire once immediately with the current snapshot.
        untrack(() => cb(snapshot()));
        return () => {
          subscribers = subscribers.filter((s) => s !== cb);
        };
      },
    };
  });

  onDestroy(() => {
    if (idleTimer) clearTimeout(idleTimer);
    ai.cancel();
    handle = null;
    // The editor's destroy() returns a promise; we can't await in onDestroy
    // but we can fire it.
    void editor?.destroy();
  });
</script>

<div class="editor-host" bind:this={host}></div>

{#if ghost && ghostRect}
  <div
    class="ghost"
    role="status"
    aria-live="polite"
    style:top="{ghostRect.bottom + 6}px"
    style:left="{ghostRect.left}px"
  >
    <span class="ghost-text">{ghost.text}</span>
    <button
      type="button"
      class="ghost-accept"
      onclick={acceptGhost}
      aria-label="Accept suggestion (Tab)"
    >
      <Sparkles size={12} /> Accept
    </button>
    <button
      type="button"
      class="ghost-dismiss"
      onclick={dismissGhost}
      aria-label="Dismiss (Esc)"
    >
      <X size={12} />
    </button>
  </div>
{/if}

<!-- Keyboard handlers for ghost text. -->
<svelte:window
  onkeydown={(e) => {
    if (!ghost) return;
    if (e.key === "Tab") {
      e.preventDefault();
      acceptGhost();
    } else if (e.key === "Escape") {
      e.preventDefault();
      dismissGhost();
    }
  }}
/>

<style>
  .editor-host {
    min-height: 60vh;
  }
  .ghost {
    position: fixed;
    z-index: 40;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.4rem 0.25rem 0.5rem;
    background: var(--color-popover);
    color: var(--color-popover-foreground);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
    font-size: 0.8125rem;
    max-width: 32rem;
    pointer-events: auto;
    animation: ghost-in 120ms ease-out;
  }
  .ghost-text {
    color: var(--color-muted-foreground);
    font-style: italic;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 18rem;
  }
  .ghost-accept,
  .ghost-dismiss {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.15rem 0.45rem;
    border: 0;
    background: transparent;
    color: var(--color-muted-foreground);
    border-radius: var(--radius-sm);
    font-size: 0.75rem;
    cursor: pointer;
  }
  .ghost-accept:hover {
    background: var(--color-accent-subtle);
    color: var(--color-accent);
  }
  .ghost-dismiss:hover {
    background: var(--color-muted);
    color: var(--color-foreground);
  }
  @keyframes ghost-in {
    from { opacity: 0; transform: translateY(-2px); }
    to { opacity: 1; transform: translateY(0); }
  }
</style>
