use serde::{Deserialize, Serialize};

use crate::models::agent::GroundingSource;

/// Maximum chars per search result snippet before applying bounded preview.
const SNIPPET_PREVIEW_CHARS: usize = 200;

/// Structured workspace metadata included in a context bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub id: String,
    pub name: String,
    pub project_root: Option<String>,
    pub recent_file_count: usize,
    pub note_count: usize,
}

/// A recently accessed file path with an optional bounded-content preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedFileContext {
    pub path: String,
    /// Content preview limited to SNIPPET_PREVIEW_CHARS; empty when unread.
    pub preview: String,
}

/// Char-count approximation of how much of the context budget was consumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTokenBudget {
    pub budget_chars: usize,
    pub used_chars: usize,
    pub remaining_chars: usize,
    /// True when any content was dropped or trimmed to fit the budget.
    pub truncated: bool,
}

/// Bounded, budget-aware context assembled from managers before an agent run.
///
/// Built from WorkspaceManager, HistoryManager, and local search only —
/// no recursive filesystem scan. Search result snippets are capped at
/// SNIPPET_PREVIEW_CHARS and the total char count is bounded by `budget_chars`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    pub user_intent: String,
    pub workspace: WorkspaceContext,
    /// Recent workspace action IDs (last N, oldest first).
    pub recent_actions: Vec<String>,
    /// Recently accessed workspace files with optional bounded previews.
    pub selected_files: Vec<SelectedFileContext>,
    /// Local search results from managers, snippet-trimmed and budget-capped.
    pub search_results: Vec<GroundingSource>,
    pub token_budget: ContextTokenBudget,
}

impl ContextBundle {
    /// Construct a `ContextBundle` from pre-assembled manager data.
    ///
    /// Applies two budget passes:
    /// 1. Snippets longer than `SNIPPET_PREVIEW_CHARS` are trimmed with `…`.
    /// 2. Search results are dropped once cumulative char count exceeds `budget_chars`.
    pub fn build(
        user_intent: String,
        workspace: WorkspaceContext,
        recent_actions: Vec<String>,
        selected_files: Vec<SelectedFileContext>,
        search_results: Vec<GroundingSource>,
        budget_chars: usize,
    ) -> Self {
        let mut used = user_intent.chars().count();
        for action in &recent_actions {
            used += action.chars().count() + 2;
        }
        for file in &selected_files {
            used += file.path.chars().count() + file.preview.chars().count() + 4;
        }

        let mut bounded_results: Vec<GroundingSource> = Vec::new();
        let mut truncated = false;

        for mut result in search_results {
            // Bounded preview pass — trim long snippets.
            if result.snippet.chars().count() > SNIPPET_PREVIEW_CHARS {
                let trimmed: String = result.snippet.chars().take(SNIPPET_PREVIEW_CHARS).collect();
                result.snippet = format!("{trimmed}…");
                truncated = true;
            }
            let weight = result.title.chars().count() + result.snippet.chars().count() + 4;
            if used + weight > budget_chars {
                truncated = true;
                break;
            }
            used += weight;
            bounded_results.push(result);
        }

        let remaining = budget_chars.saturating_sub(used);

        Self {
            user_intent,
            workspace,
            recent_actions,
            selected_files,
            search_results: bounded_results,
            token_budget: ContextTokenBudget {
                budget_chars,
                used_chars: used,
                remaining_chars: remaining,
                truncated,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::agent::{ContextVisibility, GroundingSource};

    fn make_source(id: &str, snippet: &str) -> GroundingSource {
        GroundingSource {
            source_id: id.to_string(),
            source_type: "test".to_string(),
            title: id.to_string(),
            snippet: snippet.to_string(),
            uri: None,
            score: 0.5,
            visibility: ContextVisibility::PublicContext,
            redacted_reason: None,
        }
    }

    fn minimal_workspace() -> WorkspaceContext {
        WorkspaceContext {
            id: "0".into(),
            name: "Test".into(),
            project_root: None,
            recent_file_count: 0,
            note_count: 0,
        }
    }

    #[test]
    fn budget_caps_search_results() {
        let sources = vec![
            make_source("a", "short"),
            make_source("b", "short"),
            make_source("c", "short"),
            make_source("d", "short"),
        ];
        // Tiny budget — forces early drop.
        let bundle = ContextBundle::build(
            "hi".into(),
            minimal_workspace(),
            Vec::new(),
            Vec::new(),
            sources,
            20,
        );
        assert!(bundle.token_budget.truncated);
        assert!(bundle.search_results.len() < 4);
    }

    #[test]
    fn long_snippet_is_trimmed_to_preview_limit() {
        let long_snippet = "x".repeat(300);
        let sources = vec![make_source("a", &long_snippet)];
        let bundle = ContextBundle::build(
            "test".into(),
            minimal_workspace(),
            Vec::new(),
            Vec::new(),
            sources,
            10_000,
        );
        let snippet = &bundle.search_results[0].snippet;
        // 200 content chars + 1 ellipsis = 201 max.
        assert!(snippet.chars().count() <= 201);
        assert!(snippet.ends_with('…'));
        assert!(bundle.token_budget.truncated);
    }

    #[test]
    fn small_results_fit_without_truncation() {
        let sources = vec![make_source("a", "abc"), make_source("b", "def")];
        let bundle = ContextBundle::build(
            "hello".into(),
            minimal_workspace(),
            Vec::new(),
            Vec::new(),
            sources,
            10_000,
        );
        assert!(!bundle.token_budget.truncated);
        assert_eq!(bundle.search_results.len(), 2);
        assert!(bundle.token_budget.remaining_chars > 0);
    }
}