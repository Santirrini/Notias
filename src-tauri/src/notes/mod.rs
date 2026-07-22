pub mod attachments;
pub mod index;
pub mod model;
pub mod store;
pub mod sync;
pub mod wikilinks;

pub use model::{Note, NoteSummary, Frontmatter};

use crate::AppState;
use crate::error::{AppError, AppResult};
use tauri::State;
use ulid::Ulid;

#[tauri::command]
pub fn list_notes(tag: Option<String>, state: State<'_, AppState>) -> AppResult<Vec<NoteSummary>> {
    let conn = state.db_conn()?;
    index::list(&conn, tag.as_deref())
}

#[tauri::command]
pub fn get_note(id: String, state: State<'_, AppState>) -> AppResult<Note> {
    let conn = state.db_conn()?;
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
    let conn = state.db_conn()?;
    index::upsert(&conn, &note)?;
    Ok(note)
}

#[tauri::command]
pub fn update_note(
    id: String,
    title: Option<String>,
    body: Option<String>,
    paper: Option<String>,
    paper_tint: Option<String>,
    editor_font: Option<String>,
    editor_font_size: Option<String>,
    editor_line_height: Option<String>,
    editor_page_width: Option<String>,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Note> {
    let conn = state.db_conn()?;
    let path: String = conn.query_row("SELECT path FROM notes WHERE id = ?1", rusqlite::params![id], |r| r.get(0))
        .map_err(|_| AppError::NotFound(id.clone()))?;
    drop(conn);
    let mut note = store::read(std::path::Path::new(&path))?;
    if let Some(t) = title { note.frontmatter.title = t.clone(); note.title = t; }
    if let Some(b) = body { note.body = b; }
    // Paper / typography overrides. `Some("")` clears the override (back to
    // global default); `None` means "don't touch this field" — useful when the
    // UI patches a single key without sending the rest.
    apply_override(&mut note.frontmatter.paper, paper);
    apply_override(&mut note.frontmatter.paper_tint, paper_tint);
    apply_override(&mut note.frontmatter.editor_font, editor_font);
    apply_override(&mut note.frontmatter.editor_font_size, editor_font_size);
    apply_override(&mut note.frontmatter.editor_line_height, editor_line_height);
    apply_override(&mut note.frontmatter.editor_page_width, editor_page_width);
    note.frontmatter.updated = chrono_now();
    note.frontmatter.links = wikilinks::extract_links(&note.body);
    note.frontmatter.references = wikilinks::extract_attachments(&note.body);
    store::write(&note)?;
    let conn = state.db_conn()?;
    index::upsert(&conn, &note)?;
    drop(conn);

    let app2 = app.clone();
    let note_id = note.id.clone();
    let note_id_for_job = note_id.clone();
    {
        let mut jobs = state.embed_jobs.lock().map_err(|_| AppError::Config("jobs lock".into()))?;
        if let Some(prev) = jobs.remove(&note_id) { prev.abort(); }
        let handle = tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            if let Err(e) = crate::ai::embed::run(note_id_for_job.clone(), app2.clone()).await {
                tracing::warn!("embed worker failed for {note_id_for_job}: {e}");
            }
        });
        jobs.insert(note_id, handle);
    }

    Ok(note)
}

#[tauri::command]
pub fn delete_note(id: String, state: State<'_, AppState>) -> AppResult<()> {
    let conn = state.db_conn()?;
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
    let conn = state.db_conn()?;
    index::search(&conn, &q)
}

#[tauri::command]
pub fn rebuild_index(state: State<'_, AppState>) -> AppResult<usize> {
    // ponytail: holds the DB mutex for the entire disk walk + per-file upsert. With <1k notes
    // the lock is held for ~10-100ms; UI stays responsive enough for MVP. When Phase 5 sync
    // pushes corpus sizes into the thousands, move this to a background task and release the
    // mutex between upserts (or use a separate index-rebuild connection).
    let conn = state.db_conn()?;
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
    Frontmatter {
        id: id.into(),
        title: title.into(),
        created: now.into(),
        updated: now.into(),
        ..Default::default()
    }
}

/// Update a single optional frontmatter field from a Tauri patch.
/// - `None`  → leave the field as-is (UI didn't send this key).
/// - `Some("")` → clear the override (back to global default).
/// - `Some(v)` → set the override to `v`.
fn apply_override(field: &mut Option<String>, patch: Option<String>) {
    if let Some(v) = patch {
        *field = if v.is_empty() { None } else { Some(v) };
    }
}

#[tauri::command]
pub fn sync_export_zip(state: State<'_, AppState>) -> AppResult<Vec<u8>> {
    sync::export_zip(&state.paths.notes_dir)
}

#[tauri::command]
pub fn sync_import_zip(bytes: Vec<u8>, state: State<'_, AppState>) -> AppResult<usize> {
    let n = sync::import_zip(&state.paths.notes_dir, &bytes)?;
    let conn = state.db_conn()?;
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
    let conn = state.db_conn()?;
    let n = index::rebuild_from_disk(&conn, &state.paths.notes_dir)?;
    let _ = crate::db::write_meta(
        &state.paths.meta_file,
        &crate::db::migrations::db_hash(&conn)?,
        crate::db::migrations::read_schema_version(&conn)?,
    );
    Ok(n)
}

// ---------------------------------------------------------------------------
// Drawing attachments — Excalidraw SVG + editable .excalidraw JSON blobs
// persisted under notes_dir/attachments/{note_id}/. The body of the note
// embeds the SVG via `![](attachments/{note_id}/{drawing_id}.svg)`. The
// existing `wikilinks::extract_attachments` scanner picks up the link and
// tracks it in frontmatter.references automatically.
// ---------------------------------------------------------------------------

/// Write a drawing (SVG for display + .excalidraw JSON for re-editing).
/// Returns the SVG's relative path under notes_dir — embed it as the image
/// src in the markdown body.
#[tauri::command]
pub fn save_drawing(
    note_id: String,
    drawing_id: String,
    svg: Vec<u8>,
    state: Vec<u8>,
    app_state: State<'_, AppState>,
) -> AppResult<String> {
    attachments::save(&app_state.paths.notes_dir, &note_id, &drawing_id, &svg, &state)
}

#[tauri::command]
pub fn read_drawing(path: String, app_state: State<'_, AppState>) -> AppResult<Vec<u8>> {
    attachments::read_svg(&app_state.paths.notes_dir, &path)
}

#[tauri::command]
pub fn read_drawing_state(path: String, app_state: State<'_, AppState>) -> AppResult<Vec<u8>> {
    attachments::read_state(&app_state.paths.notes_dir, &path)
}

#[tauri::command]
pub fn delete_drawing(path: String, app_state: State<'_, AppState>) -> AppResult<()> {
    attachments::delete(&app_state.paths.notes_dir, &path)
}
