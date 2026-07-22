<!--
  Paper appearance settings — defaults applied to notes that don't override
  the field in their frontmatter.

  State is read from and written to `paperPrefs` (localStorage-backed store).
  Each row is a small radiogroup mirroring the LanguagePicker pill style. The
  per-note overrides (set from the editor toolbar) live in frontmatter and
  are resolved by `$lib/editor/paper.ts::resolvePaper` at render time.
-->

<script lang="ts">
  import {
    EDITOR_FONTS,
    EDITOR_LINE_HEIGHTS,
    EDITOR_PAGE_WIDTHS,
    EDITOR_SIZES,
    PAPER_TINTS,
    PAPER_VARIANTS,
    type PaperPrefs,
  } from "$lib/editor/paper";
  import { paperPrefs } from "$lib/stores/paper.svelte";
  import { m } from "$lib/i18n";

  type Token = keyof PaperPrefs;
  type Entry<K extends Token> = {
    value: PaperPrefs[K];
    label: () => string;
  };

  // Type-narrowing helpers: each option list is `Entry<K>[]` with the value
  // type matching the corresponding field, so the store's `set(key, value)`
  // is fully typed without casts.
  const VARIANTS: Entry<"paper">[] = PAPER_VARIANTS.map((v) => ({
    value: v,
    label: () => variantLabel(v),
  }));
  const TINTS: Entry<"paperTint">[] = PAPER_TINTS.map((v) => ({
    value: v,
    label: () => tintLabel(v),
  }));
  const FONTS: Entry<"editorFont">[] = EDITOR_FONTS.map((v) => ({
    value: v,
    label: () => fontLabel(v),
  }));
  const SIZES: Entry<"editorFontSize">[] = EDITOR_SIZES.map((v) => ({
    value: v,
    label: () => sizeLabel(v),
  }));
  const LINE_HEIGHTS: Entry<"editorLineHeight">[] = EDITOR_LINE_HEIGHTS.map((v) => ({
    value: v,
    label: () => lineHeightLabel(v),
  }));
  const PAGE_WIDTHS: Entry<"editorPageWidth">[] = EDITOR_PAGE_WIDTHS.map((v) => ({
    value: v,
    label: () => pageWidthLabel(v),
  }));

  function variantLabel(v: typeof PAPER_VARIANTS[number]): string {
    switch (v) {
      case "ruled": return m.settings_paper_variant_ruled();
      case "grid": return m.settings_paper_variant_grid();
      case "dot": return m.settings_paper_variant_dot();
      case "blank": return m.settings_paper_variant_blank();
    }
  }
  function tintLabel(v: typeof PAPER_TINTS[number]): string {
    switch (v) {
      case "white": return m.settings_paper_tint_white();
      case "warm": return m.settings_paper_tint_warm();
      case "sepia": return m.settings_paper_tint_sepia();
    }
  }
  function fontLabel(v: typeof EDITOR_FONTS[number]): string {
    switch (v) {
      case "serif": return m.settings_paper_font_serif();
      case "sans": return m.settings_paper_font_sans();
      case "mono": return m.settings_paper_font_mono();
    }
  }
  function sizeLabel(v: typeof EDITOR_SIZES[number]): string {
    switch (v) {
      case "sm": return m.settings_paper_size_sm();
      case "md": return m.settings_paper_size_md();
      case "lg": return m.settings_paper_size_lg();
    }
  }
  function lineHeightLabel(v: typeof EDITOR_LINE_HEIGHTS[number]): string {
    switch (v) {
      case "compact": return m.settings_paper_lineheight_compact();
      case "normal": return m.settings_paper_lineheight_normal();
      case "relaxed": return m.settings_paper_lineheight_relaxed();
    }
  }
  function pageWidthLabel(v: typeof EDITOR_PAGE_WIDTHS[number]): string {
    switch (v) {
      case "narrow": return m.settings_paper_pagewidth_narrow();
      case "normal": return m.settings_paper_pagewidth_normal();
      case "wide": return m.settings_paper_pagewidth_wide();
    }
  }
</script>

