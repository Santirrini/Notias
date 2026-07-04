use crate::error::{AppError, AppResult};
use crate::notes::model::Note;
use std::path::{Path, PathBuf};

pub fn write(note: &Note) -> AppResult<()> {
    if let Some(parent) = note.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let serialized = format!(
        "---\nid: {}\ntitle: {}\ntags: [{}]\ncreated: {}\nupdated: {}\n---\n{}",
        note.frontmatter.id,
        note.frontmatter.title,
        note.frontmatter.tags.join(","),
        note.frontmatter.created,
        note.frontmatter.updated,
        note.body,
    );
    let tmp = tmp_path(&note.path);
    std::fs::write(&tmp, serialized)?;
    std::fs::rename(&tmp, &note.path)?;
    Ok(())
}

pub fn read(path: &Path) -> AppResult<Note> {
    let raw = std::fs::read_to_string(path)?;
    let (fm, body) = crate::notes::model::parse(&raw)
        .map_err(|e| AppError::Invalid(format!("yaml: {e}")))?;
    Ok(Note {
        id: fm.id.clone(),
        path: path.to_path_buf(),
        title: fm.title.clone(),
        body,
        frontmatter: fm,
    })
}

fn tmp_path(p: &Path) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::model::{Frontmatter, Note};
    use tempfile::tempdir;

    #[test]
    fn atomic_write_then_read() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("abc.md");
        let n = Note {
            id: "abc".into(),
            path: path.clone(),
            title: "Hello".into(),
            body: "world".into(),
            frontmatter: Frontmatter {
                id: "abc".into(),
                title: "Hello".into(),
                tags: vec![],
                created: "2026-07-03T00:00:00Z".into(),
                updated: "2026-07-03T00:00:00Z".into(),
                links: vec![],
                references: vec![],
            },
        };
        write(&n).unwrap();
        let r = read(&path).unwrap();
        assert_eq!(r.body, "world");
        assert_eq!(r.frontmatter.title, "Hello");
    }
}
