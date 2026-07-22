import { browser } from "$app/environment";
import {
  DEFAULT_PAPER_PREFS,
  EDITOR_FONTS,
  EDITOR_LINE_HEIGHTS,
  EDITOR_PAGE_WIDTHS,
  EDITOR_SIZES,
  PAPER_TINTS,
  PAPER_VARIANTS,
  type PaperPrefs,
} from "$lib/editor/paper";

/**
 * Global paper / typography defaults. These are the values that new notes (or
 * notes whose frontmatter doesn't override a field) inherit. Per-note
 * overrides live in the note's YAML frontmatter and are resolved by
 * `$lib/editor/paper.ts::resolvePaper`.
 *
 * Persisted to localStorage. Survives reloads but not device transfer. When
 * the user starts exporting the zip for cross-device sync we can promote
 * this to the Rust `config_file` if they ask for it.
 */
const STORAGE_KEY = "notias.paper.v1";

function isValidShape(raw: unknown): raw is Partial<PaperPrefs> {
  if (!raw || typeof raw !== "object") return false;
  const r = raw as Record<string, unknown>;
  const checks: [string, readonly string[]][] = [
    ["paper", PAPER_VARIANTS],
    ["paperTint", PAPER_TINTS],
    ["editorFont", EDITOR_FONTS],
    ["editorFontSize", EDITOR_SIZES],
    ["editorLineHeight", EDITOR_LINE_HEIGHTS],
    ["editorPageWidth", EDITOR_PAGE_WIDTHS],
  ];
  for (const [k, allowed] of checks) {
    if (r[k] !== undefined && !allowed.includes(r[k] as string)) return false;
  }
  return true;
}

function safeRead(): PaperPrefs {
  if (!browser) return { ...DEFAULT_PAPER_PREFS };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_PAPER_PREFS };
    const parsed = JSON.parse(raw);
    if (!isValidShape(parsed)) return { ...DEFAULT_PAPER_PREFS };
    return { ...DEFAULT_PAPER_PREFS, ...parsed };
  } catch {
    return { ...DEFAULT_PAPER_PREFS };
  }
}

function persist(state: PaperPrefs) {
  if (!browser) return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    /* quota / disabled storage — silently ignore */
  }
}

class PaperPrefsStore {
  state = $state<PaperPrefs>({ ...DEFAULT_PAPER_PREFS });
  initialized = $state(false);

  hydrate() {
    if (this.initialized || !browser) return;
    this.state = safeRead();
    this.initialized = true;
  }

  /** Update a single field. Out-of-range values are silently ignored. */
  set<K extends keyof PaperPrefs>(key: K, value: PaperPrefs[K]) {
    const allowed: Record<keyof PaperPrefs, readonly string[]> = {
      paper: PAPER_VARIANTS,
      paperTint: PAPER_TINTS,
      editorFont: EDITOR_FONTS,
      editorFontSize: EDITOR_SIZES,
      editorLineHeight: EDITOR_LINE_HEIGHTS,
      editorPageWidth: EDITOR_PAGE_WIDTHS,
    };
    if (!allowed[key].includes(value as string)) return;
    this.state = { ...this.state, [key]: value };
    persist(this.state);
  }

  /** Reset every field back to the hardcoded defaults. */
  reset() {
    this.state = { ...DEFAULT_PAPER_PREFS };
    persist(this.state);
  }
}

export const paperPrefs = new PaperPrefsStore();