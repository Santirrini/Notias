<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Editor, rootCtx, defaultValueCtx, serializerCtx, editorViewCtx } from "@milkdown/core";
  import { commonmark } from "@milkdown/preset-commonmark";
  import { nord } from "@milkdown/theme-nord";
  import "@milkdown/theme-nord/style.css";

  let { initial = "", onChange } = $props<{ initial?: string; onChange?: (v: string) => void }>();
  let host: HTMLDivElement;
  let editor: Editor | null = null;
  // svelte-ignore state_referenced_locally
  let lastEmitted = initial;
  let poll: ReturnType<typeof setInterval> | null = null;

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

  onDestroy(async () => {
    if (poll) clearInterval(poll);
    await editor?.destroy();
  });
</script>

<div bind:this={host} class="editor"></div>

<style>
  .editor { min-height: 60vh; }
</style>