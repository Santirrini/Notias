# Phase 0 smoke checklist

- [ ] `pnpm install` succeeds
- [ ] `cd src-tauri && cargo run --example selfcheck` prints "selfcheck OK" (requires MSVC Build Tools / link.exe on Windows)
- [ ] `pnpm tauri dev` opens a window with sidebar + "Today" page showing "Backend says: pong"
- [ ] Sidebar navigation works: Today shows ping result, Chat and Settings show placeholder pages, Notes 404s (Phase 1 scope)
- [ ] `pnpm tauri build` produces an artifact for the current OS (requires MSVC Build Tools on Windows)

# Phase 1 smoke checklist

- [ ] Create 5 notes via UI; close/reopen app; all 5 persist
- [ ] Edit a note; reload page; content preserved
- [ ] Search "borrow" returns notes containing that word
- [ ] Add `[[Some Title]]` in a note; save; reopen file on disk shows updated frontmatter `links: ["Some Title"]`
- [ ] Delete `notias.db`; restart app; banner shown; click rebuild; notes reappear
- [ ] `pnpm tauri build` produces artifact for current OS

# Phase 2 smoke checklist (IA local)

- [ ] App starts with Ollama running: `/settings` shows Ollama healthy.
- [ ] `/settings`: toggle Ollama off, "Test connection" reports unhealthy.
- [ ] Edit a note; wait 6s; sqlite row in `note_vec` + `notes.embedding_status='ready'`.
- [ ] `/chat` ask a question; tokens stream; citations list appears.
- [ ] Stop Ollama; `/chat` shows error banner; app does not crash.
- [ ] Editor: ✨ Suggest button shows popover; Tab accepts, Esc dismisses.
- [ ] OLLAMA_TEST_URL=http://127.0.0.1:11434 cargo run --example selfcheck → healthy=true.

## Phase 3 — Cloud IA + STT

- [ ] Settings shows three provider cards (Ollama, OpenAI, Groq).
- [ ] Toggle provider → "enabled" pill flips, app does NOT need restart.
- [ ] Paste a fake OpenAI key, click Save → "key set" badge appears; key not visible in any log line (check `tracing` output if `RUST_LOG=debug`).
- [ ] Reload after Save: provider_key_status returns `{has_key: true}`; ai_chat routes through OpenAI when Ollama disabled.
- [ ] Disable Ollama + enable OpenAI with a real key → chat replies via OpenAI (verify by selecting "OpenAI" in /chat dropdown).
- [ ] Open a note → Record in toolbar → record 5–30s → transcript appears + clicking Insert appends to doc.
- [ ] `pnpm tauri build` still produces an artifact (capability additions for `commands_secrets:*` and `ai:allow-ai-transcribe` don't break the manifest).
- [ ] `cargo run --example selfcheck` reports schema v4.

## Phase 4 — Estudio autónomo

- [ ] `/tasks`: drag a card across columns → status updates without reload.
- [ ] `/tasks`: add a new task via input + Enter → row appears in `todo` column.
- [ ] `/study` Today: queue loads; Reveal → grade (Again/Hard/Good/Easy) advances; SM-2 interval grows on Good.
- [ ] `/study` Today: pass `?note=<id>` in URL → "Generate 5 cards" button appears + draft cards show.
- [ ] `/study` Quizzes: paste 2 note IDs + Generate → answer questions + Submit → score + per-question rationale visible.
- [ ] `/study` Plan: Regenerate writes `study_sessions.plan_json` with `week_start` of current Monday; dropdown moves a block across days.
- [ ] Disable Ollama, enable Groq or OpenAI: SRS generation, quiz, and plan still work via cloud.
- [ ] `cd src-tauri && cargo run --example selfcheck` prints `phase-4 selfcheck OK: tasks=1 srs_due>=3 plan_persisted=true` (tasks seeded by selfcheck; 3 SRS cards seeded).
- [ ] `pnpm tauri dev` opens window; sidebar nav shows Tasks + Study links.
- [ ] `cargo test --lib` passes including all 7 SM-2 tests + 4 quiz grading tests + 4 plan validation tests.

## Phase 5 — Sync opcional

- [ ] `/settings` Sync section: "Export notes.zip" downloads a zip containing every `.md` from the data dir.
- [ ] Extract that zip manually and confirm it contains only `.md` files (no `.db`, no `.meta`).
- [ ] Delete two notes from disk; restart app; "Rebuild index" repopulates the list (or the startup hook rebuilds automatically when their mtime > db mtime).
- [ ] On machine B, replace `notes/` with the unzipped folder; restart; all notes appear in the tree.
- [ ] With the app running, edit a `.md` in another process (text editor) — within ~1s the change is reflected in the UI list/search.
- [ ] `/settings` Import zip: pick the exported zip; status reports "Imported N files + rebuilt index".
- [ ] `/settings` Import zip: feed a malicious zip with `../escape.md` — UI shows an error, no file escapes the `notes/` folder.
- [ ] `cargo run --example selfcheck` prints `phase-5 selfcheck OK: zip roundtrip + stale detection wired`.
- [ ] `cargo test --lib notes::sync` passes all 4 tests.
- [ ] `pnpm tauri build` still produces an artifact (no new Tauri permissions needed for sync — IPC reads/writes the notes dir via Rust, not the frontend).

## Phase 6 — Google Calendar

- [ ] Register an OAuth client at console.cloud.google.com (type: Web application, redirect URI: `http://127.0.0.1:PORT/callback` with any port).
- [ ] Edit `src-tauri/src/oauth/google.rs` and set `OAUTH_CLIENT_ID` to your client id; rebuild.
- [ ] `/settings` → Calendar section: "Connect…" opens browser, prompts Google sign-in, redirects back; status pill flips to "connected".
- [ ] `/calendar` → "Sync now (next 7 days)" pulls events from your primary Google calendar; list populates within 2s.
- [ ] `/calendar` → Create event: summary + date + start/end → new event appears in list; verify same event exists in Google Calendar web UI.
- [ ] Edit an event in Google Calendar web UI → click "Sync now" → local list reflects the change (last-write-wins).
- [ ] `cargo run --example selfcheck` prints `phase-6 selfcheck OK: pkce + calendar client + mirror schema`.
- [ ] `cargo test --lib oauth::google calendar::google` passes.
- [ ] `pnpm tauri build` still produces an artifact.
