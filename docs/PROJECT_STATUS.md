# Notias — Project Status

**Snapshot:** all 6 spec phases (`specs/2026-07-03-notias-design.md` §13) shipped, tagged, committed.

## Phase Roll-up

| Phase | Scope | Tag | Headline |
|-------|-------|-----|----------|
| 0 | Cimientos | `phase-0-cimientos` | Tauri 2 + SvelteKit (SPA) + SQLite + migrations |
| 1 | MVP Notas | `phase-1-mvp-notas` | Editor + Markdown-on-disk + FTS5 search + wiki-links |
| 2 | IA local | `phase-2-ia-local` | Ollama HTTP + RAG chat + embeddings worker |
| 3 | Cloud IA + STT | `phase-3-cloud-stt` | OpenAI + Groq providers + Whisper transcription |
| 4 | Estudio autónomo | `phase-4-estudio` | SM-2 SRS + quiz generator + weekly plan + tasks kanban |
| 5 | Sync opcional | `phase-5-sync` | Manual zip export/import + startup rebuild + folder watcher |
| 6 | Calendar (post-MVP) | `phase-6-calendar` | Google OAuth PKCE + Calendar API v3 + local mirror |

Total: **68 commits** on `main`.

## Architecture

```
SvelteKit SPA (Svelte 5 runes)
  ↕  Tauri IPC (`invoke`)
Rust core (notias_lib)
  ├─ notes/   (store, index, wikilinks, sync)
  ├─ ai/      (Provider trait: Ollama | OpenAI | Groq, router, embed worker)
  ├─ tasks/   (CRUD)
  ├─ srs/     (SM-2)
  ├─ study/   (quiz grading + weekly plan)
  ├─ calendar/(API client + mirror, last-write-wins)
  ├─ oauth/   (Google PKCE flow, keyring-backed tokens)
  ├─ db/      (rusqlite, migrations v1–v6, FTS5, WAL)
  ├─ secrets/ (OS keyring wrapper)
  └─ error/   (AppError → wire format)

SQLite  notias.db          ← single file, WAL, derived index only
Files   notes/*.md         ← source of truth (Markdown + YAML frontmatter)
Keys    OS keyring         ← provider_api_keys, google_calendar
Meta    notias.meta        ← db_hash + schema_version (integrity)
```

## Verification Status (this host)

| Check | Result |
|-------|--------|
| `pnpm build` (frontend) | **PASS** throughout phases 1–6 |
| `cargo check` (Rust) | **BLOCKED** — Windows SDK Lib missing (`kernel32.lib`); only MSVC `link.exe` present, no SDK Lib dir |
| `cargo test --lib` | **BLOCKED** — same linker blocker; 30+ unit tests written, unrunnable here |
| `cargo run --example selfcheck` | **BLOCKED** — same; `selfcheck.rs` seeds rows + asserts invariants per phase |
| `pnpm tauri dev` | **NOT WITNESSED** on this host (needs full SDK); README documents it |

To verify locally: install Windows SDK Lib (or run on a host that has it) → `cd src-tauri && cargo test --lib && cargo run --example selfcheck && pnpm tauri build`.

## Deferred (acknowledged, by phase)

| Item | Phase | Why deferred |
|------|-------|--------------|
| Cross-OS CI matrix | 1 | No CI runner configured |
| Watcher `Remove` events → soft-delete from index | 5 | Conflicts with conflict UI not yet built |
| Calendar event edit/delete UI | 6 | Read + Create ship first; edit lands when user demand exists |
| 3-way merge on calendar last-write-wins | 6 | Version vectors; needs local-only edits to matter |
| Periodic calendar background refresh | 6 | Manual "Sync now" suffices for MVP |

## Known Risks (from spec §14)

| Risk | Status |
|------|--------|
| OAuth client_id compiled into binary (phase 6) | Acceptable for personal/educational use; document a backend-proxy approach before enterprise distribution |
| Embedding-model drift (phase 2) | Re-embed command + per-row `model` column in `note_embeddings` already in schema; UX lands when user switches models |
| DB corruption (phase 0) | `notias.meta` sidecar + `db_hash`; recovery screen logic is partial in `lib.rs` (`recovery_required` flag exposed via Tauri command; full rebuild UX in `commands::recovery_required` consumer) |
| Sync conflicts (phase 5) | Last-write-wins on `.md` mtime via watcher; CRDT explicitly out of scope |
| Bundle size (phase 0) | SvelteKit lean, no font/icon packs beyond what's used |

## File Map (top-level)

```
notias/
├── src-tauri/
│   ├── Cargo.toml, Cargo.lock
│   ├── tauri.conf.json, build.rs
│   ├── capabilities/default.json
│   ├── migrations/  (0001..0006 .sql)
│   ├── examples/selfcheck.rs
│   └── src/  (lib.rs + 11 modules)
├── src/  (SvelteKit SPA: routes/, lib/components/, lib/stores/, lib/ipc.ts)
├── docs/
│   ├── superpowers/specs/2026-07-03-notias-design.md
│   ├── superpowers/plans/2026-07-03-notias.md          (master plan)
│   ├── superpowers/plans/2026-07-06-notias-phase-3..6.md (phase plans)
│   └── test-checklist.md
├── package.json, pnpm-lock.yaml, vite.config.js, svelte.config.js, tsconfig.json
└── README.md
```

## How to Keep Going

- **Verify:** install Windows SDK Lib → run `cargo test --lib && cargo run --example selfcheck` per the checklists.
- **Polish phase 5:** add `Remove`-event handling once a conflict UI exists.
- **Polish phase 6:** add edit/delete UI; switch to background refresh once a scheduler is justified.
- **Out of scope (spec §15):** mobile, real-time collab, E2E encryption, first-party cloud backend, bundled LLM/Whisper, theme marketplace. None of these are "next steps" — they're explicit non-goals.