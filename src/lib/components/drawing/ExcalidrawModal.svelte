<!--
  ExcalidrawModal — Svelte 5 wrapper that mounts a React Excalidraw instance
  inside a fullscreen Dialog. The Svelte side handles:
  - Dialog open/close state
  - Lazy import of React + Excalidraw (only on first open)
  - IPC roundtrip (save → reload from disk → restore state on edit)
  - Injecting Paraglide strings into the React component as plain props

  The React component (`ExcalidrawReact.tsx`) handles all drawing logic and
  exposes an imperative handle for `serialize()` so this parent can save the
  SVG + .excalidraw JSON through `safeInvoke`.
-->
<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { createRoot, type Root } from "react-dom/client";
  import { createElement } from "react";
  import * as Dialog from "$lib/components/ui/dialog/index.js";
  import { safeInvoke } from "$lib/stores/backend.svelte";
  import {
    ExcalidrawDrawing,
    type ExcalidrawDrawingHandle,
  } from "./ExcalidrawReact";
  import { m } from "$lib/i18n";
  import { toast } from "svelte-sonner";

  // `ExcalidrawElement` is inferred in the React wrapper; we mirror it here
  // for the JSON parsed from the .excalidraw state file. The shape is
  // deliberately permissive — any plain object array works at runtime, and
  // Excalidraw itself validates on load.
  type ExcalidrawElement = Record<string, unknown>;

  type Props = {
    open: boolean;
    noteId: string;
    /** When opening for editing: relative SVG path (e.g. attachments/01J0.../abc.svg).
     *  When opening for a new drawing: null. */
    drawingId: string | null;
    /** Called with the SVG relative path after a successful save. */
    onSaved: (relPath: string) => void;
    /** Called when the user cancels / dismisses without saving. */
    onClose: () => void;
  };

  let { open, noteId, drawingId, onSaved, onClose }: Props = $props();

  let host: HTMLDivElement | null = $state(null);
  let reactRoot: Root | null = null;
  let drawRef = $state<ExcalidrawDrawingHandle | null>(null);
  let initialElements: ExcalidrawElement[] | null = $state(null);
  let loading = $state(false);
  /** The actual drawing id used for save. We assign a fresh ULID on first
   *  open and reuse the original when editing. */
  let activeDrawingId = $state<string | null>(null);

  // ---- Modal lifecycle ----------------------------------------------------

  // When the modal opens, ensure we have a fresh drawing id and (if editing)
  // load the previous state from disk. We do this in an effect keyed on
  // (open, drawingId) so cancel + reopen works correctly.
  $effect(() => {
    if (!open) return;
    if (drawingId) {
      activeDrawingId = drawingId;
      void loadExisting(drawingId);
    } else {
      activeDrawingId = cryptoRandomId();
      initialElements = null;
    }
  });

  $effect(() => {
    if (!open || !host) return;
    mountReact();
  });

  onDestroy(() => {
    unmountReact();
  });

  // ---- IPC ----------------------------------------------------------------

  async function loadExisting(drawingId: string) {
    loading = true;
    const stateRel = drawingId.replace(/\.svg$/, ".excalidraw");
    const result = await safeInvoke<Uint8Array>("read_drawing_state", {
      path: stateRel,
    });
    loading = false;
    if (!result.ok) {
      toast.error(m.drawing_error_load());
      return;
    }
    try {
      const text = new TextDecoder().decode(result.value);
      const parsed = JSON.parse(text) as { elements?: ExcalidrawElement[] };
      initialElements = parsed.elements ?? [];
    } catch {
      toast.error(m.drawing_error_load());
      initialElements = [];
    }
  }

  async function save() {
    if (!drawRef) return;
    if (!activeDrawingId) return;
    const { svg, state } = await drawRef.serialize();
    if (svg.byteLength === 0) {
      toast.error(m.drawing_empty_hint());
      return;
    }
    // Frontend IPC expects Vec<u8>; pass plain numbers (Tauri serializes).
    const result = await safeInvoke<string>("save_drawing", {
      noteId,
      drawingId: activeDrawingId,
      svg: Array.from(svg),
      state: Array.from(state),
    });
    if (!result.ok) {
      toast.error(m.drawing_error_save());
      return;
    }
    onSaved(result.value);
    close();
  }

  function close() {
    onClose();
  }

  // ---- React mount --------------------------------------------------------

  function mountReact() {
    if (!host) return;
    unmountReact();
    reactRoot = createRoot(host);
    renderReact();
  }

  function unmountReact() {
    if (reactRoot) {
      try {
        reactRoot.unmount();
      } catch {
        // ignore: can throw during hot reload
      }
      reactRoot = null;
    }
  }

  function renderReact() {
    if (!reactRoot || !host) return;
    reactRoot.render(
      createElement(ExcalidrawDrawing, {
        ref: (api: ExcalidrawDrawingHandle | null): void => {
          drawRef = api;
        },
        // Excalidraw's `OrderedExcalidrawElement` is a branded subtype that's
        // structurally identical to the JSON we read back from `.excalidraw`
        // — we cast through `unknown` because the type lives behind a public
        // prop signature and we already validated shape via JSON.parse.
        initialElements: initialElements as unknown as Parameters<
          typeof ExcalidrawDrawing
        >[0]["initialElements"],
        onSave: () => void save(),
        onCancel: () => close(),
        labels: {
          save: m.drawing_save(),
          cancel: m.drawing_cancel(),
          saveAndClose: m.drawing_save_and_close(),
          empty: m.drawing_empty_hint(),
        },
      }),
    );
  }

  function cryptoRandomId(): string {
    // 16-char base36 random id is plenty for local file names — ULID is the
    // canonical id but we don't need time-orderable here.
    if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
      return crypto.randomUUID().replace(/-/g, "").slice(0, 16);
    }
    return Math.random().toString(36).slice(2, 10) + Date.now().toString(36);
  }
