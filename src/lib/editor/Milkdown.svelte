<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Editor, rootCtx, defaultValueCtx, serializerCtx, editorViewCtx } from "@milkdown/core";
  import { commonmark } from "@milkdown/preset-commonmark";
  import { nord } from "@milkdown/theme-nord";
  import "@milkdown/theme-nord/style.css";
  import { InlineAi, type InlineSuggestion } from "$lib/editor/inline-ai";
  import AudioRecorder from "$lib/components/AudioRecorder.svelte";

  let { initial = "", onChange } = $props<{ initial?: string; onChange?: (v: string) => void }>();
  let host: HTMLDivElement;
  let editor: Editor | null = null;
  // svelte-ignore state_referenced_locally
  let lastEmitted = initial;
  let poll: ReturnType<typeof setInterval> | null = null;
  let suggestion = $state<InlineSuggestion | null>(null);

  onMount(async () => {
    editor = await Editor.make()
      .config(ctx => ctx.set(rootCtx, host))
      .config(ctx => ctx.set(defaultValueCtx, initial))
      .use(commonmark)
      // ponytail: nord's published .d.ts returns void, but Editor.use wants a
      // CtxRunner. Runtime works; cast is the one-line patch.
      .use(nord as Parameters<Editor["use"]>[0])
      .create();

    // ponytail: poll every 700ms; serialize the editor's markdown and emit via onChange.
    // A real Milkdown listener (editor.on(listenerCtx)) is the upgrade path; polling keeps
    // the Phase 1 surface area small and survives Milkdown API churn.
    poll = setInterval(() => {
      if (!editor || !onChange) return;
      const md = editor.action((ctx) => {
        const view = ctx.get(editorViewCtx);
        const serializer = ctx.get(serializerCtx);
        return serializer(view.state.doc);
      });
      if (md && md !== lastEmitted) {
        lastEmitted = md;
        onChange(md);
      }
    }, 700);
  });

  const ai = new InlineAi(
    () => editor?.action((ctx) => {
      const view = ctx.get(editorViewCtx);
      const serializer = ctx.get(serializerCtx);
      return serializer(view.state.doc);
    }) ?? "",
    (s) => { suggestion = s; },
  );

  function onSuggestClick() { ai.trigger(); }

  // ponytail: insertAtEnd appends transcript at end-of-doc via a manual DOM edit. Milkdown 7
  // has no stable public insertText action yet; Phase 3 ships the simplest working version.
  function insertAtEnd(text: string) {
    if (!editor) return;
    editor.action((ctx) => {
      const view = ctx.get(editorViewCtx);
      const { state } = view;
      const end = state.doc.content.size;
      const tr = state.tr.insertText("\n\n" + text + "\n", end);
      view.dispatch(tr);
    });
  }

  onDestroy(async () => {
    ai.cancel();
    if (poll) clearInterval(poll);
    await editor?.destroy();
  });
</script>

<div class="toolbar">
  <button onclick={onSuggestClick}>✨ Suggest</button>
  <AudioRecorder onInsert={insertAtEnd} />
</div>

<div bind:this={host} class="editor"></div>

{#if suggestion}
  <div class="suggestion-popover">
    <p>{suggestion.text}</p>
    <button onclick={suggestion.onAccept}>Accept (Tab)</button>
    <button onclick={suggestion.onDismiss}>Dismiss (Esc)</button>
  </div>
{/if}

<svelte:window onkeydown={(e) => {
  if (!suggestion) return;
  if (e.key === 'Tab') { e.preventDefault(); suggestion.onAccept(); }
  else if (e.key === 'Escape') { e.preventDefault(); suggestion.onDismiss(); }
}} />

<style>
  .editor { min-height: 60vh; }
  .toolbar { margin-bottom: 0.5rem; display: flex; gap: 0.75rem; align-items: flex-start; }
  .toolbar :global(.rec) { flex: 1; }
  .suggestion-popover {
    position: fixed; bottom: 1rem; right: 1rem;
    padding: 0.75rem; max-width: 30rem;
    background: var(--bg-elevated, #222); color: var(--fg, #eee);
    border: 1px solid var(--border, #444); border-radius: 0.5rem;
  }
</style>
