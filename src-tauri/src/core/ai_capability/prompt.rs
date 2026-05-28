//! Shared prompt assembly + audit emitter for the capability layer.
//!
//! Budget is intentionally smaller than the legacy agent path's
//! `PROMPT_BUDGET_CHARS` because capability calls are single-shot, latency
//! sensitive (ADR-0029 §8 target: P50 < 800 ms with `qwen2.5:7b`).

use crate::core::grounding::truncate;
use crate::core::knowledge_store::{AgentAuditEntry, KnowledgeStoreHandle};
use crate::models::agent::GroundingSource;

pub const CAPABILITY_PROMPT_BUDGET_CHARS: usize = 1400;
pub const CAPABILITY_PROMPT_SOURCE_LIMIT: usize = 6;
pub const CAPABILITY_SNIPPET_MAX_CHARS: usize = 160;

/// Build a single user-turn prompt for a stateless capability call.
///
/// Layout:
/// ```text
/// <system_preamble>
///
/// ### Context
/// - title: snippet
/// ...
///
/// ### Task
/// <user_text>
/// ```
///
/// Sources beyond `CAPABILITY_PROMPT_SOURCE_LIMIT` are dropped; if the total
/// exceeds the char budget, the context block is dropped first; if that still
/// overruns, the result is truncated at the char boundary by
/// [`crate::core::grounding::truncate`].
pub fn build_prompt(system_preamble: &str, sources: &[GroundingSource], user_text: &str) -> String {
    let mut prompt = String::new();
    prompt.push_str(system_preamble.trim());
    prompt.push_str("\n\n");

    if !sources.is_empty() {
        prompt.push_str("### Context\n");
        for s in sources.iter().take(CAPABILITY_PROMPT_SOURCE_LIMIT) {
            prompt.push_str(&format!(
                "- {}: {}\n",
                s.title,
                truncate(&s.snippet, CAPABILITY_SNIPPET_MAX_CHARS)
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str("### Task\n");
    prompt.push_str(user_text);

    if prompt.chars().count() <= CAPABILITY_PROMPT_BUDGET_CHARS {
        return prompt;
    }

    let mut shrunk = String::new();
    shrunk.push_str(system_preamble.trim());
    shrunk.push_str("\n\n### Task\n");
    shrunk.push_str(user_text);
    if shrunk.chars().count() > CAPABILITY_PROMPT_BUDGET_CHARS {
        return truncate(&shrunk, CAPABILITY_PROMPT_BUDGET_CHARS);
    }
    shrunk
}

/// Emit an audit entry for a capability call. No-op when `audit_required`
/// is false or `store` is None. Audit decision is set at capability
/// registration, not in the per-call return (ADR-0030 §4).
pub fn maybe_audit(
    store: Option<&KnowledgeStoreHandle>,
    audit_required: bool,
    capability_id: &str,
    status: &str,
    summary: &str,
    payload_json: Option<String>,
) {
    if !audit_required {
        return;
    }
    let Some(store) = store else { return };
    store.try_log_agent_audit(AgentAuditEntry {
        run_id: format!("capability:{capability_id}"),
        event_type: "capability_call".into(),
        status: status.into(),
        summary: summary.to_string(),
        payload_json,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::agent::ContextVisibility;

    fn src(title: &str, snippet: &str) -> GroundingSource {
        GroundingSource {
            source_id: format!("t:{title}"),
            source_type: "test".into(),
            title: title.into(),
            snippet: snippet.into(),
            uri: None,
            score: 0.5,
            visibility: ContextVisibility::PublicContext,
            redacted_reason: None,
        }
    }

    #[test]
    fn build_prompt_includes_sources_when_under_budget() {
        let p = build_prompt(
            "you are a tester",
            &[src("alpha", "first snippet")],
            "do the thing",
        );
        assert!(p.contains("### Context"));
        assert!(p.contains("alpha"));
        assert!(p.contains("### Task"));
        assert!(p.contains("do the thing"));
    }

    #[test]
    fn build_prompt_drops_context_when_over_budget() {
        // Long task body pushes total over the budget even with snippet truncation
        // → the context block must be dropped.
        let long_task = "x".repeat(CAPABILITY_PROMPT_BUDGET_CHARS);
        let sources: Vec<_> = (0..6).map(|i| src(&format!("s{i}"), "snippet")).collect();
        let p = build_prompt("sys", &sources, &long_task);
        assert!(p.contains("### Task"));
        assert!(!p.contains("### Context"));
    }

    #[test]
    fn build_prompt_caps_source_count() {
        let sources: Vec<_> = (0..20).map(|i| src(&format!("s{i}"), "snippet")).collect();
        let p = build_prompt("sys", &sources, "task");
        let count = p.matches("- s").count();
        assert!(count <= CAPABILITY_PROMPT_SOURCE_LIMIT);
    }

    #[test]
    fn maybe_audit_is_noop_when_disabled() {
        // Just exercise the no-op path; success means no panic.
        maybe_audit(None, false, "explain", "ok", "x", None);
        maybe_audit(None, true, "explain", "ok", "x", None);
    }
}
