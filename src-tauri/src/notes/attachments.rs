// Drawing attachments — Excalidraw SVGs + editable `.excalidraw` JSON blobs
// stored alongside the note that owns them. Source of truth on disk is
// `notes_dir/attachments/{note_id}/{drawing_id}.{svg,excalidraw}`. The body
// of the .md file embeds the SVG via `![](attachments/{note_id}/{drawing_id}.svg)`
// which the existing `wikilinks::extract_attachments` scanner already picks up
// and stores in `frontmatter.references` — so we get attachment tracking for free.
//
// All paths exchanged with the frontend are RELATIVE to `notes_dir` (forward
// slashes), never absolute. This keeps notes portable between machines and
// matches the convention already used for note paths in the SQLite index.

use crate::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

/// Subdirectory of `notes_dir` where attachments live. Hardcoded because it
/// also appears (string-literal-prefixed) in `wikilinks::extract_attachments`.
pub const ATTACHMENTS_DIR: &str = "attachments";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Svg,
    Excalidraw,
}

impl Kind {
    pub fn extension(self) -> &'static str {
        match self {
            Kind::Svg => "svg",
            Kind::Excalidraw => "excalidraw",
        }
    }
}

/// Write both the SVG (used as inline image) and the `.excalidraw` JSON
/// (used to restore the editable scene). Returns the SVG's path RELATIVE to
/// `notes_dir` — that's the value the markdown image embeds.
pub fn save(
    notes_dir: &Path,
    note_id: &str,
    drawing_id: &str,
    svg: &[u8],
    state: &[u8],
) -> AppResult<String> {
    let note_dir = ensure_note_dir(notes_dir, note_id)?;
    write_kind(&note_dir, drawing_id, Kind::Svg, svg)?;
    write_kind(&note_dir, drawing_id, Kind::Excalidraw, state)?;
    Ok(rel_path(note_id, drawing_id, Kind::Svg))
}

/// Read the SVG bytes for the relative path returned by `save`.
pub fn read_svg(notes_dir: &Path, rel: &str) -> AppResult<Vec<u8>> {
    resolve_rel(notes_dir, rel, Kind::Svg).and_then(|p| std::fs::read(&p).map_err(AppError::from))
}

/// Read the `.excalidraw` JSON bytes for the relative path returned by `save`.
pub fn read_state(notes_dir: &Path, rel: &str) -> AppResult<Vec<u8>> {
    resolve_rel(notes_dir, rel, Kind::Excalidraw).and_then(|p| std::fs::read(&p).map_err(AppError::from))
}

