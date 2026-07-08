/**
 * Lightweight "is the Tauri backend reachable?" probe and offline callout store.
 *
 * The frontend loads inside a plain browser during `pnpm dev` (no Tauri runtime).
 * Every IPC call then throws "Cannot read properties of undefined (reading 'invoke')"
 * because `@tauri-apps/api/core::invoke` is undefined. We:
 *   1. Detect once via `__TAURI_INTERNALS__` global.
 *   2. Expose a derived $state store so any view can render a friendly callout.
 *   3. Expose `safeInvoke<T>(cmd, args)` returning `{ ok: true; value: T } | { ok: false; error: string }`
 *      so call sites can choose between silent fallback or visible error.
 */

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const isTauri =
  typeof window !== "undefined" &&
  typeof window.__TAURI_INTERNALS__ === "object";

import { invoke } from "@tauri-apps/api/core";

class BackendStore {
  /** True only when the app runs inside Tauri (window.__TAURI_INTERNALS__ present). */
  available = $state(isTauri);
  /** Optional human-readable reason this was set to false. */
  reason = $state<string | null>(null);
  /** Last error message surfaced by safeInvoke, for UI banners. */
  lastError = $state<string | null>(null);

  setUnavailable(reason: string) {
    this.available = false;
    this.reason = reason;
  }

  setAvailable() {
    this.available = true;
    this.reason = null;
    this.lastError = null;
  }

  recordError(message: string) {
    this.lastError = message;
    if (!this.available) return;
    this.setUnavailable(message);
  }
}

export const backend = new BackendStore();

export type InvokeResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: string; offline: boolean };

/**
 * Wrap Tauri invoke with offline-awareness. When run in the plain browser,
 * `invoke` is undefined — we catch and return a clean error.
 */
export async function safeInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<InvokeResult<T>> {
  if (typeof invoke !== "function") {
    backend.setUnavailable("Tauri runtime not detected");
    return { ok: false, offline: true, error: "Backend unavailable" };
  }
  try {
    const value = (await invoke(cmd, args)) as T;
    if (!backend.available) backend.setAvailable();
    return { ok: true, value };
  } catch (e) {
    const message =
      (e as { message?: string })?.message ??
      (typeof e === "string" ? e : "Unknown IPC error");
    backend.recordError(message);
    return { ok: false, offline: false, error: message };
  }
}

/**
 * AI helpers — thin wrappers over `safeInvoke` that return `null` on failure
 * instead of a discriminated union, since the caller almost always wants to
 * fall back to "do nothing" or surface a toast. Errors still flow into the
 * `backend.lastError` so the global `OfflineCallout` stays accurate.
 */
export async function safeAiComplete(
  prompt: string,
  model?: string,
): Promise<string | null> {
  const r = await safeInvoke<string>("ai_complete", { prompt, model });
  return r.ok ? r.value : null;
}

export async function safeAiSummarize(
  text: string,
  style: string,
): Promise<string | null> {
  const r = await safeInvoke<string>("ai_summarize", { text, style });
  return r.ok ? r.value : null;
}

export async function safeAiTranscribe(
  audioPath: string,
): Promise<string | null> {
  const r = await safeInvoke<string>("ai_transcribe", { audioPath });
  return r.ok ? r.value : null;
}