<div class="paper">
  <!-- Variant row: each option renders a mini preview tile so the user can
       tell at a glance what ruled/grid/dot/blank actually look like. -->
  <fieldset class="row">
    <legend>{m.settings_paper_field_paper()}</legend>
    <div class="tiles" role="radiogroup" aria-label={m.settings_paper_field_paper()}>
      {#each VARIANTS as opt (opt.value)}
        {@const active = paperPrefs.state.paper === opt.value}
        <button
          type="button"
          role="radio"
          aria-checked={active}
          class="tile"
          class:active
          onclick={() => paperPrefs.set("paper", opt.value)}
        >
          <span class="preview preview-{opt.value}" aria-hidden="true"></span>
          <span class="tile-label">{opt.label()}</span>
        </button>
      {/each}
    </div>
  </fieldset>

  <!-- Tint: three solid color chips. -->
  <fieldset class="row">
    <legend>{m.settings_paper_field_tint()}</legend>
    <div class="pills" role="radiogroup" aria-label={m.settings_paper_field_tint()}>
      {#each TINTS as opt (opt.value)}
        {@const active = paperPrefs.state.paperTint === opt.value}
        <button
          type="button"
          role="radio"
          aria-checked={active}
          class="pill"
          class:active
          onclick={() => paperPrefs.set("paperTint", opt.value)}
        >
          <span class="swatch swatch-{opt.value}" aria-hidden="true"></span>
          <span>{opt.label()}</span>
        </button>
      {/each}
    </div>
  </fieldset>

  <fieldset class="row">
    <legend>{m.settings_paper_field_font()}</legend>
    <div class="pills" role="radiogroup" aria-label={m.settings_paper_field_font()}>
      {#each FONTS as opt (opt.value)}
        {@const active = paperPrefs.state.editorFont === opt.value}
        <button
          type="button"
          role="radio"
          aria-checked={active}
          class="pill"
          class:active
          style:font-family={opt.value === "serif" ? '"Charter", Georgia, serif'
            : opt.value === "mono" ? "var(--font-mono)"
            : "var(--font-sans)"}
          onclick={() => paperPrefs.set("editorFont", opt.value)}
        >
          {opt.label()}
        </button>
      {/each}
    </div>
  </fieldset>

  <div class="row two-col">
    <fieldset class="row">
      <legend>{m.settings_paper_field_size()}</legend>
      <div class="pills" role="radiogroup" aria-label={m.settings_paper_field_size()}>
        {#each SIZES as opt (opt.value)}
          {@const active = paperPrefs.state.editorFontSize === opt.value}
          <button
            type="button"
            role="radio"
            aria-checked={active}
            class="pill"
            class:active
            onclick={() => paperPrefs.set("editorFontSize", opt.value)}
          >
            {opt.label()}
          </button>
        {/each}
      </div>
    </fieldset>

    <fieldset class="row">
      <legend>{m.settings_paper_field_lineheight()}</legend>
      <div class="pills" role="radiogroup" aria-label={m.settings_paper_field_lineheight()}>
        {#each LINE_HEIGHTS as opt (opt.value)}
          {@const active = paperPrefs.state.editorLineHeight === opt.value}
          <button
            type="button"
            role="radio"
            aria-checked={active}
            class="pill"
            class:active
            onclick={() => paperPrefs.set("editorLineHeight", opt.value)}
          >
            {opt.label()}
          </button>
        {/each}
      </div>
    </fieldset>
  </div>

  <fieldset class="row">
    <legend>{m.settings_paper_field_pagewidth()}</legend>
    <div class="pills" role="radiogroup" aria-label={m.settings_paper_field_pagewidth()}>
      {#each PAGE_WIDTHS as opt (opt.value)}
        {@const active = paperPrefs.state.editorPageWidth === opt.value}
        <button
          type="button"
          role="radio"
          aria-checked={active}
          class="pill"
          class:active
          onclick={() => paperPrefs.set("editorPageWidth", opt.value)}
        >
          {opt.label()}
        </button>
      {/each}
    </div>
  </fieldset>

  <div class="actions">
    <button type="button" class="reset" onclick={() => paperPrefs.reset()}>
      {m.settings_paper_reset()}
    </button>
  </div>
</div>

<style>
  .paper {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .row {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  legend {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-muted-foreground);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    margin-bottom: 0.375rem;
    padding: 0;
  }
  .two-col {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }
  @media (max-width: 640px) {
    .two-col {
      grid-template-columns: 1fr;
    }
  }

  /* Pill (matches LanguagePicker styling). */
  .pills {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    padding: 0.25rem 0.625rem;
    color: var(--color-foreground);
    font-family: inherit;
    font-size: 0.8125rem;
    cursor: pointer;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      color 120ms ease,
      transform 80ms ease;
  }
  .pill:hover {
    background: var(--color-muted);
    border-color: var(--color-accent-subtle);
  }
  .pill:active {
    transform: scale(0.98);
  }
  .pill.active {
    background: var(--color-accent);
    border-color: var(--color-accent);
    color: var(--color-accent-foreground);
  }

  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
  }
  .swatch-white { background: #ffffff; }
  .swatch-warm  { background: #fffbf1; }
  .swatch-sepia { background: #f4ecd8; }

  /* Tile preview row — only for the paper variant. */
  .tiles {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.5rem;
  }
  @media (max-width: 640px) {
    .tiles {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 0.375rem;
    background: var(--color-card);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: 0.5rem;
    cursor: pointer;
    text-align: left;
    transition:
      border-color 120ms ease,
      background 120ms ease,
      transform 80ms ease;
  }
  .tile:hover {
    border-color: var(--color-accent-subtle);
  }
  .tile:active {
    transform: scale(0.99);
  }
  .tile.active {
    border-color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 6%, var(--color-card));
  }
  .tile-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-foreground);
  }
  .preview {
    width: 100%;
    height: 36px;
    border-radius: var(--radius-sm);
    border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    background-color: var(--paper-white-bg, #fffbf1);
  }
  .preview-ruled {
    background-image: linear-gradient(to bottom, var(--canvas-gridline) 1px, transparent 1px);
    background-size: 100% 9px;
  }
  .preview-grid {
    background-image:
      linear-gradient(to right, var(--canvas-gridline) 1px, transparent 1px),
      linear-gradient(to bottom, var(--canvas-gridline) 1px, transparent 1px);
    background-size: 9px 100%, 100% 9px;
  }
  .preview-dot {
    background-image: radial-gradient(circle, var(--canvas-gridline) 1px, transparent 1.5px);
    background-size: 9px 9px;
  }
  .preview-blank {
    background-image: none;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }
  .reset {
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0.25rem 0.625rem;
    color: var(--color-muted-foreground);
    font-size: 0.75rem;
    cursor: pointer;
  }
  .reset:hover {
    background: var(--color-muted);
    color: var(--color-foreground);
  }
</style>