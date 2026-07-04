use std::collections::HashSet;

pub fn extract_links(body: &str) -> Vec<String> {
    // ponytail: hand-rolled scanner; no regex dep for one pattern.
    let mut out = HashSet::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(end) = find_close(&bytes[i + 2..]) {
                let target = &body[i + 2..i + 2 + end];
                let stripped = target.split('|').next().unwrap_or(target).trim();
                if !stripped.is_empty() { out.insert(stripped.to_string()); }
                i += 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
    let mut v: Vec<String> = out.into_iter().collect();
    v.sort();
    v
}

fn find_close(b: &[u8]) -> Option<usize> {
    for i in 0..b.len().saturating_sub(1) {
        if b[i] == b']' && b[i + 1] == b']' { return Some(i); }
    }
    None
}

pub fn extract_attachments(body: &str) -> Vec<String> {
    // ponytail: ceiling — the `](` scanner triggers inside fenced code blocks; the result is
    // usually a non-attachment URL that we filter out via the `attachments/` prefix check.
    // Phase 1 ships this; Phase 1.5 polish moves the scan behind a proper Markdown tokenizer
    // if false positives actually surface.
    let mut out = vec![];
    let bytes = body.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b'(' {
            if let Some(end) = find_paren_close(&bytes[i + 2..]) {
                let path = &body[i + 2..i + 2 + end];
                if path.starts_with("attachments/") {
                    out.push(path.to_string());
                }
                i += 2 + end + 1;
                continue;
            }
        }
        i += 1;
    }
    out.sort();
    out.dedup();
    out
}

fn find_paren_close(b: &[u8]) -> Option<usize> {
    b.iter().position(|&c| c == b')')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_wiki_links() {
        let body = "see [[Rust]] and [[Borrow Checker|the checker]]; ignore [[]]";
        let l = extract_links(body);
        assert_eq!(l, vec!["Borrow Checker".to_string(), "Rust".to_string()]);
    }

    #[test]
    fn extracts_attachments() {
        let body = "![photo](attachments/abc/p.png) and ![x](https://example.com)";
        assert_eq!(extract_attachments(body), vec!["attachments/abc/p.png".to_string()]);
    }
}