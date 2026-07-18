/**
 * Locale detection — resolve the initial UI language from the host OS,
 * the browser, or a previously persisted preference.
 *
 * Strategy (priority):
 *   1. localStorage (`notias.locale`) — user has chosen before
 *   2. OS locale via `tauri-plugin-os::locale()` — BCP-47 tag like "es-ES"
 *      (preferred when running inside Tauri; falls through if not)
 *   3. `navigator.language` — pure-browser fallback (e.g. `pnpm dev`)
 *   4. base locale ("en") — last resort
 *
 * The OS branch is async (Tauri IPC). Callers should `await` it from
 * `+layout.svelte`'s `onMount`.
 */

import type { Locale } from "$lib/paraglide/runtime.js";
import { STORAGE_KEY } from "./locale.svelte";
import { locales as supported, baseLocale } from "$lib/paraglide/runtime.js";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

function readPersisted(): Locale | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  return (supported as readonly string[]).includes(raw) ? (raw as Locale) : null;
}

function normalize(raw: string | null | undefined): Locale {
  if (!raw) return baseLocale as Locale;
  const lower = raw.toLowerCase();
  if (lower.startsWith("es")) return "es";
  // Add more branches here when new locales are introduced.
  return baseLocale as Locale;
}

/**
 * Detect the initial locale. Safe to call outside the browser (returns
 * the base locale), but should be called on the client (during `onMount`)
 * to actually differentiate. Has no side effects.
 */
export async function detectInitialLocale(): Promise<Locale> {
  if (typeof window === "undefined") return baseLocale as Locale;

  // 1) Persisted preference wins.
  const saved = readPersisted();
  if (saved) return saved;

  // 2) Tauri OS locale — only when the runtime is present.
  try {
    if (window.__TAURI_INTERNALS__) {
      const { locale } = await import("@tauri-apps/plugin-os");
      const raw = await locale();
      return normalize(raw);
    }
  } catch {
    // OS plugin not available — fall through to navigator.
  }

  // 3) Browser navigator as a fine last-resort.
  return normalize(window.navigator?.language);
}
