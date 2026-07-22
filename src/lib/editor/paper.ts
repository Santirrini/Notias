/**
 * Paper / typography resolver.
 *
 * Resolution order for each field: per-note frontmatter override → global
 * user default (from `paperPrefs` store) → hardcoded fallback.
 *
 * Frontmatter fields are typed unions (PaperVariant, PaperTint, …). When the
 * resolver is fed raw strings — either coming back from the Rust round-trip
 * or written by hand — anything not in the allowlist is silently replaced by
 * the global default. This makes the system tolerant to junk values without
 * ever rendering a broken UI.
 */

import type {
  EditorFont,
  EditorLineHeight,
  EditorPageWidth,
  EditorSize,
  NoteFrontmatter,
  PaperTint,
  PaperVariant,
} from "$lib/types";

// ─── Tokens (single source of truth) ────────────────────────────────────────

export const PAPER_VARIANTS = ["ruled", "grid", "dot", "blank"] as const;
export const PAPER_TINTS = ["white", "warm", "sepia"] as const;
export const EDITOR_FONTS = ["serif", "sans", "mono"] as const;
export const EDITOR_SIZES = ["sm", "md", "lg"] as const;
export const EDITOR_LINE_HEIGHTS = ["compact", "normal", "relaxed"] as const;
export const EDITOR_PAGE_WIDTHS = ["narrow", "normal", "wide"] as const;

export const FONT_FAMILY: Record<EditorFont, string> = {
  // System serif / mono come first so the cascade always finds a fallback.
  serif: '"Charter", "Iowan Old Style", Georgia, "Times New Roman", serif',
  sans: 'var(--font-sans)',
  mono: 'var(--font-mono)',
};

export const FONT_SIZE_PX: Record<EditorSize, number> = {
  sm: 15,
  md: 17,
  lg: 19,
};

export const LINE_HEIGHT: Record<EditorLineHeight, number> = {
  compact: 1.45,
  normal: 1.6,
  relaxed: 1.8,
};

export const PAGE_WIDTH_PX: Record<EditorPageWidth, number> = {
  narrow: 640,
  normal: 880,
  wide: 1080,
};

/** Map a tint to the CSS variable pair that defines its background + lines. */
export const TINT_VARS: Record<PaperTint, { bg: string; line: string }> = {
  white: { bg: "var(--paper-white-bg)", line: "var(--paper-white-gridline)" },
  warm: { bg: "var(--canvas-bg)", line: "var(--canvas-gridline)" },
  sepia: { bg: "var(--paper-sepia-bg)", line: "var(--paper-sepia-gridline)" },
};

// ─── Defaults ───────────────────────────────────────────────────────────────

/** Hardcoded fallback used when no global preference is hydrated yet.
    `as const` would over-narrow to literal string types and make the resolver
    hard to type; we keep these as the named union members explicitly. */
export const DEFAULT_PAPER_PREFS: PaperPrefs = {
  paper: "ruled",
  paperTint: "warm",
  editorFont: "sans",
  editorFontSize: "md",
  editorLineHeight: "normal",
  editorPageWidth: "normal",
};

export type PaperPrefs = {
  paper: PaperVariant;
  paperTint: PaperTint;
  editorFont: EditorFont;
  editorFontSize: EditorSize;
  editorLineHeight: EditorLineHeight;
  editorPageWidth: EditorPageWidth;
};

// ─── Validation helpers ─────────────────────────────────────────────────────

function pickVariant(v: unknown, fallback: PaperVariant): PaperVariant {
  return (PAPER_VARIANTS as readonly string[]).includes(v as string)
    ? (v as PaperVariant)
    : fallback;
}
function pickTint(v: unknown, fallback: PaperTint): PaperTint {
  return (PAPER_TINTS as readonly string[]).includes(v as string)
    ? (v as PaperTint)
    : fallback;
}
function pickFont(v: unknown, fallback: EditorFont): EditorFont {
  return (EDITOR_FONTS as readonly string[]).includes(v as string)
    ? (v as EditorFont)
    : fallback;
}
function pickSize(v: unknown, fallback: EditorSize): EditorSize {
  return (EDITOR_SIZES as readonly string[]).includes(v as string)
    ? (v as EditorSize)
    : fallback;
}
function pickLineHeight(v: unknown, fallback: EditorLineHeight): EditorLineHeight {
  return (EDITOR_LINE_HEIGHTS as readonly string[]).includes(v as string)
    ? (v as EditorLineHeight)
    : fallback;
}
function pickPageWidth(v: unknown, fallback: EditorPageWidth): EditorPageWidth {
  return (EDITOR_PAGE_WIDTHS as readonly string[]).includes(v as string)
    ? (v as EditorPageWidth)
    : fallback;
}

