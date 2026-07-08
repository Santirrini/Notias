import { browser } from "$app/environment";

/**
 * sectionMap: notebook/section grouping derived purely on the frontend.
 * Persisted in localStorage so it survives reloads but never touches the DB.
 *
 * Shape:
 *   {
 *     "byNote": { "<noteId>": "My section" },
 *     "sections": [{ name, color, virtualPinned?: boolean }],
 *     "active": "My section" | null
 *   }
 *
 * If no entry exists, the store lazily builds it from note tags at first call.
 */
export type SectionDef = {
  name: string;
  color: string;
  pinned?: boolean;
};

type SectionState = {
  byNote: Record<string, string>;
  sections: SectionDef[];
  active: string | null;
};

const STORAGE_KEY = "notias.sectionMap.v1";

function isValidColor(hex: string): boolean {
  return /^#?[0-9a-fA-F]{6}$/.test(hex);
}

function safeRead(): SectionState {
  if (!browser) return { byNote: {}, sections: [], active: null };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { byNote: {}, sections: [], active: null };
    const parsed = JSON.parse(raw) as SectionState;
    if (!parsed || typeof parsed !== "object") throw new Error("bad shape");
    return {
      byNote: parsed.byNote ?? {},
      sections: (parsed.sections ?? []).filter(
        (s): s is SectionDef =>
          typeof s?.name === "string" && typeof s?.color === "string" && isValidColor(s.color)
      ),
      active: typeof parsed.active === "string" ? parsed.active : null,
    };
  } catch {
    return { byNote: {}, sections: [], active: null };
  }
}

function persist(state: SectionState) {
  if (!browser) return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    /* quota / disabled storage — silently ignore */
  }
}

class SectionsStore {
  state = $state<SectionState>({ byNote: {}, sections: [], active: null });
  initialized = $state(false);

  hydrate() {
    if (this.initialized || !browser) return;
    this.state = safeRead();
    this.initialized = true;
  }

  /** Seed sections from tag set when no manual config exists. */
  seedFromTags(uniqueTags: string[]) {
    if (this.state.sections.length > 0) return;
    const palette = [
      "#0078d4",
      "#107c10",
      "#d13438",
      "#ffb900",
      "#5c2d91",
      "#008272",
      "#ca5010",
    ];
    const seeded: SectionDef[] = uniqueTags.map((name, i) => ({
      name,
      color: palette[i % palette.length] ?? "#0078d4",
    }));
    if (!seeded.find((s) => s.name === "General")) {
      seeded.push({ name: "General", color: "#8a8886", pinned: true });
    }
    this.state = { ...this.state, sections: seeded, active: seeded[0]?.name ?? "General" };
    persist(this.state);
  }

  setActive(name: string) {
    this.state = { ...this.state, active: name };
    persist(this.state);
  }

  createSection(name: string, color: string) {
    if (!name.trim()) return;
    if (this.state.sections.some((s) => s.name === name)) {
      this.state = { ...this.state, active: name };
      persist(this.state);
      return;
    }
    const sections = [...this.state.sections, { name, color }];
    this.state = { ...this.state, sections, active: name };
    persist(this.state);
  }

  renameSection(oldName: string, nextName: string) {
    const target = nextName.trim();
    if (!target || target === oldName) return;
    if (this.state.sections.some((s) => s.name === target)) return;
    const sections = this.state.sections.map((s) =>
      s.name === oldName ? { ...s, name: target } : s,
    );
    const byNote: Record<string, string> = {};
    for (const [nid, sec] of Object.entries(this.state.byNote)) {
      byNote[nid] = sec === oldName ? target : sec;
    }
    const active = this.state.active === oldName ? target : this.state.active;
    this.state = { sections, byNote, active };
    persist(this.state);
  }

  removeSection(name: string) {
    if (name === "General") return; // protect the default
    const sections = this.state.sections.filter((s) => s.name !== name);
    const byNote: Record<string, string> = {};
    for (const [nid, sec] of Object.entries(this.state.byNote)) {
      byNote[nid] = sec === name ? "General" : sec;
    }
    const active =
      this.state.active === name
        ? sections[0]?.name ?? "General"
        : this.state.active;
    this.state = { sections, byNote, active };
    persist(this.state);
  }

  togglePin(name: string) {
    const sections = this.state.sections.map((s) =>
      s.name === name ? { ...s, pinned: !s.pinned } : s,
    );
    this.state = { ...this.state, sections };
    persist(this.state);
  }

  /** Resolve the section name for a note; falls back to 'General'. */
  sectionFor(noteId: string, tags: string[]): string {
    const assigned = this.state.byNote[noteId];
    if (assigned) return assigned;
    const firstKnown = tags.find((t) =>
      this.state.sections.some((s) => s.name === t)
    );
    if (firstKnown) return firstKnown;
    return "General";
  }
}

export const sections = new SectionsStore();
