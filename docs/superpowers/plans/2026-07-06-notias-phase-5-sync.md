# Phase 5 — Sync opcional Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add opt-in sync — manual export/import of the `notes/` folder as a `.zip`, a startup rebuild hook for files touched externally (Drive/Dropbox/iCloud), and a live folder watcher — so editing on machine A surfaces on machine B after the user's cloud app syncs.

**Architecture:** New `notes/sync` Rust module owns three pure-ish functions (`export_zip`, `import_zip`, `rebuild_if_stale`) plus a notify-based background watcher. Three Tauri commands (`sync_export_zip`, `sync_import_zip`, `sync_rebuild_now`) expose the manual actions; the watcher is spawned from `lib.rs::run()` so its lifetime matches the app. Settings gains a `SyncSettings.svelte` section with Export / Import / Rebuild controls. `notias.db` is **never** synced — each machine rebuilds its own index from the synced `.md` files per spec §14.

**Tech Stack:** Tauri 2, Rust (edition 2021), `notify` 6 (folder watch), `zip` 0.6 (store + deflate), Svelte 5 runes, existing SQLite + ulid.

**Spec:** `docs/superpowers/specs/2026-07-03-notias-design.md` §13 row 5 + §14
**Master plan:** `docs/superpowers/plans/2026-07-03-notias.md` Chunk 6

---

## File Structure (delta only)

```
src-tauri/
├── Cargo.toml                                   MOD — add `notify` + `zip`
├── src/
│   ├── lib.rs                                   MOD — spawn watcher thread; startup rebuild_if_stale
│   ├── notes/
│   │   ├── mod.rs                               EXTEND — 3 new tauri commands
│   │   └── sync.rs                              NEW — export_zip / import_zip / rebuild_if_stale + watcher
│   └── ...                                      AS-IS
└── ...

src/
├── lib/
│   ├── ipc.ts                                   EXTEND — 3 sync bindings
│   ├── types.ts                                 EXTEND — SyncStatus type
│   └── components/
│       └── SyncSettings.svelte                  NEW — Export / Import / Rebuild controls
└── routes/
    └── settings/+page.svelte                    EXTEND — mount SyncSettings
```

**Decomposition:**
- `sync.rs` is one module with three small public fns + a `Watcher` struct — total scope fits a single file, no subdir.
- Watcher thread owns its own `Arc<Mutex<Connection>>` cloned from `state.db`; no new state added to `AppState` (the join handle is dropped — thread lives until app exits).
- Export/import over IPC as `Vec<u8>` — no Tauri dialog plugin dep; frontend uses `<input type="file">` + blob URLs.

---

## Decisions Locked

- **No cloud-storage OAuth.** Spec §13 row 5 is explicit: "Rely on the user's existing cloud-storage app." Phase 6 reuses the OAuth pattern for Google Calendar. Spec §14's "Cloud storage OAuth (Phase 5)" risk is deferred.
- **Last-write-wins** on the Markdown file (spec §14). The watcher upserts by file content, not by mtime conflict resolution; if both machines edited the same note since the last sync, the newer file (by mtime) wins when the watcher re-indexes it.
- **Export scope:** `notes/` only. No `.db`, no `.meta`, no `attachments/` from `<data>/attachments` if any (none in current build). Encrypted secrets never cross IPC.
- **Import behavior:** additive. Files in the zip are extracted into `notes/` overwriting by name. No deletion of files present locally but absent in the zip — that would lose data if the zip is partial. After extraction, `rebuild_index` runs.
- **Watcher debounce:** 500ms per-path coalescing window. Bursts of editor saves collapse into one reindex.
- **Watcher thread:** std thread with a `mpsc::Receiver<DebouncedEvent>` loop. Locks `state.db` mutex per upsert (matches existing pattern in `rebuild_index`).
- **Startup rebuild_if_stale trigger:** walk `notes/` for any `.md` whose mtime is newer than the DB file's mtime; if any, call `rebuild_index` before the first UI render. Cheap on small corpora; spec §13 row 5 exit criterion is "after the user's cloud app syncs" → the syncing app touches mtimes → hook fires.
- **No new frontend dependencies.** `<input type="file">` + FileReader + Blob URL are web standards; the Tauri webview supports them.

