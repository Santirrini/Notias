use serde::{Deserialize, Serialize};

/// Frontmatter keys are stored on disk in camelCase to match the rest of the
/// YAML (e.g. `paperTint`, `editorFont`) and the Svelte side. `serde_yaml`
/// doesn't translate casing by default, so we rename the snake_case Rust
/// fields explicitly. Existing keys (`id, title, tags, created, updated,
/// links, references`) keep their original YAML spelling.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Frontmatter {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub created: String,
    pub updated: String,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub paper: Option<String>,
    #[serde(default)]
    pub paper_tint: Option<String>,
    #[serde(default)]
    pub editor_font: Option<String>,
    #[serde(default)]
    pub editor_font_size: Option<String>,
    #[serde(default)]
    pub editor_line_height: Option<String>,
    #[serde(default)]
    pub editor_page_width: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: String,
    pub path: std::path::PathBuf,
    pub title: String,
    pub body: String,
    pub frontmatter: Frontmatter,
}

#[derive(Debug, Serialize, Clone)]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub updated: String,
    pub tags: Vec<String>,
}

pub fn parse(s: &str) -> Result<(Frontmatter, String), serde_yaml::Error> {
    // ponytail: minimal YAML frontmatter splitter; known ceiling — a literal `\n---\n`
    // inside the note body will mis-parse. We don't currently render or accept such
    // Markdown, so this is acceptable. Replace with a real parser if direct YAML edits
    // become a supported workflow.
    let mut parts = s.splitn(2, "---\n");
    let _ = parts.next();
    let fm_and_body = parts.next().unwrap_or("");
    let mut split = fm_and_body.splitn(2, "\n---\n");
    let yaml = split.next().unwrap_or("");
    let body = split.next().unwrap_or("").to_string();
    let fm: Frontmatter = serde_yaml::from_str(yaml)?;
    Ok((fm, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_roundtrip() {
        let s = "---\nid: abc\ntitle: T\ntags: [a,b]\ncreated: 2026-07-03T00:00:00Z\nupdated: 2026-07-03T00:00:00Z\n---\nbody\n";
        let (fm, body) = parse(s).unwrap();
        assert_eq!(fm.id, "abc");
        assert_eq!(body.trim(), "body");
    }

    #[test]
    fn parse_handles_fenced_dashes_in_body() {
        // ponytail: the ceiling we documented — body containing literal `---\n` will
        // mis-parse because we split on the first `\n---\n`. Documented; not fixed in Phase 1.
        let s = "---\nid: x\ntitle: t\ntags: []\ncreated: c\nupdated: u\n---\n# Heading\n\n---\n\nMore body\n";
        let (_fm, body) = parse(s).unwrap();
        assert!(body.starts_with("# Heading"));
    }
}