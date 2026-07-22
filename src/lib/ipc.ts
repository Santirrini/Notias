/**
 * Tauri IPC façade.
 *
 * Every function in this module returns `Promise<InvokeResult<T>>`, a
 * discriminated union:
 *
 *   - `{ ok: true;  value: T }`           — backend reached and answered.
 *   - `{ ok: false; offline: true;  error }` — Tauri runtime not present
 *                                            (`pnpm dev` in a plain browser,
 *                                            extension context, etc.).
 *   - `{ ok: false; offline: false; error }` — backend reached but the command
 *                                            rejected (DB error, missing key,
 *                                            bad input, …).
 *
 * Use the `offline` flag to decide between "use localStorage fallback" and
 * "surface this to the user as an error". Do NOT just check `r.ok` if your
 * store has a local-only fallback path — `safeInvoke` swallows both cases
 * into a single boolean otherwise.
 *
 * The thin wrappers below used to call `invoke()` directly, which threw on
 * any failure. That behavior forced every call-site to add a try/catch and
 * silently broke pages that called IPC in `onMount` (the rejection landed
 * outside the Svelte runtime and the user saw a blank screen). Routing
 * every command through `safeInvoke` keeps the frontend usable in plain
 * browser mode and centralises backend-state tracking on the `backend`
 * store.
 */

import {
  safeInvoke,
  type InvokeResult,
} from "$lib/stores/backend.svelte";
import type {
  WireError,
  Note,
  NoteSummary,
  Task,
  TaskSummary,
  NewTask,
  TaskPatch,
  Card,
  CardSummary,
  DraftCard,
  SaveCardsInput,
  ReviewOutcome,
  Quiz,
  QuizResult,
  NewQuizInput,
  Plan,
  CalendarEvent,
  CalendarAuthStatus,
} from "./types";
import type {
  ProviderInfo,
  ProviderStatus,
  StreamHandle,
  RagHit,
  ChatMessage,
} from "./types/ai";

// Re-export so call-sites only need one import path for the result type.
export type { InvokeResult };

// ──────────────────────────────────────────────────────────────────────────
// Phase 1 — bootstrap
// ──────────────────────────────────────────────────────────────────────────

export const ping = () => safeInvoke<string>("ping");

export const recoveryRequired = () =>
  safeInvoke<boolean>("recovery_required");

// ──────────────────────────────────────────────────────────────────────────
// Phase 1 — notes
// ──────────────────────────────────────────────────────────────────────────

export const listNotes = (tag?: string) =>
  safeInvoke<NoteSummary[]>("list_notes", { tag });

export const getNote = (id: string) =>
  safeInvoke<Note>("get_note", { id });

export const createNote = (title: string) =>
  safeInvoke<Note>("create_note", { title });

export const updateNote = (
  id: string,
  patch: {
    title?: string;
    body?: string;
    paper?: string | null;
    paperTint?: string | null;
    editorFont?: string | null;
    editorFontSize?: string | null;
    editorLineHeight?: string | null;
    editorPageWidth?: string | null;
  },
) => safeInvoke<Note>("update_note", {
  id,
  title: patch.title,
  body: patch.body,
  paper: patch.paper,
  paper_tint: patch.paperTint,
  editor_font: patch.editorFont,
  editor_font_size: patch.editorFontSize,
  editor_line_height: patch.editorLineHeight,
  editor_page_width: patch.editorPageWidth,
});

export const deleteNote = (id: string) =>
  safeInvoke<void>("delete_note", { id });

export const searchNotes = (q: string) =>
  safeInvoke<string[]>("search_notes", { q });

export const rebuildIndex = () =>
  safeInvoke<number>("rebuild_index");

// ──────────────────────────────────────────────────────────────────────────
// Phase 2 — AI providers
// ──────────────────────────────────────────────────────────────────────────

export const listProviders = () =>
  safeInvoke<ProviderInfo[]>("list_providers");

export const enableProvider = (
  name: string,
  enabled: boolean,
  configJson?: string | null,
) => safeInvoke<void>("enable_provider", {
  name,
  enabled,
  configJson,
});

export const testProvider = (name: string) =>
  safeInvoke<ProviderStatus>("test_provider", { name });

export const aiChat = (req: {
  messages: ChatMessage[];
  model: string;
  provider?: string;
}) => safeInvoke<StreamHandle>("ai_chat", { req });

export const aiComplete = (prompt: string, model?: string) =>
  safeInvoke<string>("ai_complete", { prompt, model });

export const aiSummarize = (text: string, style: string) =>
  safeInvoke<string>("ai_summarize", { text, style });

export const ragSearch = (query: string) =>
  safeInvoke<RagHit[]>("rag_search", { query });

export const aiTranscribe = (audioPath: string) =>
  safeInvoke<string>("ai_transcribe", { audioPath });

export const setProviderKey = (name: string, key: string) =>
  safeInvoke<void>("set_provider_key", { name, key });

export const deleteProviderKey = (name: string) =>
  safeInvoke<void>("delete_provider_key", { name });

export const hasProviderKey = (name: string) =>
  safeInvoke<boolean>("has_provider_key", { name });

export const providerKeyStatus = () =>
  safeInvoke<{ name: string; has_key: boolean }[]>("provider_key_status");

