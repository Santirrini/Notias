# Notias — Design Spec

**Date:** 2026-07-03
**Status:** Draft → Review

## 1. Purpose

Notias is a cross-platform (macOS, Windows, Linux) desktop app for university
students that combines fast Markdown note-taking with integrated AI
(local-first via Ollama, cloud fallback) and self-directed study tools
(SRS flashcards, auto-generated quizzes, weekly study plan). The product is
local-first: data lives on disk as plain `.md` files; an SQLite index makes
search and embeddings fast; cloud is opt-in.

## 2. Goals & Non-Goals

### Goals

- Multi-OS native package (.dmg / .msi / .AppImage / .deb).
- High performance: cold start < 1s on a mid-range laptop; idle RAM < 250MB.
- Minimalist: single binary + standard assets; no servers required to run.
- AI features that work fully offline when Ollama is installed and configured.
- Local-first storage; sync is opt-in and best-effort.

### Non-Goals

- Mobile clients (out of scope; possible future).
- Real-time collaborative editing.
- A cloud-hosted backend operated by us. (Google Calendar uses Google's API;
  sync to cloud storage uses the user's own drive.)
- Bundling an LLM inside the app. Ollama is an external dependency.
- Bundling Whisper. STT is cloud-only via Groq in MVP.

## 3. Personas

- **Student (primary):** undergraduate juggling lectures, readings, and
  revision. Wants fast capture, AI summaries, study automation. Owns one
  laptop; occasionally a second machine (desktop at home).
- **Tutor / TA (secondary):** captures notes for others, generates
  flashcards from shared materials. Uses summarization and quiz generation.

## 4. Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│ SvelteKit SPA (no SSR)                                       │
│  Editor (Milkdown) · Chat panel · Calendar · Tasks · Search  │
└───────────────────────────┬──────────────────────────────────┘
                            │ Tauri IPC (`invoke`)
┌───────────────────────────▼──────────────────────────────────┐
│ Rust core — Tauri commands                                   │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐         │
│  │ notes    │ │ ai_      │ │ scheduler│ │ calendar │         │
│  │ store    │ │ bridge   │ │  (cron)  │ │ (oauth)  │         │
│  └────┬─────┘ └────┬─────┘ └──────────┘ └──────────┘         │
│       │              │                                       │
│  ┌────▼────────┐ ┌───▼────────────────────────┐              │
│  │ SQLite      │ │ Providers                 │              │
│  │ - notes idx │ │ - OllamaProvider          │              │
│  │ - FTS5      │ │ - OpenAIProvider          │              │
│  │ - sqlite-vec│ │ - GroqProvider (incl STT) │              │
│  └─────────────┘ └────────────────────────────┘              │
└──────────────────────────────────────────────────────────────┘
```

### Boundary principles

- **Markdown files are the source of truth.** SQLite is a derived index.
  Deleting the index file is recoverable; deleting the folder is data loss.
- **AI bridge is a Rust `trait Provider`.** UI never names a vendor.
- **Tauri commands are the only API surface.** No shared types between
  Rust and JS beyond serde-defined payloads.
- **No background threads cross module boundaries without channels.**
  Each subsystem owns its tasks; the scheduler owns cross-subsystem timers.

## 5. Data Model

### Filesystem (source of truth)

```
<user-data-dir>/
  notes/
    <note-id>.md           # one file per note
    attachments/<id>/      # images, PDFs, audio referenced by notes
  notias.db                # SQLite (index + embeddings + tasks + srs)
  notias.meta              # sidecar: SHA-256 of notias.db + schema version (see §11)
  config.json              # user preferences (non-secret)
  secrets/                 # OS-keyring-backed; no plaintext here
```

Markdown frontmatter per file:

```yaml
---
id: ulid
title: "..."
tags: [lecture, cs101]
created: 2026-07-03T10:14:00Z
updated: 2026-07-03T10:14:00Z
links: []             # resolved wiki-link targets; resolved on blur (see §7)
references: []        # attachments/<id>/... paths; resolved on blur (see §7)
---
```

### SQLite schema (single file, WAL mode)

```
notes(
  id PK, path UNIQUE, title, created, updated,
  deleted_at NULL,
  embedding_status TEXT NOT NULL DEFAULT 'pending'  -- pending|ready|failed
)
notes_fts(title, body, content='')  -- FTS5 virtual table
note_tags(note_id, tag)
note_embeddings(note_id PK, vec BLOB, model TEXT)  -- sqlite-vec, dim=768; model supports drift detection (see §14)
note_attachments(note_id, path, kind)  -- kind: image|file|audio
tasks(id PK, note_id NULL, title, due NULL, done_at NULL, priority)
srs_cards(id PK, note_id, front, back, ease, interval, due)
study_sessions(id PK, started, ended NULL, plan_json)
quiz_attempts(id PK, session_id, score, total, taken_at)
calendar_events(id PK, gcal_id NULL, starts_at, ends_at, summary)
provider_settings(provider PRIMARY KEY, model, enabled, last_health)
audit_log(id PK, ts, actor, action, payload_json)
```

Migrations live in `core/src/db/migrations/` as numbered `.sql` files
applied at startup in a single transaction.

### Embeddings

- Default model: `nomic-embed-text` via Ollama (768 dims).
- Embedding generation runs in a Tokio task pool, debounced 5s after note
  save. Failures retry with backoff up to 3 times; persistent failure flags
  the note `embedding_status='failed'` and is surfaced in UI.
- Re-embed full corpus command in Settings.

## 6. AI Bridge

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn health(&self) -> Result<ProviderStatus>;
    async fn complete(&self, req: CompleteRequest) -> Result<Completion>;
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse>;
    async fn embed(&self, inputs: &[String]) -> Result<Vec<Vec<f32>>>;
    async fn transcribe(&self, audio_path: &Path) -> Result<Transcript>;
    async fn summarize(&self, text: &str, style: SummaryStyle) -> Result<String>;
}
```

Implementations:

- **OllamaProvider:** HTTP against `http://127.0.0.1:11434` (configurable).
  Detected on app start; if absent, marked `enabled=false`. Supports
  complete, chat, embed. Transcribe returns `Unsupported`.
- **OpenAIProvider:** OpenAI Chat Completions + Embeddings API.
- **GroqProvider:** Groq Chat + Whisper (`whisper-large-v3-turbo`).

### Routing rules

- Default `local` group → Ollama if healthy, else fallback to first enabled
  cloud provider.
- Tasks that require a non-Ollama feature (transcribe) route explicitly to
  the configured cloud provider.
- User can pin a provider per action in Settings (advanced).

### Streaming

All chat/complete responses stream via Tauri events (`provider://stream/<id>`).
Frontends never block-await full completions.

### Privacy

- Telemetry: none by default. Crash reports opt-in only.
- Cloud providers receive only what the user submits. Body of notes is
  never auto-uploaded; cloud is invoked only on explicit user action
  (button click, command, scheduled task the user enabled).

## 7. Frontend (SvelteKit)

### Routes (SPA)

- `/` — Today (recent notes, today's tasks, due SRS cards).
- `/notes` — Tree + list of all notes.
- `/notes/:id` — Editor view (Milkdown WYSIWYG Markdown).
- `/chat` — Full-screen chat with RAG over the user's notes.
- `/tasks` — Kanban + calendar view (internal events).
- `/study` — SRS review session + quiz runner.
- `/settings` — Providers, models, embeddings, shortcuts, sync.

### Editor

- **Milkdown** (`@milkdown/core`) in WYSIWYG mode; underlying model is
  Markdown. Saving serializes to `.md` and writes via Tauri command.
- `[[wiki-link]]` and `![](attachments/...)` resolved on blur: shows
  autocomplete of note titles for links; missing attachments surface a
  broken-image marker. Creates stub notes on link click.
- Autosave: 1.5s debounce after last keystroke; on save the file is
  written via a temp-file + atomic rename.
- External change detection: before each save the editor re-reads the
  file's `updated` timestamp; if it changed externally, the editor shows
  a 3-way merge banner (keep mine / take theirs / open diff).
- Slash commands for headings, callouts, code, math (`$…$` via KaTeX).
- Inline AI suggestion: while typing, a debounced (700ms) request to
  provider for next-sentence completion; `Tab` to accept, `Esc` to dismiss.

### Chat panel

- Sidebar in `/notes/:id` route; expandable to full-screen at `/chat`.
- Always includes RAG: top-k=8 notes by `vec_distance_cosine` between query
  embedding and note_embeddings. Optionally pinned context (current note).
- Citations: each assistant message lists note titles it referenced; click
  to open.

## 8. Study Automation

### SRS (Anki-style)

- Cards are generated per-note on user command or auto-suggested when a
  note ends with a "Definition:" or numbered list pattern (heuristic,
  opt-in).
- Scheduler: SM-2 variant. `due` advances on review (Again/Hard/Good/Easy).
- Daily cap: 50 new cards/day (configurable). UI shows queue length.

### Quizzes

- Source: one or more notes selected by user.
- Question types: multiple choice, short answer, cloze deletion.
- Generated via provider with a constrained prompt; user sees raw JSON
  before saving. Generation runs in background; UI shows progress.
- Score + per-question explanation persisted in `quiz_attempts`.

### Weekly study plan

- Input: tasks with due dates, upcoming calendar events (Phase 6), recent
  notes, SRS backlog.
- Provider returns a structured plan (study blocks per day, capped at
  user-configured daily hours). Plan saved as `study_sessions.plan_json`
  with shape:
  ```json
  {
    "week_start": "2026-07-06",
    "daily_hours_cap": 4,
    "blocks": [
      {"day": "2026-07-06", "start": "09:00", "minutes": 60,
       "kind": "review", "refs": ["note:<id>"], "rationale": "..."}
    ]
  }
  ```
- Editable; user can drag blocks to reassign days.

## 9. Tauri Commands (public IPC surface)

All commands return `Result<T, AppError>` where `AppError` serializes to
`{code, message, detail?}`.

```
notes.list(query?, tag?, limit?, offset?) -> NoteSummary[]
notes.get(id) -> Note
notes.create(input) -> Note
notes.update(id, patch) -> Note
notes.delete(id) -> ()
notes.search(q) -> NoteHit[]   // FTS5 ranked
ai.complete(req) -> StreamId
ai.chat(req) -> StreamId
ai.embed(note_id) -> ()   // emits `embedding_updated { note_id, status }` on completion
ai.summarize(note_id, style) -> String
ai.transcribe(audio_path) -> Transcript
tasks.*, srs.*, study.*, calendar.*   // analogous CRUD + actions
settings.get/set
providers.list, providers.health, providers.enable
```

Streaming uses Tauri events. Frontends subscribe via `listen()` and
unsubscribe on unmount.

## 10. Cross-Platform Concerns

- **Paths:** all disk I/O goes through `directories` crate (ProjectDirs).
  No hardcoded `/` or `C:\` separators.
- **Keyring:** `tauri-plugin-stronghold` or OS keyring via `keyring` crate.
  Secrets never written to `config.json`.
- **Notifications:** `tauri-plugin-notification` for SRS reminders.
- **Auto-update:** `tauri-plugin-updater` against GitHub releases; off by
  default in dev builds.
- **Code signing:** required for macOS distribution and Windows SmartScreen.
  Notias ships an unsigned dev build first; signing is a release concern.

### Build artifacts

| OS | Artifact | Builder |
|---|---|---|
| macOS | `.dmg`, `.app` | `tauri build` (universal binary via lipo) |
| Windows | `.msi`, `.exe` | `tauri build` (WiX) |
| Linux | `.AppImage`, `.deb` | `tauri build` (linuxdeploy) |

## 11. Error Handling

- All Tauri commands return structured errors. Frontends display a toast and
  log to `audit_log` for non-fatal errors.
- Provider failures surface as a banner with retry. Repeated failures
  degrade gracefully (Ollama down → fallback to cloud; cloud down → read-only mode).
- Database corruption detected by checksum on startup. A `notias.meta`
  sidecar file stores the SHA-256 of the SQLite file plus the last-known-good
  schema version and timestamp. On startup the app re-hashes the `.db` and
  compares; mismatch opens a recovery screen offering to rebuild the index
  from `.md` files (delete `.db`, re-run migrations, re-scan folder, re-embed
  in background).

## 12. Testing Strategy

Per ponytail principle: leave one runnable check behind for non-trivial logic.

- **Unit tests** for: `Provider` trait impls (mocked HTTP), SQLite
  migrations, wiki-link resolver, SRS scheduler, FTS query builder.
- **Integration tests** for: Tauri command round-trip via `tauri::test`.
- **Self-check binary** (`core/examples/selfcheck.rs`): opens DB, runs
  migrations, creates a note, embeds (skipped if no Ollama), summarizes,
  deletes. Asserts each step. Runs in CI and pre-release.
- **Pure-logic unit tests** are mandatory for the SRS scheduler (SM-2
  interval math) — easy to drift, no I/O needed.
- **Frontend tests:** Vitest for store logic and chat reducer.
- **Manual e2e checklist** in `docs/test-checklist.md` (no Playwright in MVP).

## 13. Phases (Implementation Order)

| Phase | Deliverable | Exit criteria |
|---|---|---|
| **0. Cimientos** | Tauri 2 + SvelteKit (SPA mode) + SQLite + migrations scaffold | App launches on mac/win/linux; `selfcheck` passes; empty `/` route |
| **1. MVP Notas** | Editor (Milkdown), files on disk, FTS5 search, tree, tags, `[[wiki-links]]` | Create/edit/search/link notes; round-trip Markdown survives |
| **2. IA local** | `Provider` trait, Ollama wire, summarize, autocomplete, RAG chat | All AI commands work with Ollama running; offline test passes |
| **3. Cloud IA + STT** | OpenAI/Groq providers, keyring, Whisper transcription, class summary | Switch providers in Settings; record+transcribe a 30s clip end-to-end |
| **4. Estudio autónomo** | SRS, quiz generator + runner, weekly plan | Generate cards from note; complete SRS session; generate and run a quiz; produce a weekly plan |
| **5. Sync opcional** | Rely on the user's existing cloud-storage app (Drive/Dropbox/iCloud) to sync the `notes/` folder; Notias adds only manual export/import and a "rebuild index on startup" hook | Edit note on machine A; on machine B after the user's cloud app syncs, Notias detects new files and rebuilds index; content matches |
| **6. post-MVP** | Google Calendar OAuth (installed-app flow), read+write events | OAuth round-trip; events appear in both directions |

## 14. Open Questions / Risks

- **OAuth client secret in a desktop app:** Google's installed-app flow
  returns a refresh token, but the client id/secret still ships in the
  binary. Mitigations: (a) restrict OAuth client to internal testing first,
  (b) document unverification in production. Tracked for Phase 6.
- **sqlite-vec vs Qdrant:** starting with sqlite-vec. Re-evaluate if corpus
  > 50k notes or query p95 > 200ms.
- **Embedding model drift:** if user switches Ollama embedding model, the
  existing `note_embeddings` are incompatible. Re-embed flow must be
  obvious (Settings → Rebuild embeddings).
- **Offline editing + sync conflicts:** Phase 5 uses last-write-wins on
  the Markdown file. CRDTs are out of scope. **The `notias.db` index is
  never synced across machines**; each machine rebuilds its index from the
  synced `.md` files on startup.
- **Cloud storage OAuth (Phase 5):** Drive/Dropbox OAuth reuses the same
  installed-app + keyring pattern as Phase 6 (Google Calendar). The two
  phases share a `oauth` module in Rust; per-provider configuration
  lives in `provider_settings`.
- **Bundle size:** Tauri is small; the dominant cost will be the
  frontend assets. Keep SvelteKit lean; no font/icon packs beyond what's
  used.

## 15. Out of Scope (explicit)

- Mobile clients.
- Real-time collaboration.
- E2E encryption of notes.
- A first-party cloud backend.
- Bundled LLM or Whisper.
- Custom theming store / marketplace.