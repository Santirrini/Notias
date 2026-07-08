# Notias · UI redesign — verification checklist

Snapshot: phase-7-ui-redesign (post-phase-6). The 3-panel OneNote chrome ships; everything below must pass.

## Static checks (host-side)

```bash
pnpm install
pnpm check          # expect: 0 errors, 1 svelte warning (state_referenced_locally in NoteCanvas — intentional)
pnpm build          # expect: build OK, build/ emitted
```

If `pnpm check` reports more than the 1 known warning, treat as a regression.

## Smoke (manual, in `pnpm tauri dev` or `pnpm dev`)

Layout chrome

- [ ] App opens into a 3-column shell on `/notes`: NavRail (left) · Sections (200 px) · Pages (280 px) · Main.
- [ ] NavRail items (Notes, Chat, Calendar, Tasks, Study, Settings) are icons-only when collapsed, icons+labels when expanded.
- [ ] Collapsing NavRail via the chevron button at the bottom works and persists during the session.
- [ ] Sections panel shows the notebook name "Notias" at the top, then a list of sections with a colored bar at the left edge.
- [ ] Pages panel header shows the current section name, a colored dot, and a count badge; clicking "+ New" creates a fresh page and routes to `/notes/<id>`.
- [ ] Clicking a page navigates to its editor; the editor uses the warm "paper" canvas (`#FFFBF1` light / `#2B2B2B` dark) with a single vertical guide line at center.

Sections & grouping

- [ ] On first load with notes tagged differently, sections are auto-seeded from unique tags with rotating accent colors.
- [ ] Notes without a known tag land in **General** (pinned, gray).
- [ ] Creating a new section via "New section" in Sections panel immediately switches the PageList to that section.
- [ ] Selecting a section updates the dot, name, and count in the Pages panel header.

Command palette

- [ ] `Ctrl+K` (Win/Linux) or `Cmd+K` (macOS) toggles the palette.
- [ ] `Esc` closes it.
- [ ] `/` opens it from anywhere except inside an `<input>`.
- [ ] Typing "new" shows the "Create new page" action with a `Ctrl N` hint and a Plus icon.
- [ ] Typing a note title filters the **Pages** group with matching notes.
- [ ] "Toggle theme", "Rebuild search index", "Export notes as zip", "Import notes from zip", "Go to Settings" all run their actions and close the palette.

Theme

- [ ] `Ctrl+Shift+L` (or the moon/sun button in Topbar) toggles light/dark.
- [ ] `localStorage["mode-watcher-mode"]` (mode-watcher's own key) is set; reload preserves the choice.
- [ ] Dark mode: background near-black, surfaces near-`#2B2B2B`, accent blue stays readable.
- [ ] Light mode: canvas is `#FFFBF1`, surface is white, accent is `#0078D4`.

Recovery

- [ ] When `recovery_required()` returns true (DB hash mismatch after a manual disk edit), a red banner appears at the top of the main column with "Index out of sync" and a Rebuild button.
- [ ] Clicking Rebuild calls `rebuildIndex()`; on success the banner disappears.

Search

- [ ] Topbar search input shows a dropdown after ~200 ms of typing with up to 10 matches (title or tag).
- [ ] Clicking a result navigates to that note and clears the query.

## Areas intentionally not in this pass

- Chat, Tasks, Study, Calendar, Settings pages still render in their previous styles; they ride along the new chrome but keep their own layouts.
- The Resizable panel group is not yet wired in (the three columns have fixed widths). Resizing is a follow-up; the chrome already accommodates 220/200/280 widths without overflow.
- 3-way notebook/section page hierarchy is **frontend-only**: stored in `localStorage` under `notias.sectionMap.v1`. There is no DB schema migration for it yet.

## Known regressions to keep an eye on

- Milkdown Nord theme lives inside the canvas; if you change `--canvas-bg` you'll want to keep contrast against the editor chrome (titles, hover cards).
- `SRSReviewer`, `QuizRunner`, `WeeklyPlan`, `TaskBoard`, `AudioRecorder` retain their old `<style>` blocks. The CSS variables they reference (`--bg-elevated`, `--accent`) still resolve via Tailwind theme bridge.