</script>

<Dialog.Root
  {open}
  onOpenChange={(v) => {
    if (!v) close();
  }}
>
  <Dialog.Content
    class="drawing-modal"
    showCloseButton={false}
    aria-describedby={undefined}
  >
    <Dialog.Title class="sr-only">
      {drawingId ? m.drawing_modal_title_edit() : m.drawing_modal_title_new()}
    </Dialog.Title>
    {#if loading}
      <div class="drawing-loading">{m.drawing_loading()}</div>
    {/if}
    <div bind:this={host} class="drawing-host"></div>
  </Dialog.Content>
</Dialog.Root>

<style>
  /* Use :global to reach the bits-ui portaled dialog content */
  :global(.drawing-modal) {
    width: 96vw !important;
    max-width: 96vw !important;
    height: 92vh !important;
    max-height: 92vh !important;
    padding: 0 !important;
    background: var(--color-popover);
    border: 1px solid var(--color-border);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  :global(.drawing-modal [data-slot="dialog-content"]) {
    width: 100%;
    height: 100%;
    max-width: none;
    max-height: none;
  }

  .drawing-host {
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .drawing-loading {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: var(--color-popover);
    color: var(--color-muted-foreground);
    z-index: 1;
    font-size: 0.875rem;
  }

  /* Styles for the React-rendered tree inside .drawing-host. Scoped via
     :global because they're emitted by React, not Svelte. */
  :global(.drawing-host .exc-shell) {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: var(--color-popover);
  }

  :global(.drawing-host .exc-shell > .excalidraw) {
    flex: 1;
    min-height: 0;
  }

  :global(.drawing-host .exc-footer) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border-top: 1px solid var(--color-border);
    background: var(--color-popover);
  }

  :global(.drawing-host .exc-hint) {
    color: var(--color-muted-foreground);
    font-size: 0.8125rem;
  }

  :global(.drawing-host .exc-hint--empty) {
    opacity: 0.6;
  }

  :global(.drawing-host .exc-actions) {
    display: flex;
    gap: 0.5rem;
  }

  :global(.drawing-host .exc-btn) {
    padding: 0.4rem 0.875rem;
    border-radius: 6px;
    border: 1px solid var(--color-border);
    background: transparent;
    color: var(--color-foreground);
    font-size: 0.8125rem;
    cursor: pointer;
    transition: background 0.12s;
  }

  :global(.drawing-host .exc-btn:hover:not(:disabled)) {
    background: var(--color-muted);
  }

  :global(.drawing-host .exc-btn:disabled) {
    opacity: 0.4;
    cursor: not-allowed;
  }

  :global(.drawing-host .exc-btn--primary) {
    background: var(--color-accent);
    color: var(--color-accent-foreground);
    border-color: var(--color-accent);
  }

  :global(.drawing-host .exc-btn--primary:hover:not(:disabled)) {
    background: var(--color-accent-hover);
  }
</style>
