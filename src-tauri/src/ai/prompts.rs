pub fn summarize(text: &str, style: &str) -> String {
    format!(
        "Summarize the following notes in a {} style. Preserve key facts and definitions.\n\n---\n{}\n---",
        style, text
    )
}

pub fn autocomplete_so_far(text: &str) -> String {
    format!(
        "Continue the following passage naturally. Output ONLY the continuation, no preamble.\n\n---\n{}\n",
        text
    )
}

pub fn rag_system() -> &'static str {
    "You answer questions using the provided note context. Cite note titles in [brackets]. \
     If the context does not contain the answer, say so explicitly."
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_summarize_prompt_includes_style_and_body() {
        let p = summarize("body text here", "short");
        assert!(p.contains("short"), "prompt must contain style keyword");
        assert!(p.contains("body text here"), "prompt must contain body");
    }

    #[test]
    fn test_autocomplete_prompt_has_continuation_directive() {
        let p = autocomplete_so_far("hello");
        assert!(p.contains("Continue"));
        assert!(p.contains("hello"));
    }
}