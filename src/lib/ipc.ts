import { invoke } from "@tauri-apps/api/core";
import type { WireError, Note, NoteSummary } from "./types";

export async function ping(): Promise<string> {
  try {
    return await invoke<string>("ping");
  } catch (e) {
    throw e as WireError;
  }
}

export const listNotes = (query?: string, tag?: string) =>
  invoke<NoteSummary[]>("list_notes", { query, tag });
export const getNote = (id: string) => invoke<Note>("get_note", { id });
export const createNote = (title: string) => invoke<Note>("create_note", { title });
export const updateNote = (id: string, patch: { title?: string; body?: string }) =>
  invoke<Note>("update_note", { id, title: patch.title, body: patch.body });
export const deleteNote = (id: string) => invoke<void>("delete_note", { id });
export const searchNotes = (q: string) => invoke<string[]>("search_notes", { q });
export const rebuildIndex = () => invoke<number>("rebuild_index");
export const recoveryRequired = () => invoke<boolean>("recovery_required");