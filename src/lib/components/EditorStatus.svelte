<script lang="ts">
  import { Cloud, CloudOff, Loader2, Check, FileText } from "@lucide/svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import * as Popover from "$lib/components/ui/popover/index.js";

  type SaveState = "idle" | "saving" | "saved" | "dirty" | "error";

  let {
    state = "idle",
    lastSavedAt = null,
    wordCount = 0,
    charCount = 0,
  }: {
    state?: SaveState;
    lastSavedAt?: Date | null;
    wordCount?: number;
    charCount?: number;
    onReveal?: () => void;
  } = $props();

  const label = $derived.by(() => {
    if (state === "saving") return "Saving…";
    if (state === "dirty") return "Unsaved changes";
    if (state === "error") return "Save failed";
    if (state === "saved" && lastSavedAt)
      return `Saved ${lastSavedAt.toLocaleTimeString()}`;
    return "All changes saved";
  });

  const tone = $derived(
    state === "error"
      ? "error"
      : state === "saving"
        ? "warn"
        : state === "dirty"
          ? "warn"
          : "ok",
  );
</script>

<Popover.Root>
  <Popover.Trigger>
    {#snippet child({ props })}
      <button
        type="button"
        class="pill"
        data-tone={tone}
        aria-label="Note status"
        {...props}
      >
        {#if state === "saving"}
          <Loader2 size={12} class="spin" />
        {:else if state === "error"}
          <CloudOff size={12} />
        {:else if state === "saved"}
          <Check size={12} />
        {:else}
          <Cloud size={12} />
        {/if}
        <span>{label}</span>
      </button>
    {/snippet}
  </Popover.Trigger>
  <Popover.Content class="status-popover" align="end" sideOffset={6}>
    <header>
      <span class="pop-icon"><FileText size={14} /></span>
      <h3>Note details</h3>
    </header>
    <dl>
      <div>
        <dt>Status</dt>
        <dd>{label}</dd>
      </div>
      <div>
        <dt>Words</dt>
        <dd>{wordCount.toLocaleString()}</dd>
      </div>
      <div>
        <dt>Characters</dt>
        <dd>{charCount.toLocaleString()}</dd>
      </div>
      <div>
        <dt>Reading time</dt>
        <dd>{Math.max(1, Math.round(wordCount / 200))} min</dd>
      </div>
      <div>
        <dt>Last saved</dt>
        <dd>{lastSavedAt ? lastSavedAt.toLocaleString() : "—"}</dd>
      </div>
    </dl>
  </Popover.Content>
</Popover.Root>

<style>
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.625rem;
    background: transparent;
    border: 0;
    border-radius: 999px;
    color: var(--color-muted-foreground);
    font-size: 0.75rem;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .pill:hover {
    background: var(--color-muted);
    color: var(--color-foreground);
  }
  .pill[data-tone="warn"] {
    color: var(--color-warning, #ffb900);
  }
  .pill[data-tone="error"] {
    color: var(--color-destructive);
  }
  .pill[data-tone="ok"] {
    color: var(--color-success, #107c10);
  }
  :global(.status-popover) {
    min-width: 16rem;
  }
  :global(.status-popover header) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.625rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--color-border);
  }
  :global(.status-popover .pop-icon) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--color-accent-subtle);
    color: var(--color-accent);
  }
  :global(.status-popover h3) {
    margin: 0;
    font-size: 0.875rem;
    font-weight: 600;
  }
  :global(.status-popover dl) {
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 1rem;
    row-gap: 0.4rem;
    margin: 0;
    font-size: 0.8125rem;
  }
  :global(.status-popover dt) {
    color: var(--color-muted-foreground);
  }
  :global(.status-popover dd) {
    margin: 0;
    font-variant-numeric: tabular-nums;
    color: var(--color-foreground);
  }
  :global(.spin) {
    animation: status-spin 1s linear infinite;
  }
  @keyframes status-spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>