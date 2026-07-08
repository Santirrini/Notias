<script lang="ts">
  import { page } from "$app/state";
  import {
    NotebookText,
    MessageSquare,
    Calendar,
    ListTodo,
    GraduationCap,
    Settings,
    ChevronsLeft,
    ChevronsRight,
    type Icon as IconType,
  } from "@lucide/svelte";
  import { cn } from "$lib/utils.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";

  type Item = {
    href: string;
    label: string;
    icon: typeof IconType;
    match?: (path: string) => boolean;
  };

  const items: Item[] = [
    { href: "/notes", label: "Notes", icon: NotebookText, match: (p) => p.startsWith("/notes") },
    { href: "/chat", label: "Chat", icon: MessageSquare },
    { href: "/calendar", label: "Calendar", icon: Calendar },
    { href: "/tasks", label: "Tasks", icon: ListTodo },
    { href: "/study", label: "Study", icon: GraduationCap },
    { href: "/settings", label: "Settings", icon: Settings },
  ];

  let {
    collapsed = $bindable(false),
    ontoggle,
  }: {
    collapsed?: boolean;
    ontoggle?: () => void;
  } = $props();

  const pathname = $derived(page.url?.pathname ?? "/");
  function isActive(item: Item): boolean {
    return item.match ? item.match(pathname) : pathname === item.href || pathname.startsWith(item.href + "/");
  }

  function toggle() {
    collapsed = !collapsed;
    ontoggle?.();
  }
</script>

<Tooltip.Provider delayDuration={120} skipDelayDuration={300}>
  <aside class={cn("rail", collapsed && "collapsed")} aria-label="Primary">
    <a class="brand" href="/notes" aria-label="Notias home">
      <span class="logo" aria-hidden="true">N</span>
      {#if !collapsed}<span class="title">Notias</span>{/if}
    </a>

    <nav>
      {#each items as it (it.href)}
        {@const active = isActive(it)}
        {#if collapsed}
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <a
                  href={it.href}
                  class={cn("item", active && "active")}
                  aria-current={active ? "page" : undefined}
                  {...props}
                >
                  <span class="item-icon">
                    <it.icon size={20} aria-hidden="true" />
                  </span>
                </a>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content side="right" sideOffset={10}>
              {it.label}
            </Tooltip.Content>
          </Tooltip.Root>
        {:else}
          <a
            href={it.href}
            class={cn("item", active && "active")}
            aria-current={active ? "page" : undefined}
            aria-label={it.label}
          >
            <span class="item-icon">
              <it.icon size={20} aria-hidden="true" />
            </span>
            <span class="item-label">{it.label}</span>
          </a>
        {/if}
      {/each}
    </nav>

    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <button
            class="toggle"
            type="button"
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            onclick={toggle}
            {...props}
          >
            {#if collapsed}
              <ChevronsRight size={16} />
            {:else}
              <ChevronsLeft size={16} />
            {/if}
          </button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content side="right" sideOffset={10}>
        {collapsed ? "Expand" : "Collapse"}
      </Tooltip.Content>
    </Tooltip.Root>
  </aside>
</Tooltip.Provider>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 220px;
    border-right: 1px solid var(--color-border);
    background: var(--color-background);
    transition: width 220ms cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
  }
  .rail.collapsed {
    width: 56px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.875rem;
    border-bottom: 1px solid var(--color-border);
    text-decoration: none;
    color: inherit;
    cursor: pointer;
    transition: background 120ms ease;
  }
  .brand:hover {
    background: var(--color-muted);
  }
  .logo {
    width: 28px;
    height: 28px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, var(--color-accent), var(--color-accent-pressed));
    color: var(--color-accent-foreground);
    border-radius: var(--radius-sm);
    font-weight: 700;
    font-size: 0.875rem;
    flex-shrink: 0;
    box-shadow: 0 1px 2px color-mix(in srgb, var(--color-accent) 30%, transparent);
  }
  .title {
    font-weight: 600;
    font-size: 0.9375rem;
    letter-spacing: -0.01em;
  }

  nav {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0.5rem;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.625rem;
    border-radius: var(--radius-sm);
    color: var(--color-muted-foreground);
    text-decoration: none;
    font-size: 0.875rem;
    line-height: 1;
    transition:
      background 120ms ease,
      color 120ms ease,
      transform 80ms ease;
    overflow: hidden;
  }
  .item:hover {
    background: var(--color-muted);
    color: var(--color-foreground);
  }
  .item:active {
    transform: scale(0.98);
  }
  .item.active {
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    font-weight: 500;
  }
  .item.active::before {
    content: "";
    position: absolute;
    left: 0;
    top: 6px;
    bottom: 6px;
    width: 3px;
    background: var(--color-accent);
    border-radius: 0 3px 3px 0;
  }
  .item.active :global(svg) {
    color: var(--color-accent);
  }

  .item-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }
  .item-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rail.collapsed .item {
    justify-content: center;
    padding: 0.5rem;
  }
  .rail.collapsed .item.active::before {
    top: 4px;
    bottom: 4px;
  }

  .toggle {
    margin: 0.4rem;
    padding: 0.4rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-muted-foreground);
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }
  .toggle:hover {
    background: var(--color-muted);
    color: var(--color-foreground);
    border-color: var(--color-accent-subtle);
  }
  .toggle:active {
    transform: scale(0.96);
  }
  .rail.collapsed .toggle :global(svg) {
    transform: scaleX(-1);
  }
</style>