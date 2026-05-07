use serde::{Deserialize, Serialize};

pub const DEFAULT_MAX_OBSERVATION_CHARS: usize = 16_384;
pub const DEFAULT_MAX_OBSERVATION_LINES: usize = 240;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentObservationPolicy {
    pub max_chars: usize,
    pub max_lines: usize,
    pub preserve_head_lines: usize,
    pub preserve_tail_lines: usize,
    pub redact_secrets: bool,
}

impl Default for AgentObservationPolicy {
    fn default() -> Self {
        Self {
            max_chars: DEFAULT_MAX_OBSERVATION_CHARS,
            max_lines: DEFAULT_MAX_OBSERVATION_LINES,
            preserve_head_lines: 80,
            preserve_tail_lines: 80,
            redact_secrets: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreparedObservation {
    pub content: String,
    pub original_chars: usize,
    pub original_lines: usize,
    pub truncated: bool,
    pub omitted_lines: usize,
    pub redacted_secret_count: usize,
}

pub fn prepare_observation(input: &str, policy: &AgentObservationPolicy) -> PreparedObservation {
    let original_chars = input.chars().count();
    let original_lines = input.lines().count();
    let (redacted, redacted_secret_count) = if policy.redact_secrets {
        redact_secret_lines(input)
    } else {
        (input.to_string(), 0)
    };
    let redacted_lines = redacted.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    let limited_by_lines = truncate_lines(&redacted_lines, policy);
    let limited_by_chars = truncate_chars_preserving_edges(&limited_by_lines, policy.max_chars);
    let truncated = limited_by_chars != redacted || original_chars > policy.max_chars;
    let omitted_lines = original_lines.saturating_sub(limited_by_lines.lines().count());

    PreparedObservation {
        content: limited_by_chars,
        original_chars,
        original_lines,
        truncated,
        omitted_lines,
        redacted_secret_count,
    }
}

fn truncate_lines(lines: &[String], policy: &AgentObservationPolicy) -> String {
    if lines.len() <= policy.max_lines {
        return lines.join("\n");
    }

    let head_count = policy.preserve_head_lines.min(lines.len());
    let remaining = lines.len().saturating_sub(head_count);
    let tail_count = policy.preserve_tail_lines.min(remaining);
    let omitted = lines.len().saturating_sub(head_count + tail_count);

    let mut out = Vec::with_capacity(head_count + tail_count + 1);
    out.extend(lines.iter().take(head_count).cloned());
    out.push(format!("[... {omitted} lines redacted ...]"));
    if tail_count > 0 {
        out.extend(lines.iter().skip(lines.len() - tail_count).cloned());
    }
    out.join("\n")
}

fn truncate_chars_preserving_edges(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let char_count = input.chars().count();
    if char_count <= max_chars {
        return input.to_string();
    }

    let marker = "\n[... observation truncated ...]\n";
    let marker_chars = marker.chars().count();
    if max_chars <= marker_chars + 2 {
        return input.chars().take(max_chars).collect();
    }

    let edge_budget = max_chars - marker_chars;
    let head_chars = edge_budget / 2;
    let tail_chars = edge_budget - head_chars;
    let head = input.chars().take(head_chars).collect::<String>();
    let tail = input
        .chars()
        .rev()
        .take(tail_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}{marker}{tail}")
}

fn redact_secret_lines(input: &str) -> (String, usize) {
    let mut count = 0usize;
    let lines = input
        .lines()
        .map(|line| {
            if looks_secret(line) {
                count += 1;
                "[redacted secret]".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>();
    (lines.join("\n"), count)
}

fn looks_secret(line: &str) -> bool {
    let lower = line.to_lowercase();
    const TERMS: &[&str] = &[
        "api_key",
        "api key",
        "apikey",
        "authorization:",
        "bearer ",
        "password",
        "passwd",
        "secret",
        "token",
        "private_key",
        "client_secret",
    ];
    TERMS.iter().any(|term| lower.contains(term))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_observation_with_head_and_tail() {
        let input = (0..20)
            .map(|index| format!("line-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let observation = prepare_observation(
            &input,
            &AgentObservationPolicy {
                max_chars: 10_000,
                max_lines: 8,
                preserve_head_lines: 3,
                preserve_tail_lines: 2,
                redact_secrets: true,
            },
        );

        assert!(observation.truncated);
        assert!(observation.content.contains("line-0"));
        assert!(observation.content.contains("line-19"));
        assert!(observation.content.contains("[... 15 lines redacted ...]"));
    }

    #[test]
    fn redacts_secret_lines_before_returning_content() {
        let observation = prepare_observation(
            "user=keynova\napi_key = should-not-leak\nok=true",
            &AgentObservationPolicy::default(),
        );

        assert_eq!(observation.redacted_secret_count, 1);
        assert!(observation.content.contains("[redacted secret]"));
        assert!(!observation.content.contains("should-not-leak"));
    }

    #[test]
    fn char_limit_preserves_failure_tail() {
        let input = format!("{}\nPANIC: final failure", "a".repeat(1024));
        let observation = prepare_observation(
            &input,
            &AgentObservationPolicy {
                max_chars: 120,
                max_lines: 240,
                preserve_head_lines: 80,
                preserve_tail_lines: 80,
                redact_secrets: true,
            },
        );

        assert!(observation.truncated);
        assert!(observation.content.contains("PANIC: final failure"));
    }
}
