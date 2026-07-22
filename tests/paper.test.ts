/**
 * Tests for the paper / typography resolver.
 *
 * Run with `pnpm test`. These cover:
 * - frontmatter override > global default precedence
 * - invalid token in frontmatter falls back to global default
 * - empty-string / null / undefined treated as "no override"
 * - toEditorStyle maps a ResolvedPaper to the class + CSS variable map
 *   that NoteCanvas applies to `.editor-host`
 */

import { describe, it, expect } from "vitest";
import {
  DEFAULT_PAPER_PREFS,
  resolvePaper,
  toEditorStyle,
  type PaperPrefs,
} from "$lib/editor/paper";
import type { NoteFrontmatter } from "$lib/types";

const fm = (over: Partial<NoteFrontmatter>): Partial<NoteFrontmatter> => over;
const global = (over: Partial<PaperPrefs>): Partial<PaperPrefs> => over;

describe("resolvePaper precedence", () => {
  it("uses global defaults when frontmatter has no overrides", () => {
    const r = resolvePaper(null, global({ paper: "grid", paperTint: "sepia" }));
    expect(r.paper).toBe("grid");
    expect(r.paperTint).toBe("sepia");
    expect(r.overrides.size).toBe(0);
  });

  it("uses global defaults when frontmatter is undefined", () => {
    const r = resolvePaper(undefined, global({ paper: "dot" }));
    expect(r.paper).toBe("dot");
    expect(r.overrides.size).toBe(0);
  });

  it("falls back to hardcoded defaults when global is missing", () => {
    const r = resolvePaper(null, null);
    expect(r.paper).toBe(DEFAULT_PAPER_PREFS.paper);
    expect(r.paperTint).toBe(DEFAULT_PAPER_PREFS.paperTint);
    expect(r.editorFontSize).toBe(DEFAULT_PAPER_PREFS.editorFontSize);
  });

  it("frontmatter override beats global for each field independently", () => {
    const r = resolvePaper(
      fm({ paper: "blank", editorFont: "serif" }),
      global({
        paper: "ruled",
        paperTint: "warm",
        editorFont: "sans",
        editorFontSize: "md",
        editorLineHeight: "normal",
        editorPageWidth: "normal",
      }),
    );
    expect(r.paper).toBe("blank");
    expect(r.editorFont).toBe("serif");
    // Untouched fields inherit global.
    expect(r.paperTint).toBe("warm");
    expect(r.editorFontSize).toBe("md");
    expect(r.overrides.has("paper")).toBe(true);
    expect(r.overrides.has("editorFont")).toBe(true);
    expect(r.overrides.has("paperTint")).toBe(false);
  });

  it("empty-string frontmatter value is treated as 'no override' (back to global)", () => {
    const r = resolvePaper(
      fm({ paper: "" as NoteFrontmatter["paper"], paperTint: "" as NoteFrontmatter["paperTint"] }),
      global({ paper: "dot", paperTint: "sepia" }),
    );
    expect(r.paper).toBe("dot");
    expect(r.paperTint).toBe("sepia");
    expect(r.overrides.size).toBe(0);
  });

  it("null frontmatter value is treated as 'no override'", () => {
    const r = resolvePaper(
      fm({ paper: null as unknown as NoteFrontmatter["paper"] }),
      global({ paper: "grid" }),
    );
    expect(r.paper).toBe("grid");
    expect(r.overrides.size).toBe(0);
  });

  it("invalid token in frontmatter falls back to global (but is recorded as an attempt)", () => {
    const r = resolvePaper(
      fm({ paper: "neon" as unknown as NoteFrontmatter["paper"] }),
      global({ paper: "grid" }),
    );
    // Effective value comes from global.
    expect(r.paper).toBe("grid");
    // The frontmatter did try to override this field (even if the value was
    // junk); `overrides` tracks presence-of-intent, not validity. This lets
    // the toolbar show the "overridden" badge correctly even when the user
    // typed something by hand that we couldn't honour.
    expect(r.overrides.has("paper")).toBe(true);
  });

  it("invalid token with no global uses the hardcoded default", () => {
    const r = resolvePaper(
      fm({ paper: "neon" as unknown as NoteFrontmatter["paper"] }),
      null,
    );
    expect(r.paper).toBe(DEFAULT_PAPER_PREFS.paper);
  });
});

describe("toEditorStyle", () => {
  it("returns the matching paper-* class and a CSS variable map", () => {
    const r = resolvePaper(fm({ paper: "ruled", paperTint: "sepia" }), null);
    const out = toEditorStyle(r);
    expect(out.classes).toBe("paper-ruled");
    expect(out.style["--paper-bg"]).toBe("var(--paper-sepia-bg)");
    expect(out.style["--paper-gridline"]).toBe("var(--paper-sepia-gridline)");
    expect(out.style["--editor-font-size"]).toBe(`${DEFAULT_PAPER_PREFS.editorFontSize === "md" ? 17 : 17}px`);
    expect(out.style["--editor-page-width"]).toMatch(/px$/);
  });

  it("emits px values for the size and page-width tokens", () => {
    const r = resolvePaper(
      fm({ editorFontSize: "lg", editorPageWidth: "wide" }),
      null,
    );
    const out = toEditorStyle(r);
    expect(out.style["--editor-font-size"]).toBe("19px");
    expect(out.style["--editor-page-width"]).toBe("1080px");
  });
});