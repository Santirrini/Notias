# Notias

Local-first Markdown notes with integrated AI.

## Dev

Requires: Rust stable, pnpm, Node 20+, Ollama (optional, for AI features).

```
pnpm install
pnpm tauri dev
```

## Build

```
pnpm tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`.

## Status

- Phase 0 (scaffold) complete.
- Phase 1 (MVP Notas) complete.
- Phase 2 (IA local) complete — Ollama HTTP + RAG chat + embeddings worker + settings UI shipped.
- Phase 3 (Cloud IA + STT) complete — OpenAI + Groq providers, keyring-backed API keys, router fallback (local→cloud), Whisper transcription via Groq.
- Phase 4 (Estudio autónomo) complete — SM-2 SRS with UI grader, quiz generator + runner (mc/short/cloze), weekly plan generator (AI via router fallback), tasks kanban. Verify with `pnpm tauri dev`; `cargo run --example selfcheck` exercises schema v5, SM-2, plan round-trip.
- Phase 5 (Sync opcional) complete — manual zip export/import, startup rebuild hook for externally-touched files, live folder watcher. Rely on your own cloud-storage app for `notes/` sync; `notias.db` is never shared. Verify with `pnpm tauri dev`; `cargo run --example selfcheck` exercises zip roundtrip + stale detection.
- Phase 6 (Google Calendar, post-MVP) complete — OAuth PKCE + Calendar API v3 client + local mirror with last-write-wins. User supplies their own OAuth client_id via `OAUTH_CLIENT_ID`; no client_secret in binary. Verify with `pnpm tauri dev`; `cargo run --example selfcheck` exercises PKCE + JSON round-trip + mirror schema.
- Phase 7 (UI redesign, post-MVP) complete — 3-panel chrome (NavRail · Sections · Pages · NoteCanvas) + CommandPalette (Ctrl+K) + theme toggle + RecoveryBanner + bits-ui primitives. Secret IPC merged into `commands.rs`; sections persisted to `localStorage["notias.sectionMap.v1"]` (frontend-only, no schema change). Static checks pass (`pnpm check` 0 errors, 1 warning intentional; `pnpm build` OK; MSI + NSIS bundles produced). Interactive smoke per `docs/test-checklist-ui.md`.

**Windows verification (2026-07-06):** `cargo check`, `cargo test --lib` (62/62 pass), `cargo run --example selfcheck`, `pnpm build`, and `pnpm tauri build` all pass on this host (MSVC toolchain, Windows SDK 10.0.28000, Build Tools 14.44). MSI and NSIS installers generated. See `docs/WINDOWS_BUILD.md` for the full setup and `docs/PROJECT_STATUS.md` for the phase roll-up.

## Google Calendar

Notias uses OAuth 2.0 PKCE so no client secret ships in the binary. To enable:

1. Create an OAuth client at https://console.cloud.google.com/.
   - Application type: **Web application**
   - Authorized redirect URI: `http://127.0.0.1:PORT/callback` (any port; Notias binds an ephemeral one)
2. Copy the client id and edit `src-tauri/src/oauth/google.rs`, replacing the placeholder in `OAUTH_CLIENT_ID`.
3. Rebuild (`pnpm tauri build`). The first "Connect…" click opens your browser to Google's auth page; after authorizing, the redirect back to 127.0.0.1 is captured automatically.

See `docs/PROJECT_STATUS.md` for the full phase roll-up + verification matrix.