---

## Task Inventory

| Task | Subject | Files |
|------|---------|-------|
| T58 | Deps + capability | Cargo.toml, capabilities/default.json |
| T59 | sync.rs module + unit tests | notes/sync.rs |
| T60 | Tauri commands | notes/mod.rs, notes/sync.rs |
| T61 | lib.rs wiring (watcher + startup rebuild) | lib.rs |
| T62 | SyncSettings.svelte component | components/SyncSettings.svelte |
| T63 | ipc.ts + types + settings route mount | ipc.ts, types.ts, routes/settings/+page.svelte |
| T64 | selfcheck extension + smoke checklist + README + tag | examples/selfcheck.rs, docs/test-checklist.md, README.md |

---

### Task T58: Add `notify` + `zip` deps

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add deps**

Append to `[dependencies]`:

```toml
notify = "6"
zip = { version = "0.6", default-features = false, features = ["deflate"] }
```

- [ ] **Step 2: Verify compile**

Run: `cd src-tauri && cargo check`
Expected: success — both crates compile on stable.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "build(phase-5): notify + zip deps"
```

---

### Task T59: `notes/sync.rs` — pure functions + tests

**Files:**
- Create: `src-tauri/src/notes/sync.rs`

- [ ] **Step 1: Write the module skeleton with tests**

```rust
// Phase 5 — sync: zip export/import + rebuild_if_stale + notify watcher.
// Spec §13 row 5: rely on the user's cloud-storage app; we only do manual
// export/import + a startup rebuild hook + live watch for external edits.

use crate::error::{AppError, AppResult};
use crate::notes::{index, model::Note, store};
use rusqlite::Connection;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Read every `.md` under `notes_dir` into an in-memory zip. Returns the bytes.
/// Files are stored with paths relative to `notes_dir` (so root contains the
/// first path segment, e.g. `notes/abc/def.md` → `def.md` if nested — current
/// layout is flat so this is just the basename).
pub fn export_zip(notes_dir: &Path) -> AppResult<Vec<u8>> {
    if !notes_dir.exists() {
        return Err(AppError::NotFound(format!("notes dir {:?}", notes_dir)));
    }
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for entry in walk_md(notes_dir)? {
            let rel = entry
                .strip_prefix(notes_dir)
                .unwrap_or(&entry)
                .to_string_lossy()
                .replace('\\', "/");
            zip.start_file(&rel, opts)?;
            let mut f = File::open(&entry)?;
            let mut chunk = [0u8; 8192];
            loop {
                let n = f.read(&mut chunk)?;
                if n == 0 { break; }
                zip.write_all(&chunk[..n])?;
            }
        }
        zip.finish()?;
    }
    Ok(buf)
}

/// Extract every entry from `bytes` into `notes_dir`, overwriting by name.
/// Returns the count of files written. Does NOT delete existing files
/// absent from the zip (additive; safe for partial imports).
pub fn import_zip(notes_dir: &Path, bytes: &[u8]) -> AppResult<usize> {
    std::fs::create_dir_all(notes_dir)?;
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor)?;
    let mut count = 0usize;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        // Skip directory entries; we make parent dirs on demand.
        if entry.is_dir() { continue; }
        // Reject path traversal: zip entry name must be a relative path with
        // no `..` segments and no leading `/`.
        let name = entry.name().replace('\\', "/");
        if name.contains("..") || name.starts_with('/') {
            return Err(AppError::Invalid(format!("unsafe zip entry: {name}")));
        }
        let out = notes_dir.join(&name);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        std::fs::write(&out, buf)?;
        count += 1;
    }
    Ok(count)
}

