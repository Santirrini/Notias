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

Phase 0 (scaffold) complete. `pnpm tauri dev` smoke not yet witnessed on this
host — Windows SDK install needed for full webview launch. `cargo check` and
`pnpm build` succeed.