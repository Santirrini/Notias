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

  /**
   * Monotonic counter so that a fast user navigating from A to B to C
   * doesn't let the in-flight `load(A)` result clobber `load(C)`. Each
   * `load()` bumps the counter and only commits if it still matches.
   */
  private loadToken = 0;

  async refresh() {
    try {
      this.list = await listNotes();
    } catch {
      this.list = readLocalNotes().map((l) => ({
        id: l.id,
        title: l.title,
        tags: l.tags,
        updated: l.updated,
      }));
    }
  }
  async search(q: string) {
    try {
      const ids = await searchNotes(q);
      return await Promise.all(ids.map(getNote));
    } catch {
      const needle = q.toLowerCase();
      const locals = readLocalNotes().map(noteFromLocal);
      return locals.filter(
        (n) =>
          n.title.toLowerCase().includes(needle) ||
          n.frontmatter.tags.some((t) => t.toLowerCase().includes(needle)),
      );
    }
  }
  async create(title: string) {
    try {
      const n = await createNote(title);
      await this.refresh();
      return n;
    } catch {
      const id = makeLocalId();
      const now = new Date().toISOString();
      const note: LocalNote = { id, title, body: "", tags: [], updated: now };
      const all = readLocalNotes();
      all.unshift(note);
      writeLocalNotes(all);
      await this.refresh();
      return noteFromLocal(note);
    }
  }
  async load(id: string) {
    // Bump the token; only the latest call's result wins.
    const tok = ++this.loadToken;
    if (isLocalId(id)) {
      const found = readLocalNotes().find((n) => n.id === id);
      if (tok === this.loadToken) this.current = found ? noteFromLocal(found) : null;
      return;
    }
    try {
      const result = await getNote(id);
      if (tok === this.loadToken) this.current = result;
    } catch {
      if (tok === this.loadToken) this.current = null;
    }
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
    try {
      this.current = await updateNote(id, patch);
    } catch {
      // backend unavailable; keep local state untouched
    }
    await this.refresh();
  }
  async remove(id: string) {
    try {
      await deleteNote(id);
    } catch {
      const all = readLocalNotes().filter((n) => n.id !== id);
      writeLocalNotes(all);
    }
    if (this.current?.id === id) this.current = null;
    await this.refresh();
  }
}

export const notes = new NotesStore();
