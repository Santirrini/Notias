/**
 * Locale store — reactive (Svelte 5 runes) global UI language for the SPA.
 *
 * Paraglide's compiled `m.*()` functions read the active locale through
 * `getLocale()` on every call (see `src/lib/paraglide/runtime.js`). When we
 * mutate `i18n.locale` we `overwriteGetLocale(() => i18n.locale)` so the next
 * `m.*()` evaluation returns the new locale's string. Components that
 * reference `i18n.locale` directly (or any `$derived` of it) re-render via
 * Svelte 5 reactivity. This is the entire reactivity model: no `$i18n`,
 * no Svelte 4 store contract — just runes.
 *
 * Persistence: `notias.locale` in `localStorage`, matching the project's
 * existing `notias.rail.collapsed` / `notias.sort` convention.
 *
 * SSR: Notias is a SPA (`ssr = false`), so guard for `typeof window`.
 */

import {
  baseLocale,
  locales,
  overwriteGetLocale,
  setLocale as paraglideSetLocale,
  type Locale,
} from "$lib/paraglide/runtime.js";

export const STORAGE_KEY = "notias.locale" as const;

/** Resolve a BCP-47 / language tag like "es-ES" / "ES_es" to one of our locales. */
function normalizeLocale(raw: string | null | undefined): Locale {
  if (!raw) return baseLocale as Locale;
  const lower = raw.toLowerCase();
  if (lower.startsWith("es")) return "es";
  // Default fallback: anything else → English.
  // (Add more branches here when more locales are added.)
  return baseLocale as Locale;
}

/** Read the persisted locale preference. Returns null if unset. */
function readPersisted(): Locale | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  const candidate = raw as Locale;
  return (locales as readonly string[]).includes(candidate) ? candidate : null;
}

/** Persist a locale choice. No-op outside the browser. */
function persistLocale(locale: Locale): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // Storage may be full or disabled — non-fatal.
  }
}

/** Update `<html lang>` for accessibility (screen readers). */
function syncHtmlLang(locale: Locale): void {
  if (typeof document === "undefined") return;
  document.documentElement.lang = locale;
}

class I18nStore {
  /** Current locale in the UI. Drives every reactive `m.*()` call. */
  locale: Locale = $state<Locale>(baseLocale as Locale);
  /** True once the initial detection chain (localStorage → OS) has resolved. */
  hydrated = $state<boolean>(false);

  constructor() {
    // Make Paraglide's compiled messages look at our rune-backed locale.
    // This is captured by closure: when `this.locale` changes, the returned
    // value changes too.
    overwriteGetLocale(() => this.locale);
  }

  /**
   * Initialize the locale at app boot.
   *
   * Order: 1) localStorage, 2) `navigator.language`, 3) base locale.
   * No OS IPC call here — we fall back to the browser navigator because
   * `+layout.svelte` runs before the Tauri runtime handshake completes
   * in some flows, and the navigator value is good enough for first-paint.
   * The OS detection helper (`detect.ts`) is exported separately so callers
   * that run after Tauri is ready (e.g. Settings) can prefer the OS API.
   */
  hydrate(): Locale {
    if (this.hydrated) return this.locale;

    const persisted = readPersisted();
    if (persisted) {
      this.set(persisted, { persist: false });
    } else if (typeof navigator !== "undefined") {
      this.set(normalizeLocale(navigator.language), { persist: false });
    } else {
      this.set(baseLocale as Locale, { persist: false });
    }
    this.hydrated = true;
    return this.locale;
  }

  /**
   * Set the locale, persist it, and update `<html lang>`.
   * Uses `reload: false` so we control re-renders through Svelte 5 runes
   * (a full page reload would defeat the purpose of an SPA).
   */
  set(next: Locale, options: { persist?: boolean } = { persist: true }): void {
    const safe = (locales as readonly string[]).includes(next)
      ? next
      : (baseLocale as Locale);
    this.locale = safe;
    syncHtmlLang(safe);
    if (options.persist) persistLocale(safe);
    // Inform Paraglide's runtime so any non-reactive call sites stay in sync.
    void paraglideSetLocale(safe, { reload: false });
  }

  /** Human-readable display label for a locale. Useful for the language picker. */
  labelFor(locale: Locale): string {
    switch (locale) {
      case "en":
        return "English";
      case "es":
        return "Español";
      default:
        return locale;
    }
  }
}

export const i18n = new I18nStore();
export type { Locale };
