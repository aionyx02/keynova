//! Scoring, sorting, per-source quota balancing, dedup keys, and the
//! `:global` prefix parser for search results.
//!
//! Focused ranking helpers used by the search handler facade.

use std::collections::HashSet;

use crate::models::action::UiSearchItem;

use super::{APP_LIMIT, COMMAND_LIMIT, HISTORY_LIMIT, MODEL_LIMIT, NOTE_LIMIT};

/// Score a command match.
/// - Name prefix match    → 90 (user is typing the command name)
/// - Name substring match → 85 (partial name match)
/// - Description only     → 40 (below file results at 80; keeps discoverability without crowding out files)
/// - No match             → None
pub(super) fn command_match_score(name: &str, description: &str, q: &str) -> Option<i64> {
    if q.is_empty() {
        return None;
    }
    if name.starts_with(q) {
        Some(90)
    } else if name.contains(q) {
        Some(85)
    } else if description.to_lowercase().contains(q) {
        Some(40)
    } else {
        None
    }
}

pub(super) fn sort_truncate(results: &mut Vec<UiSearchItem>, limit: usize) {
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.title.cmp(&right.title))
    });
    results.truncate(limit);
}

pub(super) fn sort_balanced_truncate(results: &mut Vec<UiSearchItem>, limit: usize) {
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| source_priority(&left.source).cmp(&source_priority(&right.source)))
            .then_with(|| left.title.cmp(&right.title))
    });

    let mut accepted = Vec::new();
    let mut app_count = 0usize;
    let mut command_count = 0usize;
    let mut note_count = 0usize;
    let mut history_count = 0usize;
    let mut model_count = 0usize;
    let mut file_count = 0usize;

    for item in results.drain(..) {
        let allowed = match item.source.as_str() {
            "app" => {
                app_count += 1;
                app_count <= APP_LIMIT
            }
            "command" => {
                command_count += 1;
                command_count <= COMMAND_LIMIT
            }
            "note" => {
                note_count += 1;
                note_count <= NOTE_LIMIT
            }
            "history" => {
                history_count += 1;
                history_count <= HISTORY_LIMIT
            }
            "model" => {
                model_count += 1;
                model_count <= MODEL_LIMIT
            }
            "file" => {
                file_count += 1;
                file_count <= limit
            }
            _ => true,
        };

        if allowed {
            accepted.push(item);
        }

        if accepted.len() >= limit {
            break;
        }
    }

    *results = accepted;
}

fn source_priority(source: &str) -> usize {
    match source {
        "app" => 0,
        "command" => 1,
        "file" => 2,
        "note" => 3,
        "history" => 4,
        "model" => 5,
        _ => 9,
    }
}

pub(super) fn result_keys(results: &[UiSearchItem]) -> HashSet<String> {
    results.iter().map(search_item_key).collect()
}

pub(super) fn search_item_key(item: &UiSearchItem) -> String {
    format!("{}:{}", item.source, item.path)
}

/// Strips a leading `:global` token (with or without trailing space)
/// from the raw query. Returns `(cleaned_query, was_global)`. Pure so it can be
/// unit-tested without constructing a SearchHandler.
pub(super) fn strip_global_prefix(raw: &str) -> (String, bool) {
    let trimmed = raw.trim_start();
    if trimmed == ":global" {
        return (String::new(), true);
    }
    if let Some(rest) = trimmed.strip_prefix(":global ") {
        return (rest.trim_start().to_string(), true);
    }
    (raw.to_string(), false)
}
