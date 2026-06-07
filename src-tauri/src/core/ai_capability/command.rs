//! Shared helpers for copy-only command suggestions.

use serde::{Deserialize, Serialize};

use crate::models::unified_result::RiskTag;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandSuggestion {
    pub command: String,
    pub confidence: f32,
    pub rationale: String,
}

pub(crate) fn risk_tag_for_command(command: &str) -> RiskTag {
    let normalized = command.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return RiskTag::confirm("generated command was empty");
    }

    const SAFE_PREFIXES: &[&str] = &[
        "rg ",
        "grep ",
        "ls",
        "dir",
        "cat ",
        "type ",
        "git status",
        "git diff",
        "git log",
        "cargo test",
        "cargo check",
        "cargo clippy",
        "npm test",
        "npm run lint",
        "pnpm test",
        "yarn test",
    ];

    if SAFE_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        RiskTag::none()
    } else {
        RiskTag::confirm("generated command may change local system state")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_commands_do_not_require_confirmation() {
        assert!(!risk_tag_for_command("git status").requires_confirmation);
        assert!(!risk_tag_for_command("cargo test").requires_confirmation);
    }

    #[test]
    fn state_changing_commands_are_displayed_as_confirm_risk() {
        assert!(risk_tag_for_command("git push origin HEAD").requires_confirmation);
    }
}
