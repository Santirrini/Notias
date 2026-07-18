/**
 * Tests for `safeInvoke` — the contract every IPC wrapper in `$lib/ipc.ts`
 * relies on. Adding a regression here breaks the build before any call-site
 * sees a wrong-shape `InvokeResult`.
 *
 * Run with `pnpm test`.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";

// We mock `@tauri-apps/api/core` BEFORE importing the store so that the
// `invoke` symbol resolved at module init is the mock.
const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

// `safeInvoke` reads `import.meta.env.DEV` when dispatching the notias:error
// event in dev. Vitest sets DEV=true by default, so no extra setup needed.

import { safeInvoke, unwrap } from "./backend.svelte";

describe("safeInvoke contract", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns { ok: true, value } when invoke resolves", async () => {
    invokeMock.mockResolvedValueOnce("pong");
    const r = await safeInvoke<string>("ping");
    expect(r).toEqual({ ok: true, value: "pong" });
    expect(invokeMock).toHaveBeenCalledWith("ping", undefined);
  });

  it("returns { ok: false, offline: true } when invoke is missing", async () => {
    // Simulate the plain-browser case by making `invoke` blow up on access.
    // We do this by replacing the module export for the duration of the test.
    vi.resetModules();
    const core = await import("@tauri-apps/api/core");
    const originalInvoke = (core as { invoke: unknown }).invoke;
    (core as { invoke: unknown }).invoke = undefined as unknown as typeof core.invoke;
    try {
      // Re-import to pick up the modified `invoke` reference inside backend.svelte.
      const mod = await import("./backend.svelte");
      const r = await mod.safeInvoke<string>("ping");
      expect(r.ok).toBe(false);
      if (!r.ok) {
        expect(r.offline).toBe(true);
        expect(typeof r.error).toBe("string");
      }
    } finally {
      (core as { invoke: unknown }).invoke = originalInvoke;
    }
  });

  it("returns { ok: false, offline: false, error } when invoke throws", async () => {
    invokeMock.mockRejectedValueOnce(new Error("boom"));
    const r = await safeInvoke<string>("ping");
    expect(r).toEqual({ ok: false, offline: false, error: "boom" });
  });

  it("forwards arguments verbatim to invoke", async () => {
    invokeMock.mockResolvedValueOnce(null);
    await safeInvoke<void>("delete_note", { id: "abc" });
    expect(invokeMock).toHaveBeenCalledWith("delete_note", { id: "abc" });
  });
});

describe("unwrap helper", () => {
  it("returns value on ok", () => {
    expect(unwrap({ ok: true, value: 42 })).toBe(42);
  });
  it("returns null on failure regardless of offline flag", () => {
    expect(unwrap({ ok: false, offline: true, error: "x" })).toBeNull();
    expect(unwrap({ ok: false, offline: false, error: "x" })).toBeNull();
  });
});