# Phase 0 smoke checklist

- [ ] `pnpm install` succeeds
- [ ] `cd src-tauri && cargo run --example selfcheck` prints "selfcheck OK" (requires MSVC Build Tools / link.exe on Windows)
- [ ] `pnpm tauri dev` opens a window with sidebar + "Today" page showing "Backend says: pong"
- [ ] Sidebar navigation works: Today shows ping result, Chat and Settings show placeholder pages, Notes 404s (Phase 1 scope)
- [ ] `pnpm tauri build` produces an artifact for the current OS (requires MSVC Build Tools on Windows)
