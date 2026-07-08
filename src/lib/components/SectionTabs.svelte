<script lang="ts">
  import {
    Plus,
    Notebook,
    Pin,
    MoreHorizontal,
    Pencil,
    Trash2,
    PinOff,
  } from "@lucide/svelte";
  import { sections } from "$lib/stores/sections.svelte";
  import { cn } from "$lib/utils.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import { tick } from "svelte";

  const PALETTE = [
    "#0078d4",
    "#107c10",
    "#d13438",
    "#ffb900",
    "#5c2d91",
    "#008272",
    "#ca5010",
  ];

  let creating = $state(false);
  let draft = $state("");
  let draftInputRef = $state<HTMLElement | null>(null);
  let renaming = $state<string | null>(null);
  let renameDraft = $state("");

  function pickColor(index: number): string {
    return PALETTE[index % PALETTE.length] ?? "#0078d4";
  }

  async function start() {
    creating = true;
    draft = "";
    await tick();
    draftInputRef?.focus();
  }

  function commit() {
    const name = draft.trim();
    if (!name) {
      creating = false;
      return;
    }
    const color = pickColor(sections.state.sections.length);
    sections.createSection(name, color);
    creating = false;
  }

  function cancel() {
    creating = false;
    draft = "";
  }

  async function startRename(name: string) {
    renaming = name;
    renameDraft = name;
    await tick();
    const el = document.getElementById(`rename-${name}`) as HTMLInputElement | null;
    el?.focus();
    el?.select();
  }

  function commitRename() {
    if (!renaming) return;
    sections.renameSection(renaming, renameDraft);
    renaming = null;
    renameDraft = "";
  }

  function cancelRename() {
    renaming = null;
    renameDraft = "";
  }

  function remove(name: string) {
    if (name === "General") return;
    sections.removeSection(name);
  }
</script>

<Tooltip.Provider delayDuration={250}>
  <aside class="sections" aria-label="Sections">
    <header>
      <Notebook size={14} />
      <span class="notebook-name">Notebook</span>
    </header>

    <div class="tabs" role="tablist">
      {#each sections.state.sections as s (s.name)}
        {@const active = s.name === sections.state.active}
        {@const isRenaming = renaming === s.name}
        <div class={cn("tab", active && "active")} style:--tab-color={s.color}>
          {#if isRenaming}
            <span class="bar" aria-hidden="true"></span>
            <input
              id={`rename-${s.name}`}
              class="rename"
              bind:value={renameDraft}
              onkeydown={(e: KeyboardEvent) => {
                if (e.key === "Enter") commitRename();
                if (e.key === "Escape") cancelRename();
              }}
              onblur={commitRename}
            />
          {:else}
            <button
              type="button"
              role="tab"
              aria-selected={active}
              class="tab-btn"
              onclick={() => sections.setActive(s.name)}
              title={s.name}
            >
              <span class="bar" aria-hidden="true"></span>
              <span class="name">
                {#if s.pinned}<Pin size={11} class="pin" />{/if}
                {s.name}
              </span>
            </button>
            {#if s.name !== "General"}
              <DropdownMenu.Root>
                <DropdownMenu.Trigger>
                  {#snippet child({ props })}
                    <button
                      type="button"
                      class="more"
                      aria-label={`Actions for ${s.name}`}
                      {...props}
                    >
                      <MoreHorizontal size={12} />
                    </button>
                  {/snippet}
                </DropdownMenu.Trigger>
                <DropdownMenu.Content align="end" sideOffset={4} class="w-44">
                  <DropdownMenu.Item onclick={() => startRename(s.name)}>
                    <Pencil size={12} /> Rename
                  </DropdownMenu.Item>
                  <DropdownMenu.Item onclick={() => sections.togglePin(s.name)}>
                    {#if s.pinned}
                      <PinOff size={12} /> Unpin
                    {:else}
                      <Pin size={12} /> Pin to top
                    {/if}
                  </DropdownMenu.Item>
                  <DropdownMenu.Separator />
                  <DropdownMenu.Item variant="destructive" onclick={() => remove(s.name)}>
                    <Trash2 size={12} /> Delete section
                  </DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Root>
            {/if}
          {/if}
        </div>
      {/each}
    </div>

    <div class="create">
      {#if creating}
        <Input
          bind:ref={draftInputRef}
          bind:value={draft}
          placeholder="Section name"
          aria-label="New section name"
          onkeydown={(e: KeyboardEvent) => {
            if (e.key === "Enter") commit();
            if (e.key === "Escape") cancel();
          }}
          onblur={commit}
        />
      {:else}
        <Button variant="ghost" size="sm" onclick={start} class="w-full justify-start">
          <Plus size={14} /> New section
        </Button>
      {/if}
    </div>
  </aside>
</Tooltip.Provider>

<style>
  .sections {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-subtle);
    border-right: 1px solid var(--color-border);
    padding: 0.5rem 0;
  }
  header {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 0.875rem;
    color: var(--color-muted-foreground);
  }
  .notebook-name {
    font-weight: 600;
    font-size: 0.75rem;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--color-foreground);
  }

  .tabs {
    display: flex;
    flex-direction: column;
    gap: 1px;
    margin-top: 0.25rem;
    overflow-y: auto;
    flex: 1;
  }
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0;
    border-left: 3px solid transparent;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .tab.active {
    background: var(--color-background);
    border-left-color: var(--tab-color);
  }
  .tab:hover {
    background: var(--color-muted);
  }
  .tab.active:hover {
    background: var(--color-background);
  }
  .tab:hover .more {
    opacity: 1;
  }

  .tab-btn {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.5rem 0.5rem 0.5rem;
    background: transparent;
    border: 0;
    color: var(--color-foreground);
    font-size: 0.875rem;
    text-align: left;
    font-family: inherit;
    cursor: pointer;
    min-width: 0;
  }
  .tab.active .tab-btn {
    font-weight: 500;
  }
  .bar {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--tab-color);
    flex-shrink: 0;
  }
  .name {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  :global(.pin) {
    color: var(--color-muted-foreground);
  }

  .more {
    width: 22px;
    height: 22px;
    border: 0;
    background: transparent;
    color: var(--color-muted-foreground);
    border-radius: var(--radius-sm);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 100ms ease, background 100ms ease, color 100ms ease;
    margin-right: 0.375rem;
    flex-shrink: 0;
  }
  .more:hover {
    background: var(--color-background);
    color: var(--color-foreground);
  }
  .tab.active .more {
    opacity: 1;
  }

  .rename {
    flex: 1;
    height: 28px;
    margin: 4px 8px;
    padding: 0 6px;
    border: 1px solid var(--color-ring);
    border-radius: var(--radius-sm);
    background: var(--color-background);
    color: var(--color-foreground);
    font-size: 0.875rem;
    font-family: inherit;
    outline: none;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-ring) 18%, transparent);
  }

  .create {
    margin-top: 0.5rem;
    padding: 0.5rem 0.625rem;
    border-top: 1px solid var(--color-border);
  }
</style>