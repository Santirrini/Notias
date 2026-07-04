pub mod index;
pub mod model;
pub mod store;
pub mod wikilinks;

pub use model::{Note, NoteSummary, Frontmatter};

use crate::AppState;
use crate::error::{AppError, AppResult};
use crate::notes::{index, model::Note, store, wikilinks};
use tauri::State;
use ulid::Ulid;

#[tauri::command]
pub fn list_notes(tag: Option<String>, state: State<'_, AppState>) -> AppResult<Vec<NoteSummary>> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    index::list(&conn, tag.as_deref())
}

#[tauri::command]
pub fn get_note(id: String, state: State<'_, AppState>) -> AppResult<Note> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let path: String = conn.query_row("SELECT path FROM notes WHERE id = ?1", rusqlite::params![id], |r| r.get(0))
        .map_err(|_| AppError::NotFound(id.clone()))?;
    store::read(std::path::Path::new(&path))
}

#[tauri::command]
pub fn create_note(title: String, state: State<'_, AppState>) -> AppResult<Note> {
    let id = Ulid::new().to_string();
    let now = chrono_now();
    let note = Note {
        id: id.clone(),
        path: state.paths.notes_dir.join(format!("{id}.md")),
        title: title.clone(),
        body: String::new(),
        frontmatter: model_frontmatter(&id, &title, &now),
    };
    store::write(&note)?;
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    index::upsert(&conn, &note)?;
    Ok(note)
}

#[tauri::command]
pub fn update_note(id: String, title: Option<String>, body: Option<String>, state: State<'_, AppState>) -> AppResult<Note> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let path: String = conn.query_row("SELECT path FROM notes WHERE id = ?1", rusqlite::params![id], |r| r.get(0))
        .map_err(|_| AppError::NotFound(id.clone()))?;
    drop(conn);
    let mut note = store::read(std::path::Path::new(&path))?;
    if let Some(t) = title { note.frontmatter.title = t.clone(); note.title = t; }
    if let Some(b) = body { note.body = b; }
    note.frontmatter.updated = chrono_now();
    note.frontmatter.links = wikilinks::extract_links(&note.body);
    note.frontmatter.references = wikilinks::extract_attachments(&note.body);
    store::write(&note)?;
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    index::upsert(&conn, &note)?;
    Ok(note)
}

#[tauri::command]
pub fn delete_note(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    index::soft_delete(&conn, &id)?;
    // ponytail: we also remove the .md file so rebuild_from_disk doesn't re-insert it.
    // Until Phase 5 introduces sync, the file is the local source of truth and must
    // match the index.
    if let Ok(path) = conn.query_row::<String, _, _>("SELECT path FROM notes WHERE id = ?1", rusqlite::params![&id], |r| r.get(0)) {
        let _ = std::fs::remove_file(std::path::Path::new(&path));
    }
    Ok(())
}

#[tauri::command]
pub fn search_notes(q: String, state: State<'_, AppState>) -> AppResult<Vec<String>> {
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    index::search(&conn, &q)
}

#[tauri::command]
pub fn rebuild_index(state: State<'_, AppState>) -> AppResult<usize> {
    // ponytail: holds the DB mutex for the entire disk walk + per-file upsert. With <1k notes
    // the lock is held for ~10-100ms; UI stays responsive enough for MVP. When Phase 5 sync
    // pushes corpus sizes into the thousands, move this to a background task and release the
    // mutex between upserts (or use a separate index-rebuild connection).
    let conn = state.db.lock().map_err(|_| AppError::Config("db lock".into()))?;
    let n = index::rebuild_from_disk(&conn, &state.paths.notes_dir)?;
    let _ = crate::db::write_meta(
        &state.paths.meta_file,
        &crate::db::migrations::db_hash(&conn)?,
        crate::db::migrations::read_schema_version(&conn)?,
    );
    Ok(n)
}

fn chrono_now() -> String {
    // ponytail: uses `time` crate (already in Cargo.toml from Phase 0). ISO-8601 UTC,
    // second precision. Replace with chrono if timezone-aware logic ever lands.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_unix_seconds_as_rfc3339(secs)
}

fn format_unix_seconds_as_rfc3339(secs: u64) -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    OffsetDateTime::from_unix_timestamp(secs as i64)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn model_frontmatter(id: &str, title: &str, now: &str) -> crate::notes::model::Frontmatter {
    use crate::notes::model::Frontmatter;
    Frontmatter { id: id.into(), title: title.into(), tags: vec![], created: now.into(), updated: now.into(), links: vec![], references: vec![] }
}