/// True if any `.md` under `notes_dir` has mtime strictly newer than `db_file`'s mtime.
/// Returns false if `db_file` doesn't exist (fresh install → nothing to compare).
pub fn rebuild_if_stale(notes_dir: &Path, db_file: &Path) -> bool {
    let db_mtime = match std::fs::metadata(db_file).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    for entry in walk_md(notes_dir).unwrap_or_default() {
        let mt = match std::fs::metadata(&entry).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if mt > db_mtime {
            tracing::info!("external change detected: {}", entry.display());
            return true;
        }
    }
    false
}

fn walk_md(root: &Path) -> AppResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !root.exists() { return Ok(out); }
    for e in std::fs::read_dir(root)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() {
            out.extend(walk_md(&p)?);
        } else if p.extension().and_then(|s| s.to_str()) == Some("md") {
            out.push(p);
        }
    }
    Ok(out)
}

/// Background watcher: spawn a thread that watches `notes_dir` recursively,
/// debounces per-path events for `debounce`, then re-indexes each changed file.
/// Holds `db` mutex briefly per upsert (matches `rebuild_index` pattern).
/// Returns the thread JoinHandle for tests; production drops it.
pub fn spawn_watcher(
    notes_dir: PathBuf,
    db: Arc<Mutex<Connection>>,
    debounce: Duration,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        // ponytail: notify::recommended_watcher + manual 500ms debounce per path.
        // notify-debouncer-mini would do this in one crate but adds a dep for ~30 lines.
        use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
        let mut watcher: RecommendedWatcher = match RecommendedWatcher::new(
            move |res| { let _ = tx.send(res); },
            Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => { tracing::error!("watcher init failed: {e}"); return; }
        };
        if let Err(e) = watcher.watch(&notes_dir, RecursiveMode::Recursive) {
            tracing::error!("watch failed on {:?}: {e}", notes_dir);
            return;
        }
        // Debounce map: path → last event time.
        let mut last_seen: std::collections::HashMap<PathBuf, SystemTime> =
            std::collections::HashMap::new();
        while let Ok(res) = rx.recv() {
            let event = match res { Ok(e) => e, Err(e) => { tracing::warn!("watch error: {e}"); continue; } };
            if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)) {
                continue;
            }
            for path in event.paths {
                if path.extension().and_then(|s| s.to_str()) != Some("md") { continue; }
                let now = SystemTime::now();
                if let Some(prev) = last_seen.get(&path) {
                    if now.duration_since(*prev).unwrap_or(Duration::ZERO) < debounce { continue; }
                }
                last_seen.insert(path.clone(), now);
                // Hold the lock once per file. Index upsert + store read keep it short.
                let conn = match db.lock() {
                    Ok(c) => c,
                    Err(_) => { tracing::warn!("db lock poisoned"); continue; }
                };
                if matches!(event.kind, EventKind::Remove(_)) {
                    // ponytail: we don't soft-delete from watcher — the user may have
                    // temporarily moved the file. External deletion handling lands
                    // alongside a real conflict UI in a later phase.
                    tracing::info!("watcher: ignored remove for {}", path.display());
                    continue;
                }
                match store::read(&path) {
                    Ok(note) => {
                        if let Err(e) = index::upsert(&conn, &note) {
                            tracing::warn!("watcher upsert failed for {}: {e}", path.display());
                        } else {
                            tracing::debug!("watcher: reindexed {}", path.display());
                        }
                    }
                    Err(e) => tracing::warn!("watcher read failed for {}: {e}", path.display()),
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::model::Frontmatter;
    use tempfile::tempdir;

    fn make_note(dir: &Path, id: &str, body: &str) -> Note {
        Note {
            id: id.into(),
            path: dir.join(format!("{id}.md")),
            title: id.into(),
            body: body.into(),
            frontmatter: Frontmatter {
                id: id.into(),
                title: id.into(),
                tags: vec![],
                created: "2026-07-06T00:00:00Z".into(),
                updated: "2026-07-06T00:00:00Z".into(),
                links: vec![],
                references: vec![],
            },
        }
    }

    #[test]
    fn export_then_import_roundtrip() {
        let src = tempdir().unwrap();
        let dst = tempdir().unwrap();
        store::write(&make_note(src.path(), "alpha", "hello")).unwrap();
        store::write(&make_note(src.path(), "beta", "world")).unwrap();

        let bytes = export_zip(src.path()).unwrap();
        assert!(!bytes.is_empty());

        let dst_notes = dst.path().join("notes");
        let n = import_zip(&dst_notes, &bytes).unwrap();
        assert_eq!(n, 2);
        assert!(dst_notes.join("alpha.md").exists());
        assert!(dst_notes.join("beta.md").exists());

        let got = store::read(&dst_notes.join("alpha.md")).unwrap();
        assert_eq!(got.body, "hello");
    }

    #[test]
    fn import_rejects_path_traversal() {
        let src = tempdir().unwrap();
        // Build a zip with a `..` entry by hand.
        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            zip.start_file("../escape.md", zip::write::FileOptions::<()>::default()).unwrap();
            zip.write_all(b"pwned").unwrap();
            zip.finish().unwrap();
        }
        let dst = tempdir().unwrap();
        let r = import_zip(&dst.path().join("notes"), &buf);
        assert!(r.is_err(), "must reject .. entry");
    }

    #[test]
    fn rebuild_if_stale_true_when_md_newer_than_db() {
        let dir = tempdir().unwrap();
        let notes = dir.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        let db = dir.path().join("notias.db");
        std::fs::write(&db, b"x").unwrap();

        // Touch db into the past.
        let past = SystemTime::now() - Duration::from_secs(10);
        filetime_set(&db, past);

        // Create a .md now (newer than db).
        std::fs::write(notes.join("new.md"), b"---\nid: n\ntitle: n\ncreated: c\nupdated: u\n---\nbody").unwrap();
        assert!(rebuild_if_stale(&notes, &db));
    }

    #[test]
    fn rebuild_if_stale_false_when_db_missing() {
        let dir = tempdir().unwrap();
        let notes = dir.path().join("notes");
        std::fs::create_dir_all(&notes).unwrap();
        std::fs::write(notes.join("a.md"), b"x").unwrap();
        let db = dir.path().join("missing.db");
        assert!(!rebuild_if_stale(&notes, &db));
    }

    fn filetime_set(p: &Path, t: SystemTime) {
        let ft = filetime::FileTime::from_system_time(t);
        filetime::set_file_mtime(p, ft).unwrap();
    }
}
```

- [ ] **Step 2: Add `filetime` dev-dep for the mtime test**

The `rebuild_if_stale_true_when_md_newer_than_db` test needs to backdate `notias.db`. The cleanest way without depending on `filetime` is `std::fs::File::set_modified`, which is stable since 1.75.

Replace the `filetime_set` helper with:

```rust
fn set_mtime(p: &Path, t: SystemTime) {
    let f = std::fs::OpenOptions::new().write(true).open(p).unwrap();
    f.set_modified(t).unwrap();
}
```

And update the test call from `filetime_set(&db, past)` → `set_mtime(&db, past)`.

(No new dep needed.)

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib notes::sync`
Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/notes/sync.rs
git commit -m "feat(notes): sync module — zip roundtrip + rebuild_if_stale + watcher"
```

---

### Task T60: Tauri commands for sync

**Files:**
- Modify: `src-tauri/src/notes/mod.rs`

- [ ] **Step 1: Add the three commands**

Append to `src-tauri/src/notes/mod.rs`:

```rust
#[tauri::command]
pub fn sync_export_zip(state: State<'_, AppState>) -> AppResult<Vec<u8>> {
    sync::export_zip(&state.paths.notes_dir)
}