// ─── Resolution ─────────────────────────────────────────────────────────────

export interface ResolvedPaper {
  paper: PaperVariant;
  paperTint: PaperTint;
  editorFont: EditorFont;
  editorFontSize: EditorSize;
  editorLineHeight: EditorLineHeight;
  editorPageWidth: EditorPageWidth;
  /** Set of fields that came from frontmatter (not from global defaults). */
  overrides: Set<keyof PaperPrefs>;
}

/**
 * Merge frontmatter overrides with global defaults.
 * - `frontmatter` may be `null` (no note loaded).
 * - Global defaults may be incomplete during early hydration; missing keys
 *   fall back to the hardcoded defaults.
 */
export function resolvePaper(
  frontmatter: Pick<NoteFrontmatter,
    "paper" | "paperTint" | "editorFont" | "editorFontSize" |
    "editorLineHeight" | "editorPageWidth"> | null | undefined,
  global: Partial<PaperPrefs> | null | undefined,
): ResolvedPaper {
  const g: PaperPrefs = {
    paper: global?.paper ?? DEFAULT_PAPER_PREFS.paper,
    paperTint: global?.paperTint ?? DEFAULT_PAPER_PREFS.paperTint,
    editorFont: global?.editorFont ?? DEFAULT_PAPER_PREFS.editorFont,
    editorFontSize: global?.editorFontSize ?? DEFAULT_PAPER_PREFS.editorFontSize,
    editorLineHeight: global?.editorLineHeight ?? DEFAULT_PAPER_PREFS.editorLineHeight,
    editorPageWidth: global?.editorPageWidth ?? DEFAULT_PAPER_PREFS.editorPageWidth,
  };
  const overrides = new Set<keyof PaperPrefs>();
  const fm = frontmatter ?? null;

  const pick = <K extends keyof PaperPrefs>(
    fmVal: unknown,
    gKey: K,
    picker: (v: unknown, fb: PaperPrefs[K]) => PaperPrefs[K],
  ): PaperPrefs[K] => {
    if (fmVal != null && (fmVal as string) !== "") {
      overrides.add(gKey);
      return picker(fmVal, g[gKey]);
    }
    return g[gKey];
  };

  // `fm` may be null when no note is loaded. The `pick` helper treats a
  // `null/undefined`/empty value as "no override", so passing undefined for
  // every field when fm is null yields the global defaults.
  const safe = (key: keyof PaperPrefs): unknown =>
    fm ? (fm as Record<string, unknown>)[key] : undefined;

  return {
    paper: pick(safe("paper"), "paper", pickVariant),
    paperTint: pick(safe("paperTint"), "paperTint", pickTint),
    editorFont: pick(safe("editorFont"), "editorFont", pickFont),
    editorFontSize: pick(safe("editorFontSize"), "editorFontSize", pickSize),
    editorLineHeight: pick(safe("editorLineHeight"), "editorLineHeight", pickLineHeight),
    editorPageWidth: pick(safe("editorPageWidth"), "editorPageWidth", pickPageWidth),
    overrides,
  };
}

// ─── CSS attribute conversion ───────────────────────────────────────────────

/**
 * Convert a resolved paper into the style attribute / class list that gets
 * applied to the editor wrapper.
 *
 * - `style`  → CSS custom properties to merge onto `--paper-bg`,
 *              `--paper-gridline`, `--editor-font-family`, etc.
 * - `classes` → one of `.paper-ruled | .paper-grid | .paper-dot | .paper-blank`.
 */
export function toEditorStyle(p: ResolvedPaper): {
  style: Record<string, string>;
  classes: string;
} {
  const tint = TINT_VARS[p.paperTint];
  const fontSizePx = FONT_SIZE_PX[p.editorFontSize];
  const lineHeight = LINE_HEIGHT[p.editorLineHeight];
  const pageWidthPx = PAGE_WIDTH_PX[p.editorPageWidth];

  const style: Record<string, string> = {
    "--paper-bg": tint.bg,
    "--paper-gridline": tint.line,
    "--editor-font-family": FONT_FAMILY[p.editorFont],
    "--editor-font-size": `${fontSizePx}px`,
    "--editor-line-height": `${lineHeight}`,
    "--editor-page-width": `${pageWidthPx}px`,
  };
  return { style, classes: `paper-${p.paper}` };
}