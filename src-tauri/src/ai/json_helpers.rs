//! Most LLM outputs drift even when the prompt forbids it. We need one helper to
//! find the first complete JSON object or array in the response, ignoring any
//! prose or markdown fences before/after. Same parser used by cards, quiz, plan.

use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Copy)]
pub enum JsonShape {
    Object,
    Array,
}

pub fn first_json<T: DeserializeOwned>(text: &str, shape: JsonShape) -> Result<T, String> {
    let needle = match shape {
        JsonShape::Object => '{',
        JsonShape::Array => '[',
    };
    let label = match shape {
        JsonShape::Object => "object",
        JsonShape::Array => "array",
    };
    let mut start: Option<usize> = None;
    for (i, ch) in text.char_indices() {
        if ch == needle {
            start = Some(i);
            break;
        }
    }
    let Some(start_idx) = start else {
        return Err(format!("no JSON {label} found in output"));
    };
    let closer: char = match shape {
        JsonShape::Object => '}',
        JsonShape::Array => ']',
    };
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    for (i, &c) in bytes.iter().enumerate().skip(start_idx) {
        if escape {
            escape = false;
            continue;
        }
        if c == b'\\' && in_string {
            escape = true;
            continue;
        }
        if c == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if c == needle as u8 {
            depth += 1;
        }
        if c == closer as u8 {
            depth -= 1;
            if depth == 0 {
                let slice = &text[start_idx..=i];
                return serde_json::from_str::<T>(slice)
                    .map_err(|e| format!("json parse failed: {e}"));
            }
        }
    }
    Err("no matching closer found".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Q {
        q: u32,
    }

    #[test]
    fn extracts_object_with_prose_around() {
        let s = "Sure, here you go: {\"q\":7} -- hope this helps.";
        let out: Q = first_json(s, JsonShape::Object).unwrap();
        assert_eq!(out, Q { q: 7 });
    }

    #[test]
    fn extracts_array() {
        let s = "items: [1,2,3] end.";
        let v: Vec<u32> = first_json(s, JsonShape::Array).unwrap();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn handles_strings_with_braces() {
        let s = "{\"q\":7,\"msg\":\"curly { brace in string }\"}";
        let out: Q = first_json(s, JsonShape::Object).unwrap();
        assert_eq!(out.q, 7);
    }

    #[test]
    fn errors_when_missing() {
        assert!(first_json::<Q>("no JSON here", JsonShape::Object).is_err());
    }

    #[test]
    fn finds_nested_object() {
        let s = "noise {\"a\":{\"b\":[1,2]}} tail";
        #[derive(Deserialize, PartialEq, Debug)]
        struct A { a: serde_json::Value }
        let out: A = first_json(s, JsonShape::Object).unwrap();
        assert_eq!(out.a["b"][1].as_i64().unwrap(), 2);
    }
}
