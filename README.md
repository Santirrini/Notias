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
- Phase 3 (Cloud IA + STT) complete — OpenAI + Groq providers, keyring-backed API keys, router fallback (local→cloud), Whisper transcription via Groq. Verify with `pnpm tauri dev`, set OpenAI/Groq key in Settings, switch chat provider dropdown.

`pnpm tauri dev` smoke not yet witnessed on this host — Windows SDK install
needed for full webview launch. `cargo check` and `pnpm build` succeed.