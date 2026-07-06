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

pub fn cards_generate(note_body: &str, count: u8) -> String {
    format!(
        "Generate {count} question/answer flashcards from the notes below. \
         Output ONLY a JSON array of objects: [{{\"front\":\"...\",\"back\":\"...\"}}]. \
         Use short, atomic facts. Vary difficulty. \
         No preamble, no markdown fences, no commentary.\n\n---\n{note_body}\n---"
    )
}

pub fn quiz_generate(combined_body: &str, count: u8) -> String {
    format!(
        "Generate {count} quiz questions from the notes. Each item is one of:\n\
         - multiple choice: {{\"type\":\"mc\",\"question\":\"...\",\"choices\":[\"A\",\"B\",\"C\",\"D\"],\"answer\":0,\"rationale\":\"...\"}}\n\
         - short answer: {{\"type\":\"short\",\"question\":\"...\",\"answer\":\"...\",\"rationale\":\"...\"}}\n\
         - cloze: {{\"type\":\"cloze\",\"question\":\"...\",\"answer\":\"...\",\"rationale\":\"...\"}}\n\
         Output ONLY a JSON array. The 'answer' for mc is the zero-based index of the correct choice. \
         No markdown fences, no commentary.\n\n---\n{combined_body}\n---"
    )
}

pub fn plan_generate(context: &str, week_start: &str, daily_hours: u8) -> String {
    format!(
        "Produce a weekly study plan starting Monday {week_start}. Daily cap: {daily_hours} hours. \
         Input context follows (tasks due this week + recent note titles + SRS backlog):\n\n---\n{context}\n---\n\n\
         Output ONLY a JSON object matching the schema:\n\
         {{\"week_start\":\"{week_start}\",\"daily_hours_cap\":{daily_hours},\"blocks\":[\n\
           {{\"day\":\"YYYY-MM-DD\",\"start\":\"HH:MM\",\"minutes\":60,\"kind\":\"review|read|quiz|break\",\"refs\":[\"note:<id>\"],\"rationale\":\"...\"}}\n\
         ]}}\n\
         Constraints: blocks must fit within 09:00-21:00 local time, no overlapping minutes, total minutes per day <= {daily_hours}*60. \
         No markdown fences, no commentary."
    )
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

    #[test]
    fn cards_prompt_has_count_and_body() {
        let p = cards_generate("the note", 5);
        assert!(p.contains("5"));
        assert!(p.contains("the note"));
    }

    #[test]
    fn quiz_prompt_requires_json_only_directive() {
        let p = quiz_generate("body", 10);
        assert!(p.contains("JSON"));
        assert!(p.contains("10"));
    }

    #[test]
    fn plan_prompt_embeds_week_and_cap() {
        let p = plan_generate("ctx", "2026-07-06", 4);
        assert!(p.contains("2026-07-06"));
        assert!(p.contains("4"));
    }
}