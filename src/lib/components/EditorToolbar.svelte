<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { Sparkles, MoreHorizontal, Mic } from "@lucide/svelte";
  import {
    allCommands,
    type EditorCommand,
  } from "$lib/editor/commands";
  import type { MilkdownHandle } from "$lib/editor/Milkdown.svelte";

  let {
    handle,
    body = "",
    title = "",
    onRecordClick,
  }: {
    handle: MilkdownHandle | null;
    body?: string;
    title?: string;
    onRecordClick?: () => void;
  } = $props();

  const inlineCommands = allCommands.filter((c) => c.group === "inline");
  const blockCommands = allCommands.filter((c) => c.group === "block");
  const aiCommands = allCommands.filter((c) => c.group === "ai");

  function run(cmd: EditorCommand) {
    if (!handle) return;
    void handle.runCommand(cmd.id, { body, title });
  }

  function suggest() {
    if (!handle) return;
    void handle.triggerSuggest();
  }
</script>

<Tooltip.Provider delayDuration={250}>
  <div class="toolbar" role="toolbar" aria-label="Note formatting">
    {#each inlineCommands as cmd (cmd.id)}
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <Button
              variant="ghost"
              size="icon-sm"
              onclick={() => run(cmd)}
              aria-label={cmd.label}
              {...props}
            >
              <cmd.icon size={14} />
            </Button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>
          {cmd.label}{#if cmd.shortcut} · <kbd>{cmd.shortcut}</kbd>{/if}
        </Tooltip.Content>
      </Tooltip.Root>
    {/each}

    <Separator orientation="vertical" class="sep" />

    {#each blockCommands.slice(0, 5) as cmd (cmd.id)}
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <Button
              variant="ghost"
              size="icon-sm"
              onclick={() => run(cmd)}
              aria-label={cmd.label}
              {...props}
            >
              <cmd.icon size={14} />
            </Button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>
          {cmd.label}{#if cmd.shortcut} · <kbd>{cmd.shortcut}</kbd>{/if}
        </Tooltip.Content>
      </Tooltip.Root>
    {/each}

    <DropdownMenu.Root>
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <DropdownMenu.Trigger>
              {#snippet child({ props: dmProps })}
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="More blocks"
                  {...dmProps}
                  {...props}
                >
                  <MoreHorizontal size={14} />
                </Button>
              {/snippet}
            </DropdownMenu.Trigger>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>More blocks</Tooltip.Content>
      </Tooltip.Root>
      <DropdownMenu.Content align="start" sideOffset={6} class="w-56">
        <DropdownMenu.Label>Blocks</DropdownMenu.Label>
        {#each blockCommands.slice(5) as cmd (cmd.id)}
          <DropdownMenu.Item onclick={() => run(cmd)}>
            <cmd.icon size={14} />
            <span>{cmd.label}</span>
          </DropdownMenu.Item>
        {/each}
      </DropdownMenu.Content>
    </DropdownMenu.Root>

    <Separator orientation="vertical" class="sep" />

    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <Button
            variant="ghost"
            size="sm"
            class="ai"
            onclick={suggest}
            aria-label="AI: continue writing"
            {...props}
          >
            <Sparkles size={14} />
            <span>Suggest</span>
          </Button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content sideOffset={4}>AI: continue from cursor · <kbd>Ctrl J</kbd></Tooltip.Content>
    </Tooltip.Root>

    <DropdownMenu.Root>
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <DropdownMenu.Trigger>
              {#snippet child({ props: dmProps })}
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="More AI actions"
                  {...dmProps}
                  {...props}
                >
                  <Sparkles size={14} />
                </Button>
              {/snippet}
            </DropdownMenu.Trigger>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>More AI</Tooltip.Content>
      </Tooltip.Root>
      <DropdownMenu.Content align="start" sideOffset={6} class="w-64">
        <DropdownMenu.Label>AI</DropdownMenu.Label>
        {#each aiCommands.slice(1) as cmd (cmd.id)}
          <DropdownMenu.Item onclick={() => run(cmd)}>
            <span class="ai-icon"><cmd.icon size={14} /></span>
            <div class="ai-text">
              <div>{cmd.label}</div>
              {#if cmd.description}<small>{cmd.description}</small>{/if}
            </div>
          </DropdownMenu.Item>
        {/each}
      </DropdownMenu.Content>
    </DropdownMenu.Root>

    {#if onRecordClick}
      <Separator orientation="vertical" class="sep" />
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <Button
              variant="ghost"
              size="icon-sm"
              onclick={onRecordClick}
              aria-label="Record audio"
              {...props}
            >
              <Mic size={14} />
            </Button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>Record audio (transcribe via AI)</Tooltip.Content>
      </Tooltip.Root>
    {/if}
  </div>
</Tooltip.Provider>

<style>
  .toolbar {
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
    padding: 0.25rem;
    background: var(--color-popover);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
    flex-wrap: wrap;
  }
  :global(.toolbar .ai) {
    font-size: 0.75rem;
    padding-inline: 0.5rem;
  }
  :global(.sep) {
    height: 1.25rem;
    margin: 0 0.15rem;
  }
  :global(.toolbar kbd) {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.65rem;
    background: var(--color-muted);
    padding: 0 4px;
    border-radius: 3px;
    border: 1px solid var(--color-border);
    color: var(--color-muted-foreground);
  }
  .ai-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-sm);
    background: var(--color-accent-subtle);
    color: var(--color-accent);
    flex-shrink: 0;
  }
  .ai-text {
    flex: 1;
    min-width: 0;
  }
  .ai-text small {
    display: block;
    color: var(--color-muted-foreground);
    font-size: 0.7rem;
    margin-top: 1px;
  }
</style>