#[tauri::command]
pub fn sync_import_zip(bytes: Vec<u8>, state: State<'_, AppState>) -> AppResult<usize> {
    let n = sync::import_zip(&state.paths.notes_dir, &bytes)?;
    // Refresh the index after import so search/list reflect new files immediately.
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let _ = index::rebuild_from_disk(&conn, &state.paths.notes_dir)?;
    let _ = crate::db::write_meta(
        &state.paths.meta_file,
        &crate::db::migrations::db_hash(&conn)?,
        crate::db::migrations::read_schema_version(&conn)?,
    );
    Ok(n)
}

#[tauri::command]
pub fn sync_rebuild_now(state: State<'_, AppState>) -> AppResult<usize> {
    // Same body as the existing rebuild_index command; kept as a separate IPC
    // name so the sync UI doesn't have to know about generic indexing.
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let n = index::rebuild_from_disk(&conn, &state.paths.notes_dir)?;
    let _ = crate::db::write_meta(
        &state.paths.meta_file,
        &crate::db::migrations::db_hash(&conn)?,
        crate::db::migrations::read_schema_version(&conn)?,
    );
    Ok(n)
}
```

Also add `pub mod sync;` and `use crate::notes::sync;` near the top of `notes/mod.rs`.

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/notes/mod.rs
git commit -m "feat(notes): sync Tauri commands (export_zip / import_zip / rebuild_now)"
```

