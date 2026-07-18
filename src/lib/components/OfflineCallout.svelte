<script lang="ts">
  import { Info, TriangleAlert, X } from "@lucide/svelte";
  import { m } from "$lib/i18n";

  type Variant = "info" | "warning" | "destructive";

  let {
    variant = "info",
    title,
    description,
    dismissable = false,
    ondismiss,
  }: {
    variant?: Variant;
    title?: string;
    description?: string;
    dismissable?: boolean;
    ondismiss?: () => void;
  } = $props();

  let dismissed = $state(false);
  const visible = $derived(!dismissed);

  function handleDismiss() {
    dismissed = true;
    ondismiss?.();
  }
</script>

{#if visible}
  <aside class="callout" data-variant={variant} role="status">
    <span class="callout-icon">
      {#if variant === "destructive" || variant === "warning"}
        <TriangleAlert size={15} />
      {:else}
        <Info size={15} />
      {/if}
    </span>
    <div class="text">
      {#if title}<strong>{title}</strong>{/if}
      {#if description}<p>{description}</p>{/if}
    </div>
    {#if dismissable}
      <button type="button" class="dismiss" onclick={handleDismiss} aria-label={m.offline_callout_dismiss_aria()}>
        <X size={14} />
      </button>
    {/if}
  </aside>
{/if}

<style>
  .callout {
    display: flex;
    align-items: flex-start;
    gap: 0.625rem;
    padding: 0.625rem 0.875rem;
    margin-bottom: 1rem;
    border-radius: var(--radius-md);
    font-size: 0.8125rem;
    line-height: 1.45;
    border: 1px solid transparent;
  }
  .callout-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    flex-shrink: 0;
  }
  .callout[data-variant="info"] {
    background: var(--color-accent-subtle);
    color: var(--color-foreground);
    border-color: color-mix(in srgb, var(--color-accent) 25%, transparent);
  }
  .callout[data-variant="info"] .callout-icon {
    background: color-mix(in srgb, var(--color-accent) 18%, transparent);
    color: var(--color-accent);
  }
  .callout[data-variant="warning"] {
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
    color: var(--color-foreground);
    border-color: color-mix(in srgb, var(--color-warning) 35%, transparent);
  }
  .callout[data-variant="warning"] .callout-icon {
    background: color-mix(in srgb, var(--color-warning) 18%, transparent);
    color: var(--color-warning);
  }
  .callout[data-variant="destructive"] {
    background: var(--color-destructive-subtle);
    color: var(--color-foreground);
    border-color: color-mix(in srgb, var(--color-destructive) 35%, transparent);
  }
  .callout[data-variant="destructive"] .callout-icon {
    background: color-mix(in srgb, var(--color-destructive) 18%, transparent);
    color: var(--color-destructive);
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }
  strong {
    font-weight: 600;
    color: var(--color-foreground);
  }
  p {
    margin: 0;
    color: var(--color-muted-foreground);
  }
  .dismiss {
    background: transparent;
    border: 0;
    padding: 0.25rem;
    color: inherit;
    opacity: 0.6;
    border-radius: 3px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: opacity 100ms ease, background 100ms ease;
  }
  .dismiss:hover {
    opacity: 1;
    background: rgba(0, 0, 0, 0.05);
  }
</style>