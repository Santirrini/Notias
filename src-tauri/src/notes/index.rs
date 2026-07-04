use crate::error::{AppError, AppResult};
use crate::notes::model::{Note, NoteSummary};
use rusqlite::{params, Connection};

pub fn upsert(conn: &Connection, note: &Note) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO notes (id, path, title, created, updated) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET title=excluded.title, updated=excluded.updated, deleted_at=NULL",
        params![note.id, note.path.to_string_lossy(), note.frontmatter.title,
                note.frontmatter.created, note.frontmatter.updated],
    )?;
    tx.execute("DELETE FROM notes_fts WHERE rowid IN (SELECT rowid FROM notes WHERE id = ?1)", params![note.id])?;
    tx.execute("INSERT INTO notes_fts (rowid, title, body) VALUES ((SELECT rowid FROM notes WHERE id = ?1), ?2, ?3)",
               // ponytail: notes.id is TEXT (ULID), not aliased to rowid; we map via the implicit rowid.
               params![note.id, note.frontmatter.title, note.body])?;
    tx.execute("DELETE FROM note_tags WHERE note_id = ?1", params![note.id])?;
    for tag in &note.frontmatter.tags {
        tx.execute("INSERT INTO note_tags (note_id, tag) VALUES (?1, ?2)", params![note.id, tag])?;
    }
    tx.commit()?;
    Ok(())
}

pub fn soft_delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("UPDATE notes SET deleted_at = datetime('now') WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn list(conn: &Connection, tag: Option<&str>) -> AppResult<Vec<NoteSummary>> {
    // ponytail: title-filter is done client-side in NoteTree (cheap; in-memory). The DB returns
    // the most-recently-updated 200 live notes; if a tag is provided we filter to notes tagged
    // with it. Tag filtering for chat RAG adds a `tags` field via LEFT JOIN + GROUP_CONCAT below.
    let mut sql = String::from(
        "SELECT n.id, n.title, n.updated, COALESCE(GROUP_CONCAT(t.tag, ','), '') \
         FROM notes n LEFT JOIN note_tags t ON t.note_id = n.id WHERE n.deleted_at IS NULL"
    );
    let mut binds: Vec<String> = vec![];
    if let Some(tag_val) = tag {
        sql.push_str(" AND EXISTS (SELECT 1 FROM note_tags t2 WHERE t2.note_id = n.id AND t2.tag = ?)");
        binds.push(tag_val.into());
    }
    sql.push_str(" GROUP BY n.id ORDER BY n.updated DESC LIMIT 200");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(binds.iter()), |r| {
        let tags_csv: String = r.get(3)?;
        let tags: Vec<String> = if tags_csv.is_empty() {
            vec![]
        } else {
            tags_csv.split(',').map(|s| s.to_string()).collect()
        };
        Ok(NoteSummary {
            id: r.get(0)?,
            title: r.get(1)?,
            updated: r.get(2)?,
            tags,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

pub fn search(conn: &Connection, q: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT n.id FROM notes n JOIN notes_fts fts ON fts.rowid = n.rowid
         WHERE notes_fts MATCH ?1 AND n.deleted_at IS NULL ORDER BY rank LIMIT 50",
    )?;
    let ids: Vec<String> = stmt.query_map(params![q], |r| r.get(0))?.filter_map(Result::ok).collect();
    Ok(ids)
}

pub fn rebuild_from_disk(conn: &Connection, root: &std::path::Path) -> AppResult<usize> {
    // ponytail: rebuild_from_disk does NOT respect deleted_at; a soft-deleted note whose .md
    // file is still on disk will be re-inserted as live. Acceptable MVP behavior since
    // delete_note() in Phase 1 removes the .md file along with soft-deleting the index row.
    // When Phase 5 introduces sync, this becomes a known ceiling to revisit.
    let mut count = 0;
    for entry in walk(root)? {
        if entry.extension().and_then(|s| s.to_str()) != Some("md") { continue; }
        match crate::notes::store::read(&entry) {
            Ok(n) => { upsert(conn, &n)?; count += 1; }
            Err(e) => tracing::warn!("skip {}: {e}", entry.display()),
        }
    }
    Ok(count)
}

fn walk(root: &std::path::Path) -> AppResult<Vec<std::path::PathBuf>> {
    let mut out = vec![];
    if !root.exists() { return Ok(out); }
    for e in std::fs::read_dir(root)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() { out.extend(walk(&p)?); } else { out.push(p); }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::model::{Frontmatter, Note};
    use tempfile::tempdir;

    fn make_note(dir: &std::path::Path, id: &str, title: &str, body: &str) -> Note {
        Note {
            id: id.into(),
            path: dir.join(format!("{id}.md")),
            title: title.into(),
            body: body.into(),
            frontmatter: Frontmatter {
                id: id.into(),
                title: title.into(),
                tags: vec!["x".into()],
                created: "2026-07-03T00:00:00Z".into(),
                updated: "2026-07-03T00:00:00Z".into(),
                links: vec![],
                references: vec![],
            },
        }
    }

    #[test]
    fn upsert_and_search() {
        let dir = tempdir().unwrap();
        let conn = Connection::open_in_memory().unwrap();
        crate::db::migrations::run(&conn).unwrap();
        let n = make_note(dir.path(), "a", "Rust borrow checker", "fn main(){}");
        upsert(&conn, &n).unwrap();
        let hits = search(&conn, "borrow").unwrap();
        assert_eq!(hits, vec!["a".to_string()]);
    }
}