/// Delete both files for the relative SVG path. Tolerates one missing (in case
/// a previous crash left only one); rejects paths outside `notes_dir`.
pub fn delete(notes_dir: &Path, rel: &str) -> AppResult<()> {
    let (note_id, drawing_id) = parse_rel(rel)?;
    let dir = notes_dir.join(ATTACHMENTS_DIR).join(&note_id);
    let canonical = dir.canonicalize().map_err(|e| AppError::Invalid(format!("canonicalize {dir:?}: {e}")))?;
    let allowed = notes_dir.join(ATTACHMENTS_DIR).canonicalize().map_err(|e| AppError::Invalid(format!("canonicalize attachments root: {e}")))?;
    if !canonical.starts_with(&allowed) {
        return Err(AppError::Invalid(format!("attachment path escapes notes_dir: {rel}")));
    }
    for kind in [Kind::Svg, Kind::Excalidraw] {
        let p = dir.join(format!("{drawing_id}.{}", kind.extension()));
        match std::fs::remove_file(&p) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(AppError::Io(e)),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn ensure_note_dir(notes_dir: &Path, note_id: &str) -> AppResult<PathBuf> {
    validate_id(note_id)?;
    let dir = notes_dir.join(ATTACHMENTS_DIR).join(note_id);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn write_kind(note_dir: &Path, drawing_id: &str, kind: Kind, bytes: &[u8]) -> AppResult<()> {
    validate_id(drawing_id)?;
    let tmp = note_dir.join(format!("{drawing_id}.{}.tmp", kind.extension()));
    let final_path = note_dir.join(format!("{drawing_id}.{}", kind.extension()));
    std::fs::write(&tmp, bytes)?;
    // Atomic-ish rename. If `final_path` already exists, this overwrites it on
    // every platform we support (rename semantics for existing dest: Windows =
    // ERROR_ALREADY_EXISTS since Rust 1.5 we use std::fs::rename which on
    // Windows returns an error if dest exists — so remove first).
    if final_path.exists() {
        std::fs::remove_file(&final_path)?;
    }
    std::fs::rename(&tmp, &final_path)?;
    Ok(())
}

/// Validate that an id looks like a ULID (26 chars, alphanumeric) — protects
/// against path injection. Not strict; just rejects `..`, `/`, and empty.
fn validate_id(id: &str) -> AppResult<()> {
    if id.is_empty() || id.len() > 64 {
        return Err(AppError::Invalid(format!("bad id length: {id:?}")));
    }
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err(AppError::Invalid(format!("bad id chars: {id:?}")));
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::Invalid(format!("bad id charset: {id:?}")));
    }
    Ok(())
}

fn rel_path(note_id: &str, drawing_id: &str, kind: Kind) -> String {
    format!("{ATTACHMENTS_DIR}/{note_id}/{drawing_id}.{}", kind.extension())
}

/// Parse `attachments/{note_id}/{drawing_id}.{ext}` → `(note_id, drawing_id)`.
/// Validates both IDs.
fn parse_rel(rel: &str) -> AppResult<(String, String)> {
    let trimmed = rel.trim_start_matches("./");
    let rest = trimmed
        .strip_prefix(ATTACHMENTS_DIR)
        .ok_or_else(|| AppError::Invalid(format!("rel not under attachments/: {rel}")))?
        .trim_start_matches('/');
    let mut parts = rest.split('/');
    let note_id = parts
        .next()
        .ok_or_else(|| AppError::Invalid(format!("rel missing note_id: {rel}")))?
        .to_string();
    let drawing_with_ext = parts
        .next()
        .ok_or_else(|| AppError::Invalid(format!("rel missing drawing_id: {rel}")))?;
    if parts.next().is_some() {
        return Err(AppError::Invalid(format!("rel has extra segments: {rel}")));
    }
    let drawing_id = drawing_with_ext
        .split('.')
        .next()
        .ok_or_else(|| AppError::Invalid(format!("rel missing drawing_id: {rel}")))?
        .to_string();
    validate_id(&note_id)?;
    validate_id(&drawing_id)?;
    if drawing_id.is_empty() {
        return Err(AppError::Invalid(format!("rel has empty drawing_id: {rel}")));
    }
    Ok((note_id, drawing_id))
}

fn resolve_rel(notes_dir: &Path, rel: &str, kind: Kind) -> AppResult<PathBuf> {
    let (note_id, drawing_id) = parse_rel(rel)?;
    let p = notes_dir
        .join(ATTACHMENTS_DIR)
        .join(&note_id)
        .join(format!("{drawing_id}.{}", kind.extension()));
    // Canonicalize check is best-effort — the file may not exist yet for first
    // save. We verify the PARENT is sane and the assembled path doesn't escape.
    let canonical_parent = p
        .parent()
        .ok_or_else(|| AppError::Invalid(format!("rel has no parent: {rel}")))?
        .canonicalize()
        .map_err(|e| AppError::Invalid(format!("canonicalize parent: {e}")))?;
    let allowed = notes_dir
        .join(ATTACHMENTS_DIR)
        .canonicalize()
        .map_err(|e| AppError::Invalid(format!("canonicalize attachments root: {e}")))?;
    if !canonical_parent.starts_with(&allowed) {
        return Err(AppError::Invalid(format!("attachment path escapes notes_dir: {rel}")));
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir_path() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "notias-attach-test-{}-{}",
            std::process::id(),
            ulid::Ulid::new()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn save_then_read_roundtrip() {
        let notes_dir = tempdir_path();
        let svg = b"<svg><rect/></svg>";
        let state = br#"{"elements":[]}"#;
        let rel = save(&notes_dir, "01J0TEST", "draw1", svg, state).unwrap();
        assert_eq!(rel, "attachments/01J0TEST/draw1.svg");
        assert_eq!(read_svg(&notes_dir, &rel).unwrap(), svg);
        assert_eq!(
            read_state(&notes_dir, &rel).unwrap(),
            state
        );
        std::fs::remove_dir_all(notes_dir).ok();
    }

    #[test]
    fn save_overwrites_existing() {
        let notes_dir = tempdir_path();
        let r1 = save(&notes_dir, "N1", "d1", b"<svg/>", b"{}").unwrap();
        let r2 = save(&notes_dir, "N1", "d1", b"<svg2/>", b"{2}").unwrap();
        assert_eq!(r1, r2);
        assert_eq!(read_svg(&notes_dir, &r2).unwrap(), b"<svg2/>");
        std::fs::remove_dir_all(notes_dir).ok();
    }

    #[test]
    fn delete_removes_both_files() {
        let notes_dir = tempdir_path();
        let rel = save(&notes_dir, "NX", "dx", b"s", b"j").unwrap();
        delete(&notes_dir, &rel).unwrap();
        assert!(read_svg(&notes_dir, &rel).is_err());
        assert!(read_state(&notes_dir, &rel).is_err());
        std::fs::remove_dir_all(notes_dir).ok();
    }

    #[test]
    fn delete_tolerates_missing() {
        let notes_dir = tempdir_path();
        save(&notes_dir, "NX", "dx", b"s", b"j").unwrap();
        std::fs::remove_file(notes_dir.join("attachments/NX/dx.svg")).unwrap();
        delete(&notes_dir, "attachments/NX/dx.svg").unwrap();
        std::fs::remove_dir_all(notes_dir).ok();
    }

    #[test]
    fn path_traversal_rejected_in_save() {
        let notes_dir = tempdir_path();
        for bad in ["../etc/passwd", "foo/bar", "", "a/b"] {
            assert!(save(&notes_dir, bad, "d1", b"s", b"j").is_err(), "{bad} should fail");
            assert!(save(&notes_dir, "OK", bad, b"s", b"j").is_err(), "{bad} should fail");
        }
        std::fs::remove_dir_all(notes_dir).ok();
    }

    #[test]
    fn path_traversal_rejected_in_read() {
        let notes_dir = tempdir_path();
        std::fs::create_dir_all(notes_dir.join("attachments")).unwrap();
        for bad in ["attachments/../x.svg", "attachments/x.svg/../../etc/passwd"] {
            assert!(read_svg(&notes_dir, bad).is_err(), "{bad} should fail");
        }
        std::fs::remove_dir_all(notes_dir).ok();
    }

    #[test]
    fn rel_with_bad_chars_rejected() {
        let notes_dir = tempdir_path();
        assert!(read_svg(&notes_dir, "attachments/../etc/passwd.svg").is_err());
        assert!(read_svg(&notes_dir, "notattachments/x.svg").is_err());
        assert!(read_svg(&notes_dir, "attachments/.svg").is_err());
        std::fs::remove_dir_all(notes_dir).ok();
    }
}
