// Phase 5 — sync: zip export/import + rebuild_if_stale + notify watcher.
// Spec §13 row 5: rely on the user's cloud-storage app; we only do manual
// export/import + a startup rebuild hook + live watch for external edits.

use crate::error::{AppError, AppResult};
use crate::notes::{index, store};
use rusqlite::Connection;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Read every `.md` under `notes_dir` into an in-memory zip. Returns the bytes.
/// Files are stored with paths relative to `notes_dir` (current layout is flat,
/// so entries are basenames like `abc.md`).
pub fn export_zip(notes_dir: &Path) -> AppResult<Vec<u8>> {
    if !notes_dir.exists() {
        return Err(AppError::NotFound(format!("notes dir {:?}", notes_dir)));
    }
    let mut buf = Vec::new();
    {
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let opts: zip::write::FileOptions = zip::write::FileOptions::default()
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
        if entry.is_dir() { continue; }
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

/// Open a fresh WAL-mode connection for the watcher thread. SQLite tolerates
/// multiple connections to the same file under WAL; the watcher doesn't need
/// to share the primary handle (which Tauri owns behind a Mutex).
pub fn open_watcher_connection(db_file: &Path) -> AppResult<Connection> {
    let conn = rusqlite::Connection::open(db_file)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    Ok(conn)
}

/// Background watcher: std thread, debounces per-path events for `debounce`,
/// then re-indexes each changed `.md` via `index::upsert`. Holds `db` mutex
/// briefly per upsert. Remove events are logged but ignored — a moved file
/// should not silently soft-delete the index row.
pub fn spawn_watcher(
    notes_dir: PathBuf,
    db: Arc<Mutex<Connection>>,
    debounce: Duration,
) -> std::thread::JoinHandle<()> {
    use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;

    std::thread::spawn(move || {
        // ponytail: notify 6 + manual 500ms per-path debounce. notify-debouncer-mini
        // would do this in one crate but adds a dep for ~20 lines.
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
        let mut last_seen: std::collections::HashMap<PathBuf, SystemTime> =
            std::collections::HashMap::new();
        while let Ok(res) = rx.recv() {
            let event = match res {
                Ok(e) => e,
                Err(e) => { tracing::warn!("watch error: {e}"); continue; }
            };
            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                continue;
            }
            for path in event.paths {
                if path.extension().and_then(|s| s.to_str()) != Some("md") { continue; }
                let now = SystemTime::now();
                if let Some(prev) = last_seen.get(&path) {
                    if now.duration_since(*prev).unwrap_or(Duration::ZERO) < debounce {
                        continue;
                    }
                }
                last_seen.insert(path.clone(), now);

                if matches!(event.kind, EventKind::Remove(_)) {
                    tracing::info!("watcher: ignored remove for {}", path.display());
                    continue;
                }

                let conn = match db.lock() {
                    Ok(c) => c,
                    Err(_) => { tracing::warn!("db lock poisoned"); continue; }
                };
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
    use crate::notes::model::{Frontmatter, Note};
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
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut zip = zip::ZipWriter::new(cursor);
            zip.start_file("../escape.md", zip::write::FileOptions::default()).unwrap();
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

        let past = SystemTime::now() - Duration::from_secs(10);
        let f = std::fs::OpenOptions::new().write(true).open(&db).unwrap();
        f.set_modified(past).unwrap();
        drop(f);

        std::fs::write(
            notes.join("new.md"),
            b"---\nid: n\ntitle: n\ncreated: c\nupdated: u\n---\nbody",
        ).unwrap();
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
}