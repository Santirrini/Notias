<script lang="ts">
  let {
    title,
    description,
    icon,
    children,
  }: {
    title: string;
    description?: string;
    icon?: import("svelte").Snippet;
    children?: import("svelte").Snippet;
  } = $props();
</script>

<header class="page-header">
  <div class="title-row">
    {#if icon}
      <span class="icon-cell">{@render icon()}</span>
    {/if}
    <div class="titles">
      <h1>{title}</h1>
      {#if description}<p>{description}</p>{/if}
    </div>
  </div>
  {#if children}
    <div class="actions">{@render children()}</div>
  {/if}
</header>

<style>
  .page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
    padding: 1.25rem 1.5rem 0.5rem;
    flex-wrap: wrap;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    min-width: 0;
  }
  .icon-cell {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md);
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    flex-shrink: 0;
  }
  .titles {
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    letter-spacing: -0.005em;
    color: var(--color-foreground);
  }
  p {
    margin: 0.125rem 0 0;
    color: var(--color-muted-foreground);
    font-size: 0.8125rem;
  }
  .actions {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
</style>