// ──────────────────────────────────────────────────────────────────────────
// Phase 4 — tasks
// ──────────────────────────────────────────────────────────────────────────

export const listTasks = () => safeInvoke<TaskSummary[]>("list_tasks");

export const createTask = (input: NewTask) =>
  safeInvoke<Task>("create_task", { input });

export const updateTask = (id: string, patch: TaskPatch) =>
  safeInvoke<Task>("update_task", { id, patch });

export const deleteTask = (id: string) =>
  safeInvoke<void>("delete_task", { id });

// ──────────────────────────────────────────────────────────────────────────
// Phase 4 — SRS
// ──────────────────────────────────────────────────────────────────────────

export const generateCards = (noteId: string, count: number) =>
  safeInvoke<DraftCard[]>("generate_cards", { noteId, count });

export const saveCards = (input: SaveCardsInput) =>
  safeInvoke<Card[]>("save_cards", { input });

export const srsQueue = (limit: number) =>
  safeInvoke<CardSummary[]>("queue", { limit });

export const reviewCard = (cardId: string, outcome: ReviewOutcome) =>
  safeInvoke<Card>("review", { cardId, outcome });

export const suspendCard = (cardId: string, suspended: boolean) =>
  safeInvoke<void>("suspend", { cardId, suspended });

// ──────────────────────────────────────────────────────────────────────────
// Phase 4 — quizzes
// ──────────────────────────────────────────────────────────────────────────

export const generateQuiz = (input: NewQuizInput) =>
  safeInvoke<Quiz>("generate_quiz", { input });

export const gradeQuiz = (quiz: Quiz, answers: string[]) =>
  safeInvoke<QuizResult>("grade_quiz", { quiz, answers });

// ──────────────────────────────────────────────────────────────────────────
// Phase 4 — weekly plan
// ──────────────────────────────────────────────────────────────────────────

export const generatePlan = (weekStart: string, dailyHoursCap: number) =>
  safeInvoke<Plan>("generate_plan", { weekStart, dailyHoursCap });

export const savePlan = (plan: Plan) =>
  safeInvoke<string>("save_plan", { plan });

export const getPlan = (weekStart: string) =>
  safeInvoke<Plan | null>("get_plan", { weekStart });

// ──────────────────────────────────────────────────────────────────────────
// Phase 5 — sync
// ──────────────────────────────────────────────────────────────────────────

export const syncExportZip = () =>
  safeInvoke<number[]>("sync_export_zip");

export const syncImportZip = (bytes: number[]) =>
  safeInvoke<number>("sync_import_zip", { bytes });

export const syncRebuildNow = () =>
  safeInvoke<number>("sync_rebuild_now");

// ──────────────────────────────────────────────────────────────────────────
// Phase 6 — calendar
// ──────────────────────────────────────────────────────────────────────────

export const calendarAuthStatus = () =>
  safeInvoke<CalendarAuthStatus>("calendar_auth_status");

export const calendarConnect = () =>
  safeInvoke<void>("calendar_connect");

export const calendarDisconnect = () =>
  safeInvoke<void>("calendar_disconnect");

export const calendarPull = (days: number) =>
  safeInvoke<number>("calendar_pull", { days });

export const calendarCreate = (
  summary: string,
  description: string,
  startIso: string,
  endIso: string,
) => safeInvoke<CalendarEvent>("calendar_create", {
  summary,
  description,
  startIso,
  endIso,
});

export const calendarList = () =>
  safeInvoke<CalendarEvent[]>("calendar_list");

// ──────────────────────────────────────────────────────────────────────────
// Phase 6 — drawing attachments (Excalidraw)
// ──────────────────────────────────────────────────────────────────────────

/** Write the SVG (display) and `.excalidraw` JSON (re-editable) blobs for a
 *  drawing. Returns the SVG's relative path under `notes_dir` — embed it as
 *  the markdown image src. */
export const saveDrawing = (input: {
  noteId: string;
  drawingId: string;
  svg: Uint8Array | number[];
  state: Uint8Array | number[];
}) =>
  safeInvoke<string>("save_drawing", {
    noteId: input.noteId,
    drawingId: input.drawingId,
    svg: Array.isArray(input.svg) ? input.svg : Array.from(input.svg),
    state: Array.isArray(input.state)
      ? input.state
      : Array.from(input.state),
  });

/** Read the SVG bytes for a drawing by its relative path. Used by the editor
 *  to refresh the inline preview after a re-export (not currently needed but
 *  kept for parity with the backend API). */
export const readDrawing = (path: string) =>
  safeInvoke<Uint8Array>("read_drawing", { path });

/** Read the `.excalidraw` JSON to re-open a drawing in the editor. */
export const readDrawingState = (path: string) =>
  safeInvoke<Uint8Array>("read_drawing_state", { path });

export const deleteDrawing = (path: string) =>
  safeInvoke<void>("delete_drawing", { path });

// ──────────────────────────────────────────────────────────────────────────
// Legacy alias — keep so any caller still importing `WireError` type keeps
// compiling. The throw-e-as-WireError pattern is gone (safeInvoke returns a
// union instead), but the type alias is harmless to keep around.
// ──────────────────────────────────────────────────────────────────────────
export type { WireError };