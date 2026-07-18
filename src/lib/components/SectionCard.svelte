<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    description,
    icon,
    children,
  }: {
    title?: string;
    description?: string;
    /**
     * Optional icon. Accepts a Svelte 5 snippet so callers can pass
     * `{#snippet icon()}<Cloud size={14} />{/snippet}` (the same shape used
     * by `PageHeader`). Previously this prop was typed as `Component`, which
     * mismatched snippets and threw `invalid_snippet_arguments` during
     * hydration, leaving the page blank.
     */
    icon?: Snippet;
    children: Snippet;
  } = $props();
</script>

<section class="section-card">
  {#if title || description}
    <header>
      <div class="title-row">
        {#if icon}<span class="icon">{@render icon()}</span>{/if}
        {#if title}<h2>{title}</h2>{/if}
      </div>
      {#if description}<p>{description}</p>{/if}
    </header>
  {/if}
  <div class="body">{@render children()}</div>
</section>

<style>
  .section-card {
    background: var(--color-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }
  header {
    padding: 0.875rem 1.125rem 0.625rem;
    border-bottom: 1px solid var(--color-border);
  }
  .title-row {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    background: var(--color-accent-subtle);
    color: var(--color-accent);
  }
  h2 {
    margin: 0;
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--color-foreground);
  }
  header p {
    margin: 0.25rem 0 0;
    color: var(--color-muted-foreground);
    font-size: 0.8125rem;
  }
  .body {
    padding: 1rem 1.125rem;
  }
</style>
