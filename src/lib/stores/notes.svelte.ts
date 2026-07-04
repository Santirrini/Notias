import { listNotes, createNote, deleteNote, searchNotes, getNote, updateNote } from "$lib/ipc";
import type { Note, NoteSummary } from "$lib/types";

class NotesStore {
  list = $state<NoteSummary[]>([]);
  current = $state<Note | null>(null);

  async refresh() {
    this.list = await listNotes();
  }
  async search(q: string) {
    const ids = await searchNotes(q);
    return Promise.all(ids.map(getNote));
  }
  async create(title: string) {
    const n = await createNote(title);
    await this.refresh();
    return n;
  }
  async load(id: string) { this.current = await getNote(id); }
  async save(id: string, patch: { title?: string; body?: string }) {
    this.current = await updateNote(id, patch);
    await this.refresh();
  }
  async remove(id: string) {
    await deleteNote(id);
    if (this.current?.id === id) this.current = null;
    await this.refresh();
  }
}

export const notes = new NotesStore();