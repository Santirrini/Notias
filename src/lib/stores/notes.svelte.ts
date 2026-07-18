import { browser } from "$app/environment";
import {
  listNotes,
  createNote,
  deleteNote,
  searchNotes,
  getNote,
  updateNote,
} from "$lib/ipc";
import type { Note, NoteSummary } from "$lib/types";

const LOCAL_KEY = "notias.localNotes.v1";

type LocalNote = {
  id: string;
  title: string;
  body: string;
  tags: string[];
  updated: string;
};

/** Canonical local id generator. Used by `PageList` and the store. */
export function makeLocalId(): string {
  const ts = Date.now().toString(36).toUpperCase();
  const rand = Math.random().toString(36).slice(2, 10).toUpperCase();
  return `LOCAL-${ts}-${rand}`;
}

function readLocalNotes(): LocalNote[] {
  if (!browser) return [];
  try {
    const raw = localStorage.getItem(LOCAL_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as LocalNote[] | NoteSummary[];
    if (!Array.isArray(parsed)) return [];
    // tolerate both full and summary shapes
    return parsed.map((p): LocalNote => ({
      id: (p as LocalNote).id,
      title: (p as LocalNote).title ?? "Untitled",
      body: (p as LocalNote).body ?? "",
      tags: (p as LocalNote).tags ?? [],
      updated: (p as LocalNote).updated ?? new Date().toISOString(),
    }));
  } catch {
    return [];
  }
}

function writeLocalNotes(notes: LocalNote[]) {
  if (!browser) return;
  try {
    localStorage.setItem(LOCAL_KEY, JSON.stringify(notes));
  } catch {
    /* ignore */
  }
}

function noteFromLocal(l: LocalNote): Note {
  const now = l.updated;
  return {
    id: l.id,
    path: `<local>/${l.id}.md`,
    title: l.title,
    body: l.body,
    frontmatter: {
      id: l.id,
      title: l.title,
      tags: l.tags,
      created: now,
      updated: now,
      links: [],
      references: [],
    },
  };
}

function isLocalId(id: string): boolean {
  return id.startsWith("LOCAL-") || id.startsWith("demo-");
}

class NotesStore {
  list = $state<NoteSummary[]>([]);
  current = $state<Note | null>(null);
  /** Last backend-reported error string (e.g. for toasts). Reset on next success. */
  lastError = $state<string | null>(null);

  /**
   * Monotonic counter so that a fast user navigating from A to B to C
   * doesn't let the in-flight `load(A)` result clobber `load(C)`. Each
   * `load()` bumps the counter and only commits if it still matches.
   */
  private loadToken = 0;

  async refresh() {
    const r = await listNotes();
    if (r.ok) {
      this.list = r.value;
      return;
    }
    if (r.offline) {
      // No backend: serve the local mirror (the same one we used before the
      // safeInvoke migration, but now gated explicitly on the offline flag).
      this.list = readLocalNotes().map((l) => ({
        id: l.id,
        title: l.title,
        tags: l.tags,
        updated: l.updated,
      }));
      return;
    }
    // Real backend error (DB locked, etc.). Keep the previous list intact
    // and let the caller's toast surface r.error.
    this.lastError = r.error;
  }

  async search(q: string) {
    const r = await searchNotes(q);
    if (r.ok) {
      const notes: Note[] = [];
      for (const id of r.value) {
        const n = await getNote(id);
        if (n.ok) notes.push(n.value);
      }
      return notes;
    }
    if (r.offline) {
      const needle = q.toLowerCase();
      const locals = readLocalNotes().map(noteFromLocal);
      return locals.filter(
        (n) =>
          n.title.toLowerCase().includes(needle) ||
          n.frontmatter.tags.some((t) => t.toLowerCase().includes(needle)),
      );
    }
    this.lastError = r.error;
    return [];
  }

  async create(title: string) {
    const r = await createNote(title);
    if (r.ok) {
      await this.refresh();
      return r.value;
    }
    if (r.offline) {
      const id = makeLocalId();
      const now = new Date().toISOString();
      const note: LocalNote = { id, title, body: "", tags: [], updated: now };
      const all = readLocalNotes();
      all.unshift(note);
      writeLocalNotes(all);
      await this.refresh();
      return noteFromLocal(note);
    }
    this.lastError = r.error;
    return null;
  }

  async load(id: string) {
    // Bump the token; only the latest call's result wins.
    const tok = ++this.loadToken;
    if (isLocalId(id)) {
      const found = readLocalNotes().find((n) => n.id === id);
      if (tok === this.loadToken) this.current = found ? noteFromLocal(found) : null;
      return;
    }
    const r = await getNote(id);
    if (tok !== this.loadToken) return;
    if (r.ok) {
      this.current = r.value;
      return;
    }
    if (r.offline) {
      // Try local mirror before giving up.
      const found = readLocalNotes().find((n) => n.id === id);
      this.current = found ? noteFromLocal(found) : null;
      return;
    }
    this.lastError = r.error;
    this.current = null;
  }

  async save(id: string, patch: { title?: string; body?: string }) {
    if (isLocalId(id)) {
      const all = readLocalNotes();
      const idx = all.findIndex((n) => n.id === id);
      const now = new Date().toISOString();
      const updated: LocalNote = {
        id,
        title: patch.title ?? all[idx]?.title ?? "Untitled",
        body: patch.body ?? all[idx]?.body ?? "",
        tags: all[idx]?.tags ?? [],
        updated: now,
      };
      if (idx >= 0) all[idx] = updated;
      else all.unshift(updated);
      writeLocalNotes(all);
      this.current = noteFromLocal(updated);
      await this.refresh();
      return this.current!;
    }
    const r = await updateNote(id, patch);
    if (r.ok) {
      this.current = r.value;
    } else if (r.offline) {
      // backend unavailable; keep local state untouched
    } else {
      this.lastError = r.error;
    }
    await this.refresh();
  }

  async remove(id: string) {
    const r = await deleteNote(id);
    if (!r.ok && r.offline) {
      const all = readLocalNotes().filter((n) => n.id !== id);
      writeLocalNotes(all);
    } else if (!r.ok && !r.offline) {
      this.lastError = r.error;
    }
    if (this.current?.id === id) this.current = null;
    await this.refresh();
  }
}

export const notes = new NotesStore();