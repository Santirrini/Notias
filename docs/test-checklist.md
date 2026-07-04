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
