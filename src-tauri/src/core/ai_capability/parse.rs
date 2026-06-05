//! Shared JSON-extraction helper for capabilities whose models are asked to
//! reply with strict JSON but may still wrap it in prose or markdown fences.

/// Return the first balanced top-level `{...}` object substring, skipping any
/// surrounding prose/markdown. Braces and quotes inside JSON strings (including
/// escaped quotes) are ignored when tracking depth.
pub(crate) fn extract_first_json_object(text: &str) -> Option<&str> {
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    let begin = start?;
                    return Some(&text[begin..=idx]);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_object_from_prose_and_fences() {
        let s = "sure, here:\n```json\n{\"a\": 1}\n```";
        assert_eq!(extract_first_json_object(s), Some("{\"a\": 1}"));
    }

    #[test]
    fn ignores_braces_and_quotes_inside_strings() {
        let s = "{\"k\": \"a}b{c\\\"d\"}";
        assert_eq!(extract_first_json_object(s), Some(s));
    }

    #[test]
    fn none_when_no_object() {
        assert_eq!(extract_first_json_object("no json here"), None);
    }
}
