<script lang="ts">
  import { onMount, onDestroy, untrack } from "svelte";
  import {
    Command,
    CommandGroup,
    CommandItem,
    CommandList,
  } from "$lib/components/ui/command/index.js";
  import {
    allCommands,
    type EditorCommand,
  } from "$lib/editor/commands";
  import type { MilkdownHandle, EditorStateSnapshot } from "$lib/editor/Milkdown.svelte";

  let {
    handle,
    body = "",
    title = "",
  }: {
    handle: MilkdownHandle | null;
    body?: string;
    title?: string;
  } = $props();

  let open = $state(false);
  let rect = $state<{ top: number; left: number } | null>(null);
  let query = $state("");

  type CmdRoot = {
    updateSelectedToIndex: (index: number) => void;
    updateSelectedByItem: (change: number) => void;
    updateSelectedByGroup: (change: 1 | -1) => void;
    getValidItems: () => HTMLElement[];
  };

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let cmdApi = $state<any>(null);

  const groups = $derived.by(() => {
    const inline = allCommands.filter((c) => c.group === "inline");
    const block = allCommands.filter((c) => c.group === "block");
    const ai = allCommands.filter((c) => c.group === "ai");
    return { inline, block, ai };
  });

  const filtered = $derived.by(() => {
    const q = query.toLowerCase();
    if (!q) return groups;
    const match = (c: EditorCommand) =>
      c.label.toLowerCase().includes(q) || c.id.includes(q);
    return {
      inline: groups.inline.filter(match),
      block: groups.block.filter(match),
      ai: groups.ai.filter(match),
    };
  });

  const flatList = $derived.by<EditorCommand[]>(() => [
    ...filtered.block,
    ...filtered.inline,
    ...filtered.ai,
  ]);

  const total = $derived(flatList.length);

  let unsubscribe: (() => void) | null = null;

  function maybeOpen(snap: EditorStateSnapshot) {
    if (!snap.lineIsEmpty && !query) {
      close();
      return;
    }
    if (!snap.rect) {
      close();
      return;
    }
    rect = { top: snap.rect.bottom + 6, left: snap.rect.left };
  }

  function deriveQuery(snap: EditorStateSnapshot): string | null {
    const text = snap.cursorLine;
    if (!text) return null;
    const m = text.match(/^\/(\S*)$/);
    if (m) return m[1] ?? "";
    return null;
  }

  function onState(snap: EditorStateSnapshot) {
    const q = deriveQuery(snap);
    if (q === null) {
      close();
      return;
    }
    query = q;
    open = true;
    maybeOpen(snap);
  }

  function close() {
    open = false;
    rect = null;
    query = "";
  }

  async function run(cmd: EditorCommand) {
    if (!handle) return;
    close();
    try {
      await handle.deleteLastChars(query.length + 1);
    } catch {
      /* ignore */
    }
    void handle.runCommand(cmd.id, { body, title });
    handle.focus();
  }

  function onWindowKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      close();
      return;
    }
    // Let bits-ui handle ArrowUp/Down/Enter if focused
  }

  onMount(() => {
    if (!handle) return;
    unsubscribe = handle.onStateChange((s) => {
      untrack(() => onState(s));
    });
  });

  onDestroy(() => {
    unsubscribe?.();
  });
</script>

{#if open && rect && total > 0}
  <div
    class="slash-menu"
    style:top="{rect.top}px"
    style:left="{rect.left}px"
    role="listbox"
    aria-label="Insert block"
  >
    <Command api={cmdApi}>
      <CommandList class="list">
        {#if filtered.block.length}
          <CommandGroup heading="Blocks">
            {#each filtered.block as cmd (cmd.id)}
              <CommandItem value={cmd.id} onSelect={() => run(cmd)}>
                <span class="cmd-icon">
                  <cmd.icon size={14} />
                </span>
                <div class="cmd-body">
                  <div class="lbl">{cmd.label}</div>
                  {#if cmd.description}<small>{cmd.description}</small>{/if}
                </div>
                {#if cmd.shortcut}<span class="kbd-hint">{cmd.shortcut}</span>{/if}
              </CommandItem>
            {/each}
          </CommandGroup>
        {/if}
        {#if filtered.inline.length}
          <CommandGroup heading="Inline">
            {#each filtered.inline as cmd (cmd.id)}
              <CommandItem value={cmd.id} onSelect={() => run(cmd)}>
                <span class="cmd-icon">
                  <cmd.icon size={14} />
                </span>
                <div class="cmd-body">
                  <div class="lbl">{cmd.label}</div>
                  {#if cmd.description}<small>{cmd.description}</small>{/if}
                </div>
                {#if cmd.shortcut}<span class="kbd-hint">{cmd.shortcut}</span>{/if}
              </CommandItem>
            {/each}
          </CommandGroup>
        {/if}
        {#if filtered.ai.length}
          <CommandGroup heading="AI">
            {#each filtered.ai as cmd (cmd.id)}
              <CommandItem value={cmd.id} onSelect={() => run(cmd)}>
                <span class="cmd-icon ai-icon">
                  <cmd.icon size={14} />
                </span>
                <div class="cmd-body">
                  <div class="lbl">{cmd.label}</div>
                  {#if cmd.description}<small>{cmd.description}</small>{/if}
                </div>
                {#if cmd.shortcut}<span class="kbd-hint">{cmd.shortcut}</span>{/if}
              </CommandItem>
            {/each}
          </CommandGroup>
        {/if}
      </CommandList>
    </Command>
  </div>
{/if}

<svelte:window onkeydown={onWindowKey} />

<style>
  .slash-menu {
    position: fixed;
    z-index: 45;
    width: 20rem;
    background: var(--color-popover);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.18);
    overflow: hidden;
    animation: sm-in 120ms ease;
  }
  @keyframes sm-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  :global(.slash-menu .list) {
    max-height: 22rem;
  }
  :global(.slash-menu [data-slot=command-item]) {
    display: flex;
    align-items: center;
    gap: 0.625rem;
    padding: 0.5rem 0.625rem;
  }
  :global(.slash-menu [data-slot=command-item][data-selected="true"]) {
    background: var(--color-muted) !important;
    box-shadow: inset 2px 0 0 var(--color-accent);
  }
  :global(.slash-menu [data-slot=command-item][data-selected="true"] .cmd-icon) {
    background: var(--color-accent-subtle);
    color: var(--color-accent);
  }
  .cmd-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    color: var(--color-muted-foreground);
    flex-shrink: 0;
  }
  .cmd-icon.ai-icon {
    background: var(--color-accent-subtle);
    color: var(--color-accent);
  }
  .cmd-body {
    flex: 1;
    min-width: 0;
  }
  .lbl {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-foreground);
  }
  small {
    display: block;
    color: var(--color-muted-foreground);
    font-size: 0.7rem;
    margin-top: 1px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kbd-hint {
    font-family: var(--font-mono);
    font-size: 0.6875rem;
    color: var(--color-muted-foreground);
    background: var(--color-muted);
    padding: 1px 6px;
    border-radius: 3px;
    flex-shrink: 0;
  }
</style>