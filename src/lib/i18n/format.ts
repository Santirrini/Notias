/**
 * Locale-aware date / number formatting helpers.
 *
 * All read `i18n.locale` reactively; when the user changes language,
 * derived `$derived` formatters in calling components update automatically.
 *
 * We do NOT export destructured Intl formatters — Intl instances are
 * expensive and components that need them should declare their own
 * `$derived` formatter and call into these helpers. This module ships
 * plain functions that read the locale from the store at call time.
 */

import { i18n } from "./locale.svelte";

/** "10/06/2026" / "6/10/2026" — short, unambiguous per locale. */
export function formatDate(d: Date | number | string, opts: Intl.DateTimeFormatOptions = { day: "2-digit", month: "2-digit", year: "numeric" }): string {
  const date = d instanceof Date ? d : new Date(d);
  return new Intl.DateTimeFormat(i18n.locale, opts).format(date);
}

/** "Tuesday, June 10, 2026" / "martes, 10 de junio de 2026" — full. */
export function formatDateLong(d: Date | number | string): string {
  const date = d instanceof Date ? d : new Date(d);
  return new Intl.DateTimeFormat(i18n.locale, {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
  }).format(date);
}

/** "10:23" / "10:23" — hour+minute in 24h. */
export function formatTime(d: Date | number | string): string {
  const date = d instanceof Date ? d : new Date(d);
  return new Intl.DateTimeFormat(i18n.locale, { hour: "2-digit", minute: "2-digit" }).format(date);
}

/** "1,234" / "1.234" — thousands separator, no decimal. */
export function formatInteger(n: number): string {
  return new Intl.NumberFormat(i18n.locale, { maximumFractionDigits: 0 }).format(n);
}

/** "1,234.56" / "1234,56" — group + decimal separator. */
export function formatNumber(n: number, fractionDigits = 2): string {
  return new Intl.NumberFormat(i18n.locale, {
    minimumFractionDigits: fractionDigits,
    maximumFractionDigits: fractionDigits,
  }).format(n);
}

/** "1.2 MB" / "1,2 MB" — file sizes, binary or decimal. */
export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  const formatted = new Intl.NumberFormat(i18n.locale, {
    maximumFractionDigits: unit === 0 ? 0 : 1,
  }).format(value);
  return `${formatted} ${units[unit]}`;
}

/** Relative time: "3 days ago" / "hace 3 días". Used by PageList relative dates. */
export function formatRelative(
  pastOrFuture: Date | number | string,
  pivot: Date = new Date(),
  opts: { numeric?: "always" | "auto" } = { numeric: "auto" },
): string {
  const date = pastOrFuture instanceof Date ? pastOrFuture : new Date(pastOrFuture);
  const diffMs = date.getTime() - pivot.getTime();
  const absMs = Math.abs(diffMs);

  // Choose unit by magnitude.
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;

  const rtf = new Intl.RelativeTimeFormat(i18n.locale, opts);

  if (absMs < hour) {
    return rtf.format(Math.round(diffMs / minute), "minute");
  }
  if (absMs < day) {
    return rtf.format(Math.round(diffMs / hour), "hour");
  }
  return rtf.format(Math.round(diffMs / day), "day");
}