---

### Task T61: lib.rs — spawn watcher + startup rebuild

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Spawn the watcher after state is built**

Right before `tauri::Builder::default()`:

```rust
// Phase 5 — sync: if any .md under notes/ has mtime newer than notias.db,
// the user's cloud app probably just synced new content. Rebuild before
// the UI renders so search/list are fresh.
if crate::notes::sync::rebuild_if_stale(&state.paths.notes_dir, &state.paths.db_file) {
    tracing::info!("notes/ has changes newer than notias.db; rebuilding index at startup");
    let conn = state.db.lock().expect("db lock");
    let _ = crate::notes::index::rebuild_from_disk(&conn, &state.paths.notes_dir);
    let _ = crate::db::write_meta(
        &state.paths.meta_file,
        &crate::db::migrations::db_hash(&conn).unwrap_or_default(),
        crate::db::migrations::read_schema_version(&conn).unwrap_or(0),
    );
}

// Background folder watcher — re-indexes external edits in place.
let watch_db = Arc::new(Mutex::new(unsafe {
    // ponytail: we need a second Connection handle because AppState holds the
    // primary one behind a Mutex owned by Tauri. The std::sync::Mutex above
    // and tauri::State's Mutex are independent, but SQLite + WAL tolerates
    // multiple connections to the same file. Using `unsafe { *state.db.lock().unwrap() }`
    // would require a static, so we open a fresh connection for the watcher.
    // The wal-mode connection inherits from the open() call.
    std::mem::zeroed()
}));
```

Wait — that won't compile and is unsafe. Replace the entire block with the correct, safe approach:

```rust
// Phase 5 — sync: if any .md under notes/ has mtime newer than notias.db,
// the user's cloud app probably just synced new content. Rebuild before
// the UI renders so search/list are fresh.
if crate::notes::sync::rebuild_if_stale(&state.paths.notes_dir, &state.paths.db_file) {
    tracing::info!("notes/ has changes newer than notias.db; rebuilding index at startup");
    let conn = state.db.lock().expect("db lock");
    let _ = crate::notes::index::rebuild_from_disk(&conn, &state.paths.notes_dir);
    let _ = crate::db::write_meta(
        &state.paths.meta_file,
        &crate::db::migrations::db_hash(&conn).unwrap_or_default(),
        crate::db::migrations::read_schema_version(&conn).unwrap_or(0),
    );
    drop(conn);
}

// Background folder watcher — opens its own WAL-mode connection so it can
// hold the mutex without contending with command handlers on the primary one.
let watch_db_path = state.paths.db_file.clone();
let watch_db = std::thread::spawn(move || {
    let conn = rusqlite::Connection::open(&watch_db_path).ok()?;
    conn.pragma_update(None, "journal_mode", "WAL").ok()?;
    Some(Arc::new(Mutex::new(conn)))
})
.join().ok().flatten();
if let Some(db) = watch_db {
    let _watcher = crate::notes::sync::spawn_watcher(
        state.paths.notes_dir.clone(),
        db,
        std::time::Duration::from_millis(500),
    );
    // ponytail: thread is dropped; it lives until app exit. Errors already logged.
}
```

- [ ] **Step 2: Wire the three new commands into the invoke_handler**

