/**
 * Barrel for the i18n subsystem.
 * Re-exports the reactive store, helpers, and Paraglide's compiled `m.*`
 * for ergonomic single-import call sites:
 *
 *   import { i18n, m, plural, localizeError, formatDate } from "$lib/i18n";
 */

export { i18n, STORAGE_KEY } from "./locale.svelte";
export type { Locale } from "$lib/paraglide/runtime.js";

export { detectInitialLocale } from "./detect";
export { plural, pluralCategory } from "./plural";
export {
  formatDate,
  formatDateLong,
  formatTime,
  formatInteger,
  formatNumber,
  formatBytes,
  formatRelative,
} from "./format";
export { localizeError, isOffline } from "./errors";
export type { WireError } from "./errors";

export * as m from "$lib/paraglide/messages.js";
