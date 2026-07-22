use crate::error::{AppError, AppResult};
use crate::notes::model::{Frontmatter, Note};
use std::path::{Path, PathBuf};

pub fn write(note: &Note) -> AppResult<()> {
    if let Some(parent) = note.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let fm_yaml = frontmatter_to_yaml(&note.frontmatter);
    let serialized = format!("{fm_yaml}---\n{}", note.body);
    let tmp = tmp_path(&note.path);
    std::fs::write(&tmp, serialized)?;
    std::fs::rename(&tmp, &note.path)?;
    Ok(())
}

/// Render frontmatter as the YAML block used by the on-disk format.
///
/// Notes:
/// - The base 5 fields (`id, title, tags, created, updated`) are emitted in
///   the same order/format as before so existing notes round-trip unchanged.
/// - The optional paper/typography overrides are appended after `updated` so
///   that notes that don't use them stay byte-identical on disk.
/// - Empty strings clear an override (mirror of frontend semantics: "" = "use
///   global default"). `None` is also valid; both render to the key being
///   absent, which the frontmatter parser treats as the inherited default.
fn frontmatter_to_yaml(fm: &Frontmatter) -> String {
    let mut s = String::from("---\n");
    s.push_str(&format!("id: {}\n", fm.id));
    s.push_str(&format!("title: {}\n", fm.title));
    s.push_str(&format!("tags: [{}]\n", fm.tags.join(",")));
    s.push_str(&format!("created: {}\n", fm.created));
    s.push_str(&format!("updated: {}\n", fm.updated));
    if let Some(v) = fm.paper.as_deref() {
        s.push_str(&format!("paper: {}\n", yaml_scalar(v)));
    }
    if let Some(v) = fm.paper_tint.as_deref() {
        s.push_str(&format!("paperTint: {}\n", yaml_scalar(v)));
    }
    if let Some(v) = fm.editor_font.as_deref() {
        s.push_str(&format!("editorFont: {}\n", yaml_scalar(v)));
    }
    if let Some(v) = fm.editor_font_size.as_deref() {
        s.push_str(&format!("editorFontSize: {}\n", yaml_scalar(v)));
    }
    if let Some(v) = fm.editor_line_height.as_deref() {
        s.push_str(&format!("editorLineHeight: {}\n", yaml_scalar(v)));
    }
    if let Some(v) = fm.editor_page_width.as_deref() {
        s.push_str(&format!("editorPageWidth: {}\n", yaml_scalar(v)));
    }
    s
}

/// Quote a value as a YAML scalar if it contains characters that would confuse
/// `serde_yaml`. Our tokens (`ruled`, `serif`, `md`, `normal`, ...) are all
/// safe bare strings, but if a user ever hand-edits a note and types something
/// exotic we'd rather not corrupt the file.
fn yaml_scalar(v: &str) -> String {
    if v.is_empty() {
        return "\"\"".into();
    }
    let needs_quote = v.contains([':', '#', '\n', '\t'])
        || v.starts_with([' ', '-', '?', '&', '*', '!', '|', '>', '%', '@', '`', '\'', '"'])
        || v.ends_with(' ');
    if needs_quote {
        let escaped = v.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        v.to_string()
    }
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
                created: "2026-07-03T00:00:00Z".into(),
                updated: "2026-07-03T00:00:00Z".into(),
                ..Default::default()
            },
        };
        write(&n).unwrap();
        let r = read(&path).unwrap();
        assert_eq!(r.body, "world");
        assert_eq!(r.frontmatter.title, "Hello");
    }

    /// Regression test for the paper/typography round-trip hazard: until this
    /// PR, `store::write()` used a fixed `format!` template that dropped any
    /// new frontmatter fields. This test pins the contract that any `Some(_)`
    /// value on `paper`/`paperTint`/`editorFont`/`editorFontSize`/
    /// `editorLineHeight`/`editorPageWidth` survives `write → read`.
    #[test]
    fn paper_overrides_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("p.md");
        let mut fm = Frontmatter {
            id: "p".into(),
            title: "T".into(),
            created: "2026-07-21T00:00:00Z".into(),
            updated: "2026-07-21T00:00:00Z".into(),
            ..Default::default()
        };
        fm.paper = Some("grid".into());
        fm.paper_tint = Some("sepia".into());
        fm.editor_font = Some("serif".into());
        fm.editor_font_size = Some("lg".into());
        fm.editor_line_height = Some("relaxed".into());
        fm.editor_page_width = Some("wide".into());
        let n = Note {
            id: "p".into(),
            path: path.clone(),
            title: "T".into(),
            body: "body".into(),
            frontmatter: fm.clone(),
        };
        write(&n).unwrap();
        let r = read(&path).unwrap();
        assert_eq!(r.frontmatter.paper.as_deref(), Some("grid"));
        assert_eq!(r.frontmatter.paper_tint.as_deref(), Some("sepia"));
        assert_eq!(r.frontmatter.editor_font.as_deref(), Some("serif"));
        assert_eq!(r.frontmatter.editor_font_size.as_deref(), Some("lg"));
        assert_eq!(r.frontmatter.editor_line_height.as_deref(), Some("relaxed"));
        assert_eq!(r.frontmatter.editor_page_width.as_deref(), Some("wide"));
    }

    /// Notes that don't use paper overrides must keep the same on-disk format
    /// as before — the only lines emitted are the original 5 fields. This
    /// protects existing notes from byte-level diffs on first save.
    #[test]
    fn legacy_notes_keep_old_format() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.md");
        let n = Note {
            id: "l".into(),
            path: path.clone(),
            title: "Legacy".into(),
            body: "old".into(),
            frontmatter: Frontmatter {
                id: "l".into(),
                title: "Legacy".into(),
                created: "2026-07-21T00:00:00Z".into(),
                updated: "2026-07-21T00:00:00Z".into(),
                ..Default::default()
            },
        };
        write(&n).unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        // No paper / typography keys present.
        assert!(!raw.contains("paper:"));
        assert!(!raw.contains("paperTint:"));
        assert!(!raw.contains("editorFont:"));
        assert!(!raw.contains("editorFontSize:"));
        assert!(!raw.contains("editorLineHeight:"));
        assert!(!raw.contains("editorPageWidth:"));
        // The legacy 5 fields are still present in the same order.
        assert!(raw.starts_with("---\nid: l\ntitle: Legacy\ntags: []\ncreated: "));
    }

    /// Old `.md` files (pre-PR) must still parse: missing paper keys = None.
    #[test]
    fn parse_tolerates_missing_paper_keys() {
        let raw = "---\nid: old\ntitle: Old\ntags: []\ncreated: c\nupdated: u\n---\nbody\n";
        let (fm, _) = crate::notes::model::parse(raw).unwrap();
        assert!(fm.paper.is_none());
        assert!(fm.paper_tint.is_none());
        assert!(fm.editor_font.is_none());
        assert!(fm.editor_font_size.is_none());
        assert!(fm.editor_line_height.is_none());
        assert!(fm.editor_page_width.is_none());
    }
}
