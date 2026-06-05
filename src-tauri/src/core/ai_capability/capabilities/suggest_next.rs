//! Capability: suggest likely next actions from workflow history.
//!
//! REF.6.C keeps this backend-first: the output is structured, replay is
//! best-effort, and no UI surface is wired in this batch.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::registry::CapabilityId;
use crate::core::workflow_memory;
use crate::models::unified_result::RiskTag;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SuggestNextCtx {
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct Payload {
    #[serde(default)]
    ctx: SuggestNextCtx,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayActionDescriptor {
    pub route: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SuggestedNextAction {
    pub title: String,
    pub subtitle: String,
    pub route: String,
    pub confidence: f32,
    pub rationale: String,
    pub last_executed_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay: Option<ReplayActionDescriptor>,
}

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload)
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    let limit = workflow_memory::clamp_limit(payload.ctx.limit);

    let rows = match deps.knowledge_store.as_ref() {
        Some(store) => workflow_memory::suggest(store, req.context_hash.as_deref(), limit)
            .map_err(CapabilityError::ProviderError)?,
        None => Vec::new(),
    };

    let suggestions = rank_rows(rows, req.context_hash.as_deref());
    Ok(CapabilityResponse {
        id: CapabilityId::SuggestNext,
        output: CapabilityOutput::Structured {
            value: serde_json::to_value(suggestions).unwrap_or_else(|_| Value::Array(Vec::new())),
        },
        risk_tag: RiskTag::none(),
    })
}

fn rank_rows(
    rows: Vec<workflow_memory::WorkflowHistoryRow>,
    active_context_hash: Option<&str>,
) -> Vec<SuggestedNextAction> {
    let mut seen = HashSet::new();
    let mut suggestions = Vec::new();

    for (idx, row) in rows.into_iter().enumerate() {
        if is_suggest_next_self_row(&row) {
            continue;
        }
        let dedupe_key = format!("{}::{}", row.route, row.action_label);
        if !seen.insert(dedupe_key) {
            continue;
        }

        let same_context = active_context_hash
            .zip(row.context_hash.as_deref())
            .is_some_and(|(active, row_hash)| active == row_hash);
        let confidence = score_row(&row.route, idx, same_context);
        suggestions.push(SuggestedNextAction {
            title: row.action_label.clone(),
            subtitle: build_subtitle(&row.route, idx, same_context),
            route: row.route.clone(),
            confidence,
            rationale: build_rationale(&row.route, same_context),
            last_executed_at: row.executed_at,
            workspace_id: row.workspace_id,
            replay: replay_for_row(&row.route, &row.action_label),
        });
    }

    suggestions.sort_by(|left, right| {
        right
            .replay
            .is_some()
            .cmp(&left.replay.is_some())
            .then_with(|| {
                right
                    .confidence
                    .partial_cmp(&left.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.last_executed_at.cmp(&left.last_executed_at))
    });

    suggestions
}

fn is_suggest_next_self_row(row: &workflow_memory::WorkflowHistoryRow) -> bool {
    row.route == "capability.call" && row.action_label.starts_with("suggest_next")
}

fn score_row(route: &str, recency_index: usize, same_context: bool) -> f32 {
    let recency = (1.0 - (recency_index as f32 * 0.12)).max(0.2);
    let route_bonus = match route {
        "cmd.run" => 0.18,
        "action.run" => 0.12,
        "capability.call" => 0.08,
        _ => 0.04,
    };
    let context_bonus = if same_context { 0.1 } else { 0.0 };
    (recency + route_bonus + context_bonus).clamp(0.0, 0.99)
}

fn build_subtitle(route: &str, recency_index: usize, same_context: bool) -> String {
    let freshness = match recency_index {
        0 => "most recent",
        1 => "recent",
        2..=4 => "still warm",
        _ => "older history",
    };
    let scope = if same_context {
        "same context"
    } else {
        "global history"
    };
    format!("{route} · {freshness} · {scope}")
}

fn build_rationale(route: &str, same_context: bool) -> String {
    let route_reason = match route {
        "cmd.run" => "recent command usage",
        "action.run" => "recent primary action",
        "capability.call" => "recent AI capability usage",
        _ => "recent workflow activity",
    };
    if same_context {
        format!("{route_reason} in the current workspace context")
    } else {
        route_reason.to_string()
    }
}

fn replay_for_row(route: &str, title: &str) -> Option<ReplayActionDescriptor> {
    if route != "cmd.run" {
        return None;
    }

    let trimmed = title.trim().strip_prefix('/')?;
    let mut parts = trimmed.splitn(2, ' ');
    let name = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    let args = parts.next().unwrap_or("").trim();
    Some(ReplayActionDescriptor {
        route: route.to_string(),
        payload: json!({
            "name": name,
            "args": args,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
    use crate::core::knowledge_store::{KnowledgeStoreHandle, WorkflowHistoryEntry};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct UnusedProvider;
    impl ChatProvider for UnusedProvider {
        fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok(String::new())
        }
    }

    fn temp_db_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("keynova-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("knowledge.db")
    }

    fn cleanup(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    fn deps_with_store(store: KnowledgeStoreHandle) -> CapabilityDeps {
        CapabilityDeps {
            chat: Arc::new(UnusedProvider),
            local_context: None,
            knowledge_store: Some(store),
            cancel: Arc::new(AtomicBool::new(false)),
            stream_chunk: None,
            allow_memory_grounding: false,
        }
    }

    fn req(payload: serde_json::Value, context_hash: Option<&str>) -> CapabilityRequest {
        CapabilityRequest {
            id: CapabilityId::SuggestNext,
            payload,
            context_hash: context_hash.map(str::to_owned),
        }
    }

    #[test]
    fn returns_ranked_structured_suggestions() {
        let path = temp_db_path("suggest-next");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            store.try_log_workflow_history(WorkflowHistoryEntry {
                context_hash: Some("ctx-a".into()),
                route: "cmd.run".into(),
                action_label: "/help".into(),
                payload_digest: None,
                workspace_id: Some(7),
            });
            store.try_log_workflow_history(WorkflowHistoryEntry {
                context_hash: Some("ctx-a".into()),
                route: "capability.call".into(),
                action_label: "explain rust ownership".into(),
                payload_digest: None,
                workspace_id: Some(7),
            });

            let resp = call(
                req(serde_json::json!({ "ctx": { "limit": 5 } }), Some("ctx-a")),
                &deps_with_store(store),
            )
            .unwrap();
            assert_eq!(resp.id, CapabilityId::SuggestNext);
            assert!(!resp.risk_tag.requires_confirmation);
            match resp.output {
                CapabilityOutput::Structured { value } => {
                    let items: Vec<SuggestedNextAction> = serde_json::from_value(value).unwrap();
                    assert_eq!(items.len(), 2);
                    assert_eq!(items[0].title, "/help");
                    assert!(items[0].replay.is_some());
                    assert!(items[0].confidence >= items[1].confidence);
                }
                _ => panic!("expected structured output"),
            }
        }
        cleanup(&path);
    }

    #[test]
    fn dedupes_identical_route_and_title_pairs() {
        let rows = vec![
            workflow_memory::WorkflowHistoryRow {
                id: 2,
                context_hash: Some("ctx".into()),
                route: "cmd.run".into(),
                action_label: "/help".into(),
                payload_digest: None,
                workspace_id: Some(1),
                executed_at: 20,
            },
            workflow_memory::WorkflowHistoryRow {
                id: 1,
                context_hash: Some("ctx".into()),
                route: "cmd.run".into(),
                action_label: "/help".into(),
                payload_digest: None,
                workspace_id: Some(1),
                executed_at: 10,
            },
        ];
        let items = rank_rows(rows, Some("ctx"));
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn drops_suggest_next_self_rows() {
        let rows = vec![
            workflow_memory::WorkflowHistoryRow {
                id: 2,
                context_hash: Some("ctx".into()),
                route: "capability.call".into(),
                action_label: "suggest_next".into(),
                payload_digest: None,
                workspace_id: Some(1),
                executed_at: 20,
            },
            workflow_memory::WorkflowHistoryRow {
                id: 1,
                context_hash: Some("ctx".into()),
                route: "cmd.run".into(),
                action_label: "/help".into(),
                payload_digest: None,
                workspace_id: Some(1),
                executed_at: 10,
            },
        ];
        let items = rank_rows(rows, Some("ctx"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "/help");
    }

    #[test]
    fn replay_descriptor_is_only_built_for_command_rows() {
        assert!(replay_for_row("action.run", "Open search.rs").is_none());
        let replay = replay_for_row("cmd.run", "/setting launcher.opacity 0.9").unwrap();
        assert_eq!(replay.route, "cmd.run");
        assert_eq!(replay.payload["name"], "setting");
        assert_eq!(replay.payload["args"], "launcher.opacity 0.9");
    }
}
