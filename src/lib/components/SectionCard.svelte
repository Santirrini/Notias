<script lang="ts">
  import type { Component } from "svelte";

  let {
    title,
    description,
    icon: IconComp,
    children,
  }: {
    title?: string;
    description?: string;
    icon?: Component;
    children: import("svelte").Snippet;
  } = $props();
</script>

<section class="section-card">
  {#if title || description}
    <header>
      <div class="title-row">
        {#if IconComp}<span class="icon"><IconComp size={16} /></span>{/if}
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
