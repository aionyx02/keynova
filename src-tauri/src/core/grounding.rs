//! Pure helpers for constructing [`GroundingSource`] values and matching strings.
//!
//! Lives under `core/` so that both `handlers/agent/*` (the legacy chat-first
//! path) and `core/local_context.rs` (and the upcoming `core/ai_capability/`
//! layer in REF.4) can reuse the same redaction and truncation rules without
//! pulling a handler dependency into the core layer.

use crate::models::agent::{ContextVisibility, GroundingSource};

/// Build a `GroundingSource` with snippet truncation applied.
pub fn source(
    source_id: String,
    source_type: &str,
    title: String,
    snippet: String,
    score: f32,
    visibility: ContextVisibility,
) -> GroundingSource {
    GroundingSource {
        source_id,
        source_type: source_type.into(),
        title,
        snippet: truncate(&snippet, 240),
        uri: None,
        score,
        visibility,
        redacted_reason: None,
    }
}

/// Build a `GroundingSource` with visibility classified by content inspection.
///
/// - Matches private-architecture markers → tags `PrivateArchitecture` and
///   replaces the snippet with a redaction placeholder.
/// - Matches secret-shaped markers → tags `Secret` and replaces the snippet
///   with `[redacted secret]`.
/// - Otherwise → tags `UserPrivate` and keeps the snippet.
pub fn visibility_filtered_source(
    source_id: String,
    source_type: &str,
    title: String,
    snippet: String,
    score: f32,
) -> GroundingSource {
    let combined = format!("{title} {snippet}").to_lowercase();
    if contains_any(
        &combined,
        &[
            "CLAUDE.md",
            "tasks.md",
            "memory.md",
            "decisions.md",
            "skill.md",
            "private_architecture",
            "architecture",
        ],
    ) {
        return GroundingSource {
            source_id,
            source_type: source_type.into(),
            title,
            snippet: "[redacted private architecture context]".into(),
            uri: None,
            score,
            visibility: ContextVisibility::PrivateArchitecture,
            redacted_reason: Some("private_architecture".into()),
        };
    }
    if contains_any(
        &combined,
        &[
            "api_key", "api key", "password", "token", "secret", "sk-", "bearer ",
        ],
    ) {
        return GroundingSource {
            source_id,
            source_type: source_type.into(),
            title,
            snippet: "[redacted secret]".into(),
            uri: None,
            score,
            visibility: ContextVisibility::Secret,
            redacted_reason: Some("secret".into()),
        };
    }
    source(
        source_id,
        source_type,
        title,
        snippet,
        score,
        ContextVisibility::UserPrivate,
    )
}

/// Parse a stored visibility string back into a [`ContextVisibility`] enum.
pub fn parse_visibility(value: &str) -> ContextVisibility {
    match value {
        "public_context" => ContextVisibility::PublicContext,
        "private_architecture" => ContextVisibility::PrivateArchitecture,
        "secret" => ContextVisibility::Secret,
        _ => ContextVisibility::UserPrivate,
    }
}

/// Truncate a `&str` to `max_chars` Unicode code points, appending `...` when truncation occurs.
pub fn truncate(value: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut truncated = false;
    for (count, ch) in value.chars().enumerate() {
        if count >= max_chars {
            truncated = true;
            break;
        }
        out.push(ch);
    }
    if truncated {
        out.push_str("...");
    }
    out
}

/// Returns true if `haystack` contains any of the `needles` as a substring.
pub fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}