Add to the `tauri::generate_handler![...]` list (alphabetical):

```rust
notes::sync_export_zip,
notes::sync_import_zip,
notes::sync_rebuild_now,
```

(placed before `notes::list_notes` since they share the prefix and Rust sorts them.)

- [ ] **Step 3: Verify compile**

Run: `cd src-tauri && cargo check`
Expected: success.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(sync): spawn notes watcher + startup rebuild_if_stale"
```

---

### Task T62: `SyncSettings.svelte` component

**Files:**
- Create: `src/lib/components/SyncSettings.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import { syncExportZip, syncImportZip, syncRebuildNow } from '$lib/ipc';

  let busy = $state(false);
  let status = $state<string | null>(null);
  let err = $state<string | null>(null);
  let importInput: HTMLInputElement;

  async function exportNow() {
    busy = true; err = null; status = null;
    try {
      const bytes = await syncExportZip();
      const blob = new Blob([new Uint8Array(bytes)], { type: 'application/zip' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `notias-export-${new Date().toISOString().slice(0, 10)}.zip`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      status = `Exported ${bytes.length.toLocaleString()} bytes`;
    } catch (e) {
      err = (e as { message: string }).message;
    } finally { busy = false; }
  }

  async function importNow(ev: Event) {
    const input = ev.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    busy = true; err = null; status = null;
    try {
      const buf = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buf));
      const n = await syncImportZip(bytes);
      status = `Imported ${n} file${n === 1 ? '' : 's'} + rebuilt index`;
    } catch (e) {
      err = (e as { message: string }).message;
    } finally {
      busy = false;
      input.value = '';
    }
  }

  async function rebuild() {
    busy = true; err = null; status = null;
    try {
      const n = await syncRebuildNow();
      status = `Reindexed ${n} note${n === 1 ? '' : 's'}`;
    } catch (e) {
      err = (e as { message: string }).message;
    } finally { busy = false; }
  }
</script>

