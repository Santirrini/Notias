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
  typeof (window as Window).__TAURI_INTERNALS__ === "object";

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
  // Tauri's `invoke` is an ESM import that always resolves to a function,
  // even when the runtime hasn't injected `__TAURI_INTERNALS__` yet. Guard on
  // the runtime object itself — that's what `invoke()` reads `.invoke` off of
  // internally, and missing it is what triggers
  //   TypeError: Cannot read properties of undefined (reading 'invoke').
  if (typeof window === "undefined" || !window.__TAURI_INTERNALS__) {
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
      // Surface real IPC failures (not offline-mode) on the dev overlay so
      // a thrown command doesn't disappear silently. Gated by DEV to avoid
      // leaking stack traces to end users in production.
      if (typeof window !== "undefined" && import.meta.env.DEV) {
        try {
          window.dispatchEvent(
            new CustomEvent("notias:error", {
              detail: {
                message: `IPC ${cmd}: ${message}`,
                stack: (e as { stack?: string })?.stack,
                pathname: window.location.pathname,
              },
            }),
          );
        } catch {
          /* never let dispatchEvent escape */
        }
      }
      return { ok: false, offline: false, error: message };
    }
  }

  /**
   * Wrap a Tauri's `listen()` (event subscription) with offline-awareness.
   * Mirrors `safeInvoke` for the event API. Returns `null` when the backend
   * is not reachable so the caller can no-op without a try/catch boilerplate.
   *
   * If the listener registers successfully, returns its `unlisten` callback.
   */
export async function safeListen<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<(() => void) | null> {
  if (typeof window === "undefined" || !window.__TAURI_INTERNALS__) {
    backend.setUnavailable("Tauri runtime not detected");
    return null;
  }
  try {
    const { listen } = await import("@tauri-apps/api/event");
    const unlisten = await listen<T>(event, (e) => handler(e.payload));
    if (!backend.available) backend.setAvailable();
    return unlisten;
  } catch (e) {
      const message =
        (e as { message?: string })?.message ??
        (typeof e === "string" ? e : "Unknown event error");
      backend.recordError(message);
      return null;
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

/**
 * Collapse an `InvokeResult<T>` to `T | null`. Use when the caller does not
 * care about *why* the call failed (offline vs error) — same shape as the
 * `safeAi*` helpers above but generic.
 */
export function unwrap<T>(r: InvokeResult<T>): T | null {
  return r.ok ? r.value : null;
}
