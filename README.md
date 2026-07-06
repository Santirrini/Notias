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

`pnpm tauri dev` smoke not yet witnessed on this host — Windows SDK install
needed for full webview launch. `cargo check` and `pnpm build` succeed.