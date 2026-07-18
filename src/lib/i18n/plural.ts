/**
 * Plural-aware message lookup.
 *
 * Paraglide's plugin-message-format v4 does NOT parse ICU `{n, plural, ...}`
 * syntax — it only handles `{var}` interpolation. We compensate with a
 * convention: every pluralizable key ends in `_one` or `_other`, and this
 * helper picks the right variant via `Intl.PluralRules` (CLDR).
 *
 * Usage:
 *   import { plural } from "$lib/i18n/plural";
 *   plural({ count: 3 }, "units_files")  // → m.units_files_other(...) or m.units_files_one(...)
 *
 * Supported buckets (English, Spanish): `one` and `other`. Add more by
 * extending the bucket mapping and authoring the keys per locale.
 */

import { i18n } from "./locale.svelte";
import * as messages from "$lib/paraglide/messages.js";

/** Signature of every Paraglide-compiled message function. */
type MessageFn = (...args: any[]) => string;

/**
 * Pick a plural-aware variant of a key and call it.
 *
 * @example
 *   plural({ count: 1 }, "units_files")  // m.units_files_one({ count: 1 })
 *   plural({ count: 5 }, "units_files")  // m.units_files_other({ count: 5 })
 */
export function plural(
  inputs: Record<string, unknown> & { count: number },
  baseKey: string,
): string {
  const cat = pluralCategory(inputs.count);
  const key = `${baseKey}_${cat}`;
  const fn = (messages as unknown as Record<string, MessageFn>)[key];
  if (fn) return fn(inputs);

  const fallback = (messages as unknown as Record<string, MessageFn>)[`${baseKey}_other`];
  if (fallback) return fallback(inputs);

  if (import.meta.env.DEV) {
    console.warn(`[i18n] plural key not found: ${key}`);
  }
  return String(inputs.count);
}

/** CLDR plural category for the current (or passed) locale. */
export function pluralCategory(n: number, locale: string = i18n.locale): "one" | "other" {
  const rules = new Intl.PluralRules(locale);
  const category = rules.select(n);
  return category === "one" ? "one" : "other";
}
