import { invoke } from "@tauri-apps/api/core";
import type { WireError, Note, NoteSummary } from "./types";
import type {
  ProviderInfo,
  ProviderStatus,
  StreamHandle,
  RagHit,
  ChatMessage,
} from "./types/ai";

export async function ping(): Promise<string> {
  try {
    return await invoke<string>("ping");
  } catch (e) {
    throw e as WireError;
  }
}

export const listNotes = (tag?: string) =>
  invoke<NoteSummary[]>("list_notes", { tag });
export const getNote = (id: string) => invoke<Note>("get_note", { id });
export const createNote = (title: string) => invoke<Note>("create_note", { title });
export const updateNote = (id: string, patch: { title?: string; body?: string }) =>
  invoke<Note>("update_note", { id, title: patch.title, body: patch.body });
export const deleteNote = (id: string) => invoke<void>("delete_note", { id });
export const searchNotes = (q: string) => invoke<string[]>("search_notes", { q });
export const rebuildIndex = () => invoke<number>("rebuild_index");
export const recoveryRequired = () => invoke<boolean>("recovery_required");

export const listProviders = () => invoke<ProviderInfo[]>("list_providers");
export const enableProvider = (name: string, enabled: boolean, configJson?: string | null) =>
  invoke<void>("enable_provider", { name, enabled, configJson });
export const testProvider = (name: string) =>
  invoke<ProviderStatus>("test_provider", { name });
export const aiChat = (req: { messages: ChatMessage[]; model: string; provider?: string }) =>
  invoke<StreamHandle>("ai_chat", { req });
export const aiComplete = (prompt: string, model?: string) =>
  invoke<string>("ai_complete", { prompt, model });
export const aiSummarize = (text: string, style: string) =>
  invoke<string>("ai_summarize", { text, style });
export const ragSearch = (query: string) =>
  invoke<RagHit[]>("rag_search", { query });
export const aiTranscribe = (audioPath: string) =>
  invoke<string>("ai_transcribe", { audioPath });

export const setProviderKey = (name: string, key: string) =>
  invoke<void>("set_provider_key", { name, key });
export const deleteProviderKey = (name: string) =>
  invoke<void>("delete_provider_key", { name });
export const hasProviderKey = (name: string) =>
  invoke<boolean>("has_provider_key", { name });
export const providerKeyStatus = () =>
  invoke<{ name: string; has_key: boolean }[]>("provider_key_status");