<section class="sync">
  <h2>Sync</h2>
  <p class="muted">
    Notias does not sync your notes itself. Use your cloud-storage app (Drive,
    Dropbox, iCloud) to sync the <code>notes/</code> folder between machines.
    Use these controls to manually export or import a zip, or rebuild the index.
  </p>

  <div class="row">
    <button onclick={exportNow} disabled={busy}>Export notes.zip</button>
    <button onclick={() => importInput.click()} disabled={busy}>Import zip…</button>
    <input
      type="file"
      accept=".zip,application/zip"
      bind:this={importInput}
      onchange={importNow}
      style="display:none"
    />
    <button onclick={rebuild} disabled={busy}>Rebuild index</button>
  </div>

  {#if status}<p class="status">{status}</p>{/if}
  {#if err}<p class="err">{err}</p>{/if}
</section>

<style>
  .sync { margin: 1.5rem 0; }
  .muted { color: #666; font-size: .9em; margin-bottom: .75rem; }
  code { background: #f4f4f4; padding: 0 .3em; border-radius: 3px; }
  .row { display: flex; gap: .5rem; flex-wrap: wrap; }
  button { padding: .4rem .8rem; border: 1px solid #ccc; border-radius: 4px; background: #fff; cursor: pointer; }
  button:disabled { opacity: .5; cursor: default; }
  .status { color: #178217; margin-top: .5rem; }
  .err { color: #c00; margin-top: .5rem; }
</style>
```

- [ ] **Step 2: Verify it parses**

Run: `pnpm build`
Expected: success (or only Svelte-check warnings unrelated to this file).

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/SyncSettings.svelte
git commit -m "feat(ui): SyncSettings component (export / import / rebuild)"
```

---

### Task T63: ipc.ts bindings + settings route mount

**Files:**
- Modify: `src/lib/ipc.ts`, `src/lib/types.ts`, `src/routes/settings/+page.svelte`

- [ ] **Step 1: Extend `types.ts`**

Append to `src/lib/types.ts`:

```ts
export type SyncStatus = { exported_bytes: number } | { imported_count: number } | { rebuilt: number };
```

- [ ] **Step 2: Extend `ipc.ts`**

Append to `src/lib/ipc.ts`:

```ts
// Phase 5 — sync
export const syncExportZip = () => invoke<number[]>("sync_export_zip");
export const syncImportZip = (bytes: number[]) => invoke<number>("sync_import_zip", { bytes });
export const syncRebuildNow = () => invoke<number>("sync_rebuild_now");
```

- [ ] **Step 3: Mount the component in settings**

In `src/routes/settings/+page.svelte`, add at the end of the `<script>`:

```ts
import SyncSettings from '$lib/components/SyncSettings.svelte';
```

And at the end of the markup, after the existing `<section>`s:

```svelte
<SyncSettings />
```

- [ ] **Step 4: Verify the frontend builds**

Run: `pnpm build`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add src/lib/ipc.ts src/lib/types.ts src/routes/settings/+page.svelte
git commit -m "feat(ui): mount SyncSettings + ipc bindings"
```

---

### Task T64: selfcheck extension + smoke checklist + README + tag

**Files:**
- Modify: `src-tauri/examples/selfcheck.rs`, `docs/test-checklist.md`, `README.md`

- [ ] **Step 1: Add Phase 5 invariants to selfcheck**

Append to `src-tauri/examples/selfcheck.rs`, just before the final `println!`:

```rust
// Phase 5 — sync invariants.
use notias_lib::notes::sync;

// 1. export_zip + import_zip roundtrip on a fresh dir.
let sync_src = tempfile::tempdir().expect("sync src dir");
std::fs::create_dir_all(sync_src.path().join("notes")).unwrap();
let z = sync::export_zip(&sync_src.path().join("notes")).unwrap();
assert!(!z.is_empty(), "export_zip of empty dir must still produce a zip");
let import_dst = tempfile::tempdir().expect("sync dst dir");
let n = sync::import_zip(&import_dst.path().join("notes"), &z).unwrap();
assert_eq!(n, 0, "empty source → zero imports");

// 2. rebuild_if_stale: false on a fresh install (no db file).
let stale = sync::rebuild_if_stale(&sync_src.path().join("notes"), &sync_src.path().join("missing.db"));
assert!(!stale, "no db file → not stale");

println!("phase-5 selfcheck OK: zip roundtrip + stale detection wired");
```

- [ ] **Step 2: Run selfcheck**

Run: `cd src-tauri && cargo run --example selfcheck`
Expected: prints the new line, exits 0.

- [ ] **Step 3: Append Phase 5 smoke checklist**

Append to `docs/test-checklist.md`:

```markdown
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
```

- [ ] **Step 4: Update README status**

Append to the `## Status` bullet list in `README.md`:

```markdown
- Phase 5 (Sync opcional) complete — manual zip export/import, startup rebuild hook for externally-touched files, live folder watcher. Rely on your own cloud-storage app for `notes/` sync; `notias.db` is never shared. Verify with `pnpm tauri dev`; `cargo run --example selfcheck` exercises zip roundtrip + stale detection.
```

- [ ] **Step 5: Commit + tag**

```bash
git add src-tauri/examples/selfcheck.rs docs/test-checklist.md README.md
git commit -m "test(phase-5): selfcheck zip roundtrip + smoke checklist + README status"

git tag phase-5-sync
```

- [ ] **Step 6: Final verification**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests pass (old + new).

Run: `pnpm build`
Expected: success.

---

## Exit Criteria (spec §13 row 5)

> Edit note on machine A; on machine B after the user's cloud app syncs, Notias detects new files and rebuilds index; content matches.

**Covered by:**
- Export → user uploads zip to their cloud app → machine B pulls it → user runs Import → `sync_import_zip` extracts + calls `index::rebuild_from_disk` (T60). Content matches because the zip was the source of truth.
- Alternative no-zip path: machine B syncs the `notes/` folder directly via the cloud app → on startup, `rebuild_if_stale` detects mtime skew → `index::rebuild_from_disk` runs (T61). Live edits while the app is running are caught by the watcher (T59).