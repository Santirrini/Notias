<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import * as Tooltip from "$lib/components/ui/tooltip/index.js";
  import * as DropdownMenu from "$lib/components/ui/dropdown-menu/index.js";
  import { Sparkles, MoreHorizontal, Mic, FileText, PenTool } from "@lucide/svelte";
  import {
    allCommands,
    type EditorCommand,
  } from "$lib/editor/commands";
  import type { MilkdownHandle } from "$lib/editor/Milkdown.svelte";
  import {
    EDITOR_FONTS,
    EDITOR_LINE_HEIGHTS,
    EDITOR_PAGE_WIDTHS,
    EDITOR_SIZES,
    PAPER_TINTS,
    PAPER_VARIANTS,
    type PaperPrefs,
  } from "$lib/editor/paper";
  import { m } from "$lib/i18n";

  let {
    handle,
    body = "",
    title = "",
    onRecordClick,
    onOpenDrawing,
    paperOverrides = new Set<keyof PaperPrefs>(),
    onPaperChange,
    onPaperClear,
  }: {
    handle: MilkdownHandle | null;
    body?: string;
    title?: string;
    onRecordClick?: () => void;
    /** Open the Excalidraw modal in "new drawing" mode. */
    onOpenDrawing?: () => void;
    /** Keys that the current note overrides (vs. inherits from global). */
    paperOverrides?: Set<keyof PaperPrefs>;
    /** Apply or update a single field override for this note. */
    onPaperChange?: (patch: Partial<PaperPrefs>) => void;
    /** Remove every per-note override and fall back to global defaults. */
    onPaperClear?: () => void;
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

  function setPaper<K extends keyof PaperPrefs>(key: K, value: PaperPrefs[K]) {
    onPaperChange?.({ [key]: value } as Partial<PaperPrefs>);
  }

  const variants = PAPER_VARIANTS;
  const tints = PAPER_TINTS;
  const fonts = EDITOR_FONTS;
  const sizes = EDITOR_SIZES;
  const lineHeights = EDITOR_LINE_HEIGHTS;
  const pageWidths = EDITOR_PAGE_WIDTHS;

  function variantLabel(v: typeof variants[number]) {
    return ({
      ruled: m.settings_paper_variant_ruled(),
      grid: m.settings_paper_variant_grid(),
      dot: m.settings_paper_variant_dot(),
      blank: m.settings_paper_variant_blank(),
    } as const)[v];
  }
  function tintLabel(v: typeof tints[number]) {
    return ({
      white: m.settings_paper_tint_white(),
      warm: m.settings_paper_tint_warm(),
      sepia: m.settings_paper_tint_sepia(),
    } as const)[v];
  }
  function fontLabel(v: typeof fonts[number]) {
    return ({
      serif: m.settings_paper_font_serif(),
      sans: m.settings_paper_font_sans(),
      mono: m.settings_paper_font_mono(),
    } as const)[v];
  }
  function sizeLabel(v: typeof sizes[number]) {
    return ({
      sm: m.settings_paper_size_sm(),
      md: m.settings_paper_size_md(),
      lg: m.settings_paper_size_lg(),
    } as const)[v];
  }
  function lineHeightLabel(v: typeof lineHeights[number]) {
    return ({
      compact: m.settings_paper_lineheight_compact(),
      normal: m.settings_paper_lineheight_normal(),
      relaxed: m.settings_paper_lineheight_relaxed(),
    } as const)[v];
  }
  function pageWidthLabel(v: typeof pageWidths[number]) {
    return ({
      narrow: m.settings_paper_pagewidth_narrow(),
      normal: m.settings_paper_pagewidth_normal(),
      wide: m.settings_paper_pagewidth_wide(),
    } as const)[v];
  }

  const hasOverrides = $derived(paperOverrides.size > 0);
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

    {#if onOpenDrawing}
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <Button
              variant="ghost"
              size="icon-sm"
              onclick={() => onOpenDrawing?.()}
              aria-label={m.drawing_toolbar_label()}
              {...props}
            >
              <PenTool size={14} />
            </Button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>
          {m.drawing_toolbar_label()}
        </Tooltip.Content>
      </Tooltip.Root>
    {/if}

    <DropdownMenu.Root>
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <DropdownMenu.Trigger>
              {#snippet child({ props: dmProps })}
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={m.edit_toolbar_more_blocks()}
                  {...dmProps}
                  {...props}
                >
                  <MoreHorizontal size={14} />
                </Button>
              {/snippet}
            </DropdownMenu.Trigger>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>{m.edit_toolbar_more_blocks()}</Tooltip.Content>
      </Tooltip.Root>
      <DropdownMenu.Content align="start" sideOffset={6} class="w-56">
        <DropdownMenu.Label>{m.edit_toolbar_more_blocks_heading()}</DropdownMenu.Label>
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
            aria-label={m.edit_toolbar_suggest()}
            {...props}
          >
            <Sparkles size={14} />
            <span>{m.edit_toolbar_suggest()}</span>
          </Button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content sideOffset={4}>{m.edit_toolbar_suggest_tooltip()} · <kbd>Ctrl J</kbd></Tooltip.Content>
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
                  aria-label={m.edit_toolbar_more_ai_aria()}
                  {...dmProps}
                  {...props}
                >
                  <Sparkles size={14} />
                </Button>
              {/snippet}
            </DropdownMenu.Trigger>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>{m.edit_toolbar_more_ai_tooltip()}</Tooltip.Content>
      </Tooltip.Root>
      <DropdownMenu.Content align="start" sideOffset={6} class="w-64">
        <DropdownMenu.Label>{m.edit_toolbar_more_ai_heading()}</DropdownMenu.Label>
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
      <DropdownMenu.Root>
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <DropdownMenu.Trigger>
                {#snippet child({ props: dmProps })}
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={m.paper_toolbar_label()}
                    {...dmProps}
                    {...props}
                  >
                    <FileText size={14} />
                    {#if hasOverrides}<span class="dot" aria-hidden="true"></span>{/if}
                  </Button>
                {/snippet}
              </DropdownMenu.Trigger>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content sideOffset={4}>
            {m.paper_toolbar_label()} · {hasOverrides ? m.paper_toolbar_using_override() : m.paper_toolbar_using_global()}
          </Tooltip.Content>
        </Tooltip.Root>
        <DropdownMenu.Content align="start" sideOffset={6} class="paper-menu w-72">
          <DropdownMenu.Label class="paper-menu-head">
            <span>{m.paper_toolbar_label()}</span>
            {#if hasOverrides}
              <button type="button" class="paper-clear" onclick={() => onPaperClear?.()}>
                {m.paper_toolbar_clear_override()}
              </button>
            {/if}
          </DropdownMenu.Label>

          <DropdownMenu.Group>
            <DropdownMenu.Label class="paper-section">{m.settings_paper_field_paper()}</DropdownMenu.Label>
            <div class="paper-row">
              {#each variants as v (v)}
                <DropdownMenu.Item
                  class="paper-chip"
                  data-active={paperOverrides.has("paper") ? undefined : undefined}
                  onclick={() => setPaper("paper", v)}
                >
                  <span class="preview preview-{v}" aria-hidden="true"></span>
                  <span>{variantLabel(v)}</span>
                </DropdownMenu.Item>
              {/each}
            </div>
          </DropdownMenu.Group>

          <DropdownMenu.Separator />
          <DropdownMenu.Group>
            <DropdownMenu.Label class="paper-section">{m.settings_paper_field_tint()}</DropdownMenu.Label>
            <div class="paper-row">
              {#each tints as t (t)}
                <DropdownMenu.Item class="paper-chip" onclick={() => setPaper("paperTint", t)}>
                  <span class="swatch swatch-{t}" aria-hidden="true"></span>
                  <span>{tintLabel(t)}</span>
                </DropdownMenu.Item>
              {/each}
            </div>
          </DropdownMenu.Group>

          <DropdownMenu.Separator />
          <DropdownMenu.Group>
            <DropdownMenu.Label class="paper-section">{m.settings_paper_field_font()}</DropdownMenu.Label>
            <div class="paper-row">
              {#each fonts as f (f)}
                <DropdownMenu.Item class="paper-chip" onclick={() => setPaper("editorFont", f)}>
                  <span>{fontLabel(f)}</span>
                </DropdownMenu.Item>
              {/each}
            </div>
          </DropdownMenu.Group>

          <DropdownMenu.Separator />
          <div class="paper-two-col">
            <DropdownMenu.Group>
              <DropdownMenu.Label class="paper-section">{m.settings_paper_field_size()}</DropdownMenu.Label>
              <div class="paper-row">
                {#each sizes as s (s)}
                  <DropdownMenu.Item class="paper-chip" onclick={() => setPaper("editorFontSize", s)}>
                    <span>{sizeLabel(s)}</span>
                  </DropdownMenu.Item>
                {/each}
              </div>
            </DropdownMenu.Group>
            <DropdownMenu.Group>
              <DropdownMenu.Label class="paper-section">{m.settings_paper_field_lineheight()}</DropdownMenu.Label>
              <div class="paper-row">
                {#each lineHeights as lh (lh)}
                  <DropdownMenu.Item class="paper-chip" onclick={() => setPaper("editorLineHeight", lh)}>
                    <span>{lineHeightLabel(lh)}</span>
                  </DropdownMenu.Item>
                {/each}
              </div>
            </DropdownMenu.Group>
          </div>

          <DropdownMenu.Separator />
          <DropdownMenu.Group>
            <DropdownMenu.Label class="paper-section">{m.settings_paper_field_pagewidth()}</DropdownMenu.Label>
            <div class="paper-row">
              {#each pageWidths as pw (pw)}
                <DropdownMenu.Item class="paper-chip" onclick={() => setPaper("editorPageWidth", pw)}>
                  <span>{pageWidthLabel(pw)}</span>
                </DropdownMenu.Item>
              {/each}
            </div>
          </DropdownMenu.Group>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <Button
              variant="ghost"
              size="icon-sm"
              onclick={onRecordClick}
              aria-label={m.edit_toolbar_record_aria()}
              {...props}
            >
              <Mic size={14} />
            </Button>
          {/snippet}
        </Tooltip.Trigger>
        <Tooltip.Content sideOffset={4}>{m.edit_toolbar_record_tooltip()}</Tooltip.Content>
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

  /* Paper dropdown — sits at the end of the toolbar. */
  :global(.toolbar) .dot {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--color-accent);
  }
  :global(.toolbar) button[aria-label]:has(.dot) {
    position: relative;
  }
  :global(.paper-menu) {
    max-height: 70vh;
    overflow-y: auto;
  }
  :global(.paper-menu-head) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  :global(.paper-clear) {
    background: transparent;
    border: 0;
    color: var(--color-accent);
    font-size: 0.7rem;
    cursor: pointer;
    padding: 0;
  }
  :global(.paper-clear:hover) {
    text-decoration: underline;
  }
  :global(.paper-section) {
    font-size: 0.65rem !important;
    font-weight: 600 !important;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-foreground) !important;
    margin-top: 0.25rem;
  }
  :global(.paper-row) {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    padding: 0 0.25rem;
  }
  :global(.paper-chip) {
    border-radius: var(--radius-sm);
    padding: 0.25rem 0.5rem !important;
    font-size: 0.78rem;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }
  :global(.paper-two-col) {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  :global(.paper-two-col .paper-section) {
    margin-top: 0;
  }
  :global(.paper-menu .preview) {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    background-color: var(--paper-white-bg, #fffbf1);
    flex-shrink: 0;
  }
  :global(.paper-menu .preview-ruled) {
    background-image: linear-gradient(to bottom, var(--canvas-gridline) 1px, transparent 1px);
    background-size: 100% 5px;
  }
  :global(.paper-menu .preview-grid) {
    background-image:
      linear-gradient(to right, var(--canvas-gridline) 1px, transparent 1px),
      linear-gradient(to bottom, var(--canvas-gridline) 1px, transparent 1px);
    background-size: 5px 100%, 100% 5px;
  }
  :global(.paper-menu .preview-dot) {
    background-image: radial-gradient(circle, var(--canvas-gridline) 1px, transparent 1.5px);
    background-size: 5px 5px;
  }
  :global(.paper-menu .preview-blank) {
    background-image: none;
  }
  :global(.paper-menu .swatch) {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    flex-shrink: 0;
  }
  :global(.paper-menu .swatch-white) { background: #ffffff; }
  :global(.paper-menu .swatch-warm)  { background: #fffbf1; }
  :global(.paper-menu .swatch-sepia) { background: #f4ecd8; }
</style>