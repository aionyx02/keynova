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

/// PRODUCT.1.A workspace_context term. Rewards a file/folder/app result whose
/// path lives under the active workspace `project_root` so that, in `:global`
/// mode (or when no hard filter applies), the in-workspace copy of a same-named
/// file outranks copies elsewhere. Non-file sources (synthetic `command://`,
/// `note://`, … paths) and an unset root both yield 0 — a safe no-op default.
pub(super) fn workspace_boost(source: &str, path: &str, project_root: Option<&str>) -> i64 {
    // `folder` results carry source "file"; apps carry "app".
    if !matches!(source, "file" | "app") {
        return 0;
    }
    match project_root {
        Some(root) if path_under_root(path, root) => 20,
        _ => 0,
    }
}

/// PRODUCT.1.A README/config nudge. Small additive boost so project entry points
/// (README, manifests, config files) surface above incidental files. File source
/// only; deliberately a short allow-list + a few config extensions rather than
/// every `.json`/data file.
pub(super) fn config_boost(source: &str, path: &str) -> i64 {
    if source != "file" {
        return 0;
    }
    let normalized = path.replace('\\', "/");
    let name = normalized.rsplit('/').next().unwrap_or(&normalized).to_lowercase();
    let is_readme = name.starts_with("readme");
    let is_named_config = matches!(
        name.as_str(),
        "cargo.toml"
            | "package.json"
            | "tsconfig.json"
            | "makefile"
            | "justfile"
            | "pyproject.toml"
            | "docker-compose.yml"
            | "docker-compose.yaml"
            | ".env"
    );
    let is_config_ext = [".toml", ".yml", ".yaml", ".ini", ".cfg"]
        .iter()
        .any(|ext| name.ends_with(ext));
    if is_readme || is_named_config || is_config_ext {
        6
    } else {
        0
    }
}

/// Case- and separator-insensitive "is `path` inside `root`?" check. An exact
/// match counts (selecting the project dir itself).
fn path_under_root(path: &str, root: &str) -> bool {
    let normalize = |s: &str| s.replace('\\', "/").trim_end_matches('/').to_lowercase();
    let p = normalize(path);
    let r = normalize(root);
    if r.is_empty() {
        return false;
    }
    p == r || p.starts_with(&format!("{r}/"))
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

#[cfg(test)]
mod tests {
    use super::{config_boost, workspace_boost};

    const ROOT: &str = "C:/projA";

    #[test]
    fn workspace_boost_rewards_file_under_root() {
        assert_eq!(workspace_boost("file", "C:/projA/config.toml", Some(ROOT)), 20);
        assert_eq!(workspace_boost("app", "C:/projA/bin/app.exe", Some(ROOT)), 20);
    }

    #[test]
    fn workspace_boost_zero_outside_root_or_unset() {
        assert_eq!(workspace_boost("file", "C:/projB/config.toml", Some(ROOT)), 0);
        assert_eq!(workspace_boost("file", "C:/projA/config.toml", None), 0);
        // The headline scenario: same-named file inside beats the one outside.
        let inside = workspace_boost("file", "C:/projA/config.toml", Some(ROOT));
        let outside = workspace_boost("file", "C:/projB/config.toml", Some(ROOT));
        assert!(inside > outside);
    }

    #[test]
    fn workspace_boost_is_case_and_separator_insensitive() {
        assert_eq!(workspace_boost("file", r"c:\proja\src\main.rs", Some(ROOT)), 20);
    }

    #[test]
    fn workspace_boost_ignores_non_file_sources() {
        assert_eq!(workspace_boost("command", "command://help", Some(ROOT)), 0);
        assert_eq!(workspace_boost("note", "note://todo", Some(ROOT)), 0);
    }

    #[test]
    fn config_boost_rewards_readme_and_manifests() {
        assert_eq!(config_boost("file", "C:/x/README.md"), 6);
        assert_eq!(config_boost("file", "C:/x/Cargo.toml"), 6);
        assert_eq!(config_boost("file", "C:/x/settings.yaml"), 6);
    }

    #[test]
    fn config_boost_zero_for_plain_files_and_non_files() {
        assert_eq!(config_boost("file", "C:/x/notes.txt"), 0);
        assert_eq!(config_boost("file", "C:/x/data.json"), 0);
        assert_eq!(config_boost("app", "C:/x/Cargo.toml"), 0);
    }
}
