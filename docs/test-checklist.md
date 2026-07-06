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
