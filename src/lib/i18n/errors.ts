/**
 * Map Rust `AppError` wire codes to localized UI messages.
 *
 * The backend (Rust) ALWAYS returns `{ code, message }` where:
 *   - `code` is one of: "io" | "db" | "serialization" | "auth" |
 *     "config" | "not_found" | "invalid" | "provider"
 *   - `message` is the English thiserror-serialized string
 *
 * The strategy agreed with the maintainer:
 *   - Use `code` as the stable contract (fast, no regex)
 *   - Use `message` only as last-resort fallback (when code doesn't help)
 *   - All translations live here, not in Rust
 *
 * Callers pass the result of `safeInvoke` (which already returns
 * `{ ok: false, error: string, offline: boolean }`) along with the
 * underlying wire code (we expose it via `safeInvoke` returns; if
 * unavailable we fall back to message-pattern matching).
 */

import * as m from "$lib/paraglide/messages.js";

/** Sub-shape of `AppError` Serialize output that we care about. */
export interface WireError {
  code?: string;
  message?: string;
}

/** Detect "offline" mode from safeInvoke's offline boolean AND error string. */
export function isOffline(error: { offline?: boolean; error?: string; message?: string }): boolean {
  if (error.offline) return true;
  const msg = (error.error ?? error.message ?? "").toLowerCase();
  return msg === "backend unavailable" || msg === "tauri runtime not detected";
}

/**
 * Translate a WireError into a localized message ready for `toast.error()`.
 *
 * `code` is preferred; if it's missing we sniff `message` for known
 * substrings (so the upgrade path stays safe if a backend serializes
 * slightly different text). Falls through to a generic message that
 * surfaces the original backend string for debugging.
 */
export function localizeError(e: WireError | null | undefined): string {
  if (!e) return m.err_unknown();

  if (e.code) {
    switch (e.code) {
      case "io":
        return m.err_io();
      case "db":
        return m.err_db();
      case "serialization":
        return m.err_serialization();
      case "auth":
        return m.err_auth();
      case "config":
        return m.err_config();
      case "not_found":
        return m.err_not_found();
      case "invalid":
        return errInvalidWithDetail(e.message);
      case "provider":
        return errProviderWithDetail(e.message);
      default:
        return m.err_generic({ message: e.message ?? "—" });
    }
  }

  // No code → sniff message for known specifics.
  return sniffByMessage(e.message);
}

function errInvalidWithDetail(message: string | undefined): string {
  const m_ = (message ?? "").toLowerCase();
  if (m_.includes("title required")) return m.err_task_title_required();
  if (m_.includes("1..=20") || m_.includes("between 1 and 20")) return m.err_invalid_count();
  return m.err_invalid();
}

function errProviderWithDetail(message: string | undefined): string {
  const m_ = (message ?? "").toLowerCase();
  if (m_.includes("no ai provider")) return m.err_no_ai_provider();
  if (m_.includes("transcribe requires")) return m.err_transcribe_requires_cloud();
  return m.err_provider();
}

function sniffByMessage(message: string | undefined): string {
  if (!message) return m.err_unknown();
  const m_ = message.toLowerCase();
  if (m_.includes("title required")) return m.err_task_title_required();
  if (m_.includes("not connected") && m_.includes("calendar")) return m.err_calendar_not_connected();
  if (m_.includes("token expired")) return m.err_calendar_token_expired();
  if (m_.includes("no code in redirect")) return m.err_no_code();
  if (m_.includes("empty key")) return m.err_key_empty();
  if (m_.includes("note_ids empty")) return m.err_quiz_empty();
  if (m_.includes("db lock poisoned")) return m.err_db_lock_poisoned();
  if (m_.includes("database unavailable")) return m.err_db_unavailable();
  if (m_.includes("no ai provider")) return m.err_no_ai_provider();
  if (m_.includes("transcribe requires")) return m.err_transcribe_requires_cloud();
  if (m_.includes("not found")) return m.err_not_found();
  if (m_.includes("invalid")) return m.err_invalid();
  if (m_.includes("auth")) return m.err_auth();
  if (m_.includes("io") || m_.includes("zip")) return m.err_io();
  if (m_.includes("provider")) return m.err_provider();
  // Last-resort: include the raw message (English) so devs can diagnose.
  return m.err_generic({ message });
}
