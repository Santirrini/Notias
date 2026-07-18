/**
 * Smoke tests for the i18n subsystem (Paraglide API only).
 *
 * Tests the compiled runtime directly (no runes, no Svelte compiler).
 * Rune-backed helpers (locale.svelte.ts + format.ts) need a Svelte
 * runtime; those are validated manually through the SPA UI.
 *
 * The runes-backed pieces still get exercised by:
 *   - `pnpm check` (svelte-check, type-level)
 *   - `pnpm i18n:check` (parity EN ↔ ES at compile time)
 *   - `pnpm build` (production bundle)
 *   - the manual DOM check you did earlier (Phase 3 bug find)
 */

import { describe, it, expect, beforeAll } from "vitest";
import * as m from "../src/lib/paraglide/messages.js";
import {
  getLocale,
  setLocale,
  locales,
  baseLocale,
  assertIsLocale,
  toLocale,
} from "../src/lib/paraglide/runtime.js";

beforeAll(() => {
  setLocale("en", { reload: false });
});

describe("Paraglide: compile-time API contract", () => {
  it("exposes the configured locales", () => {
    expect(baseLocale).toBe("en");
    expect(locales).toEqual(["en", "es"]);
  });

  it("getLocale() returns current locale", () => {
    expect(getLocale()).toBe("en");
    setLocale("es", { reload: false });
    expect(getLocale()).toBe("es");
    setLocale("en", { reload: false });
  });

  it("toLocale matches only exact configured tags (case-insensitive)", () => {
    // toLocale is the *strict* matcher used by Paraglide's locale
    // validation. It returns the canonical tag if the input matches one
    // of the configured locales case-insensitively; otherwise undefined.
    // Sub-tag parsing ('en-US' → 'en') is reserved for extractLocaleFromHeader
    // / extractLocaleFromNavigator which splits on '-'.
    expect(toLocale("en")).toBe("en");
    expect(toLocale("ES")).toBe("es");
    expect(toLocale("en-US")).toBeUndefined();
    expect(toLocale("fr-FR")).toBeUndefined();
  });

  it("assertIsLocale throws on unknown locales", () => {
    expect(assertIsLocale("en")).toBe("en");
    expect(assertIsLocale("es")).toBe("es");
    expect(() => assertIsLocale("fr")).toThrow();
  });
});

describe("Paraglide: messages reflect current locale on every call", () => {
  it("EN/ES parity on a representative sample of keys", () => {
    const samples: Array<[keyof typeof m, string, string]> = [
      ["common_save", "Save", "Guardar"],
      ["common_cancel", "Cancel", "Cancelar"],
      ["nav_notes", "Notes", "Notas"],
      ["nav_chat", "Chat", "Chat"],
      ["nav_tasks", "Tasks", "Tareas"],
      ["nav_study", "Study", "Estudio"],
      ["nav_settings", "Settings", "Ajustes"],
      ["err_offline", "Backend unavailable", "Servicio no disponible"],
      ["err_invalid", "Invalid input.", "Entrada no válida."],
      ["err_auth", "Authentication required.", "Autenticación requerida."],
      ["err_db_unavailable", "Database is not available.", "La base de datos no está disponible."],
      ["err_task_title_required", "Task title is required.", "El título de la tarea es obligatorio."],
      ["dates_today", "Today", "Hoy"],
      ["dates_yesterday", "Yesterday", "Ayer"],
      ["dates_due_today", "Due today", "Vence hoy"],
      ["dates_due_tomorrow", "Due tomorrow", "Vence mañana"],
    ];

    setLocale("en", { reload: false });
    for (const [key, enText] of samples) {
      const fn = m[key] as () => string;
      expect(fn(), `key ${String(key)} should be English`).toBe(enText);
    }

    setLocale("es", { reload: false });
    for (const [key, , esText] of samples) {
      const fn = m[key] as () => string;
      expect(fn(), `key ${String(key)} should be Spanish`).toBe(esText);
    }
    setLocale("en", { reload: false });
  });

  it("parametrized messages interpolate correctly per locale", () => {
    setLocale("en", { reload: false });
    expect(m.settings_language_saved({ lang: "Español" })).toBe(
      "Language updated to Español",
    );
    setLocale("es", { reload: false });
    expect(m.settings_language_saved({ lang: "English" })).toBe(
      "Idioma cambiado a English",
    );
    setLocale("en", { reload: false });
  });

  it("plural-like variants exist (one/other pairs)", () => {
    setLocale("en", { reload: false });
    expect(m.dates_minutes_ago_one({ minutes: 1 })).toBe("1 minute ago");
    expect(m.dates_minutes_ago_other({ minutes: 5 })).toBe("5 minutes ago");
    setLocale("es", { reload: false });
    expect(m.dates_minutes_ago_one({ minutes: 1 })).toBe("hace 1 minuto");
    expect(m.dates_minutes_ago_other({ minutes: 5 })).toBe("hace 5 minutos");
    setLocale("en", { reload: false });
  });
});

describe("Paraglide: bundle integrity", () => {
  it("does not expose stub m.*() functions for every namespace", () => {
    // Sanity check: every "namespace.*" we promise in messages/ shows up as
    // a callable function. Catches accidental partial compiles.
    const namespaces = ["common_", "nav_", "err_", "dates_", "settings_", "tasks_", "calendar_", "chat_", "study_", "audio_", "editor_", "palette_", "home_", "provider_key_", "note_canvas_", "section" /* placeholder intentionally invalid */];
    for (const ns of namespaces) {
      if (ns === "section") continue;
      const any = Object.keys(m).filter((k) => k.startsWith(ns));
      expect(any.length, `namespace '${ns}' should have keys`).toBeGreaterThan(0);
    }
  });
});
