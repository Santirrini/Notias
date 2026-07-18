<!--
  Language picker.

  UI surface for switching the SPA's active language. Calls `i18n.set()`
  which is the only place that persists to `localStorage.notias.locale`
  and keeps `<html lang>` and `document.documentElement.lang` in sync.

  The component itself accepts NO props — it reads `i18n.locale`
  reactively, so when the user picks "es" every part of the app that
  calls `m.xxx()` updates in place.

  Adding more languages later:
    1. Add the locale to project.inlang/settings.json `locales`.
    2. Add a row below for each new option.
    3. Add a new <entry> mapping in `i18n.labelFor()`.
-->

<script lang="ts">
  import { Globe } from "@lucide/svelte";
  import { i18n } from "../locale.svelte";
  import { m } from "../index";
  import { isLocale } from "$lib/paraglide/runtime.js";
  import { toast } from "svelte-sonner";
  import type { LocalizedString } from "$lib/paraglide/runtime.js";

  /**
   * Source of truth for the option list. Each entry carries the actual
   * function we'll call from the markup so TypeScript can narrow the
   * key (no string-indexing into the Paraglide namespace).
   */
  type PickerLocale = "en" | "es";
  const LOCALES: ReadonlyArray<{
    code: PickerLocale;
    /** Returns a fresh LocalizedString for current locale. */
    label: () => LocalizedString;
  }> = [
    { code: "en", label: () => m.settings_language_english() },
    { code: "es", label: () => m.settings_language_spanish() },
  ] as const;

  function pick(code: PickerLocale) {
    if (!isLocale(code)) return;
    // Capture pre-switch label so the toast reflects what the user just
    // chose (without depending on the reactive store mid-event).
    const labelBefore = i18n.labelFor(code);
    i18n.set(code);
    toast.success(m.settings_language_saved({ lang: labelBefore }));
  }
</script>

<div class="picker" role="radiogroup" aria-label={m.settings_language_label()}>
  <span class="globe" aria-hidden="true"><Globe size={14} /></span>
  {#each LOCALES as opt (opt.code)}
    {@const active = i18n.locale === opt.code}
    <button
      type="button"
      role="radio"
      aria-checked={active}
      class="pill"
      class:active
      onclick={() => pick(opt.code)}
    >
      <span class="code">{opt.code.toUpperCase()}</span>
      <span class="label">{opt.label()}</span>
    </button>
  {/each}
</div>

<style>
  .picker {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    flex-wrap: wrap;
  }
  .globe {
    display: inline-flex;
    color: var(--color-muted-foreground);
    margin-right: 0.25rem;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: transparent;
    border: 1px solid var(--color-border);
    border-radius: 999px;
    padding: 0.25rem 0.625rem 0.25rem 0.4rem;
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
  .pill.active:hover {
    background: var(--color-accent-pressed, var(--color-accent));
  }
  .code {
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: 0.6875rem;
    letter-spacing: 0.06em;
    font-weight: 600;
    background: color-mix(in srgb, var(--color-background) 60%, transparent);
    padding: 0 4px;
    border-radius: 3px;
  }
  .pill.active .code {
    background: color-mix(in srgb, var(--color-background) 18%, transparent);
  }
  .label {
    font-weight: 500;
  }
</style>
