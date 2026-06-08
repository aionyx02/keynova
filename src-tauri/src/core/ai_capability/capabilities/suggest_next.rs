//! Capability: suggest likely next actions from workflow history.
//!
//! REF.6.C keeps this backend-first: the output is structured, replay is
//! best-effort, and no UI surface is wired in this batch.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::registry::CapabilityId;
use crate::core::local_context::LocalContextSearcher;
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

const MAX_SUGGESTION_AGE_SECS: i64 = 30 * 24 * 60 * 60;
const HISTORY_ONLY_MAX_AGE_SECS: i64 = 7 * 24 * 60 * 60;

/// History window pulled for transition modeling. Wider than the display limit
/// so the `(prev → next)` model has enough pairs; output is truncated to the
/// caller's limit after ranking. (ADR-0052)
const TRANSITION_WINDOW: usize = 200;
/// How strongly `P(next | anchor)` reweights a candidate above plain recency.
const TRANSITION_WEIGHT: f32 = 0.5;
/// Penalty applied to the anchor's own action so `next` predicts a step forward
/// rather than echoing what the user just did (without fully hiding it).
const ECHO_PENALTY: f32 = 0.3;
/// PRODUCT.4.A: how strongly a habitually-repeated action (high occurrence
/// frequency in the window) ranks up. Log-normalized so one very frequent action
/// cannot dominate.
const FREQUENCY_WEIGHT: f32 = 0.25;
/// PRODUCT.4.A: boost for candidates in the same workspace as the anchor — "what
/// I do in *this* project" — looser than the exact `same_context` hash match.
const WORKSPACE_WEIGHT: f32 = 0.15;
/// PRODUCT.4.B / ADR-0053: how strongly success rate reweights a candidate.
/// Applied as `(rate - 1.0) * weight`: a perfect record is the neutral baseline
/// (no bonus, so it can't saturate the score) and only failures penalize.
const SUCCESS_WEIGHT: f32 = 0.3;
/// A candidate with at least this many recorded attempts and zero successes is
/// considered broken and dropped — don't suggest a reliably-failing action.
const BROKEN_MIN_ATTEMPTS: u32 = 3;

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload)
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    let limit = workflow_memory::clamp_limit(payload.ctx.limit);

    // Pull a wider window than the display limit so the transition model sees
    // enough (prev -> next) pairs; the ranked output is truncated to `limit`.
    let rows = match deps.knowledge_store.as_ref() {
        Some(store) => {
            workflow_memory::suggest(store, req.context_hash.as_deref(), TRANSITION_WINDOW)
                .map_err(CapabilityError::ProviderError)?
        }
        None => Vec::new(),
    };

    let mut suggestions = rank_rows(
        rows,
        req.context_hash.as_deref(),
        deps.local_context.as_ref(),
    );
    suggestions.truncate(limit);
    Ok(CapabilityResponse {
        id: CapabilityId::SuggestNext,
        output: CapabilityOutput::Structured {
            value: serde_json::to_value(suggestions).unwrap_or_else(|_| Value::Array(Vec::new())),
        },
        risk_tag: RiskTag::none(),
        sources: Vec::new(),
    })
}

fn rank_rows(
    rows: Vec<workflow_memory::WorkflowHistoryRow>,
    active_context_hash: Option<&str>,
    local_context: Option<&LocalContextSearcher>,
) -> Vec<SuggestedNextAction> {
    rank_rows_at(
        rows,
        active_context_hash,
        local_context,
        current_epoch_seconds(),
    )
}

fn rank_rows_at(
    rows: Vec<workflow_memory::WorkflowHistoryRow>,
    active_context_hash: Option<&str>,
    local_context: Option<&LocalContextSearcher>,
    now: i64,
) -> Vec<SuggestedNextAction> {
    // ADR-0052: the most recent row is the "anchor" (what the user just did);
    // rank candidates by how often they historically followed it, blended with
    // recency. Built before `rows` is consumed below.
    let anchor_key = rows
        .first()
        .map(|row| transition_key(&row.route, &row.action_label));
    let transitions = build_transitions(&rows);
    let anchor_out_total: u32 = anchor_key
        .as_ref()
        .and_then(|key| transitions.get(key))
        .map(|next| next.values().sum())
        .unwrap_or(0);

    // PRODUCT.4.A: occurrence frequency per action + the anchor's workspace, both
    // derived from the existing window (no schema change).
    let frequencies = build_frequencies(&rows);
    let max_frequency = frequencies.values().copied().max().unwrap_or(1);
    let anchor_workspace = rows.first().and_then(|row| row.workspace_id);
    // PRODUCT.4.B / ADR-0053: per-action (successes, attempts) from the window.
    let success_stats = build_success_stats(&rows);

    let mut seen = HashSet::new();
    let mut suggestions = Vec::new();

    for (idx, row) in rows.into_iter().enumerate() {
        if is_suggest_next_self_row(&row)
            || is_stale(&row, now)
            || !target_resolves(&row, local_context)
        {
            continue;
        }
        let dedupe_key = transition_key(&row.route, &row.action_label);
        if !seen.insert(dedupe_key.clone()) {
            continue;
        }

        let same_context = active_context_hash
            .zip(row.context_hash.as_deref())
            .is_some_and(|(active, row_hash)| active == row_hash);

        // P(this action | anchor) from the transition model, 0 when the anchor
        // has no recorded follow-ups (cold start → pure recency fallback).
        let transition_p = if anchor_out_total > 0 {
            anchor_key
                .as_ref()
                .and_then(|key| transitions.get(key))
                .and_then(|next| next.get(&dedupe_key))
                .copied()
                .unwrap_or(0) as f32
                / anchor_out_total as f32
        } else {
            0.0
        };
        let predicted = transition_p > 0.0;
        let is_anchor = anchor_key.as_deref() == Some(dedupe_key.as_str());
        let echo = if is_anchor { ECHO_PENALTY } else { 0.0 };

        // PRODUCT.4.A signals.
        let frequency = frequencies.get(&dedupe_key).copied().unwrap_or(1);
        let frequency_norm = if max_frequency > 1 {
            (frequency as f32).ln_1p() / (max_frequency as f32).ln_1p()
        } else {
            0.0
        };
        let workspace_match =
            anchor_workspace.is_some() && row.workspace_id == anchor_workspace;
        let workspace_bonus = if workspace_match { WORKSPACE_WEIGHT } else { 0.0 };

        // PRODUCT.4.B / ADR-0053 success rate. Legacy rows (succeeded = None) were
        // recorded only on success, so they count as successes here.
        let stat = success_stats.get(&dedupe_key).copied().unwrap_or_default();
        if stat.attempts >= BROKEN_MIN_ATTEMPTS && stat.successes == 0 {
            // Reliably-failing action — don't suggest it.
            continue;
        }
        let success_rate = if stat.attempts > 0 {
            stat.successes as f32 / stat.attempts as f32
        } else {
            1.0
        };
        let success_bonus = (success_rate - 1.0) * SUCCESS_WEIGHT;

        let confidence = (score_row(&row.route, idx, same_context)
            + transition_p * TRANSITION_WEIGHT
            + frequency_norm * FREQUENCY_WEIGHT
            + workspace_bonus
            + success_bonus
            - echo)
            .clamp(0.0, 0.99);

        suggestions.push(SuggestedNextAction {
            title: row.action_label.clone(),
            subtitle: build_subtitle(&row.route, idx, same_context, predicted, frequency),
            route: row.route.clone(),
            confidence,
            rationale: build_rationale(&row.route, same_context, predicted),
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

/// PROFILE.1 / ADR-0054: base score so a project's commands get sensible
/// confidences; frequency is the dominant signal for a "toolkit" view.
const PROFILE_BASE: f32 = 0.4;
const PROFILE_FREQUENCY_WEIGHT: f32 = 0.5;

/// PROFILE.1 / ADR-0054: scope the window to the *active project*. The active
/// project is the most recent row's `project_root` (the user's current project,
/// mirroring the `next` anchor heuristic); fall back to its `workspace_id` slot,
/// else the whole window. Pure.
pub(super) fn scope_to_active_project(
    rows: Vec<workflow_memory::WorkflowHistoryRow>,
) -> Vec<workflow_memory::WorkflowHistoryRow> {
    let Some(head) = rows.first() else {
        return rows;
    };
    if let Some(root) = head.project_root.clone() {
        return rows
            .into_iter()
            .filter(|r| r.project_root.as_deref() == Some(root.as_str()))
            .collect();
    }
    if let Some(slot) = head.workspace_id {
        return rows
            .into_iter()
            .filter(|r| r.workspace_id == Some(slot))
            .collect();
    }
    rows
}

/// PROFILE.1 / ADR-0054: rank a project's signature commands by frequency ×
/// success rate (no transition/anchor/echo — this is "your toolkit here", not
/// "your next step"). Reuses the shared frequency/success helpers. Pure.
pub(super) fn rank_profile(
    rows: Vec<workflow_memory::WorkflowHistoryRow>,
    local_context: Option<&LocalContextSearcher>,
    now: i64,
) -> Vec<SuggestedNextAction> {
    let scoped = scope_to_active_project(rows);
    let frequencies = build_frequencies(&scoped);
    let max_frequency = frequencies.values().copied().max().unwrap_or(1);
    let success_stats = build_success_stats(&scoped);

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for row in scoped {
        if is_suggest_next_self_row(&row)
            || is_stale(&row, now)
            || !target_resolves(&row, local_context)
        {
            continue;
        }
        let key = transition_key(&row.route, &row.action_label);
        if !seen.insert(key.clone()) {
            continue;
        }
        let stat = success_stats.get(&key).copied().unwrap_or_default();
        if stat.attempts >= BROKEN_MIN_ATTEMPTS && stat.successes == 0 {
            continue;
        }
        let success_rate = if stat.attempts > 0 {
            stat.successes as f32 / stat.attempts as f32
        } else {
            1.0
        };
        let frequency = frequencies.get(&key).copied().unwrap_or(1);
        let frequency_norm = if max_frequency > 1 {
            (frequency as f32).ln_1p() / (max_frequency as f32).ln_1p()
        } else {
            1.0
        };
        let confidence = (PROFILE_BASE
            + frequency_norm * PROFILE_FREQUENCY_WEIGHT
            + (success_rate - 1.0) * SUCCESS_WEIGHT)
            .clamp(0.0, 0.99);
        out.push(SuggestedNextAction {
            title: row.action_label.clone(),
            subtitle: profile_subtitle(&row.route, frequency, success_rate),
            route: row.route.clone(),
            confidence,
            rationale: "frequent in this project".to_string(),
            last_executed_at: row.executed_at,
            workspace_id: row.workspace_id,
            replay: replay_for_row(&row.route, &row.action_label),
        });
    }
    out.sort_by(|left, right| {
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
    out
}

fn profile_subtitle(route: &str, frequency: u32, success_rate: f32) -> String {
    let pct = (success_rate * 100.0).round() as u32;
    format!("{route} · used {frequency}× · {pct}% ok")
}

/// Stable per-action key shared by dedupe + the transition model.
fn transition_key(route: &str, action_label: &str) -> String {
    format!("{route}::{action_label}")
}

/// Count `(prev → next)` action pairs from history. `rows` arrive newest-first,
/// so they are walked in reverse (chronological) order to form real successions.
fn build_transitions(
    rows: &[workflow_memory::WorkflowHistoryRow],
) -> HashMap<String, HashMap<String, u32>> {
    let mut map: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let chronological: Vec<&workflow_memory::WorkflowHistoryRow> = rows.iter().rev().collect();
    for pair in chronological.windows(2) {
        let prev = transition_key(&pair[0].route, &pair[0].action_label);
        let next = transition_key(&pair[1].route, &pair[1].action_label);
        *map.entry(prev).or_default().entry(next).or_insert(0) += 1;
    }
    map
}

/// PRODUCT.4.A: occurrence count per `(route, action_label)` across the window —
/// the "how habitual is this action" signal.
fn build_frequencies(rows: &[workflow_memory::WorkflowHistoryRow]) -> HashMap<String, u32> {
    let mut map: HashMap<String, u32> = HashMap::new();
    for row in rows {
        *map.entry(transition_key(&row.route, &row.action_label))
            .or_insert(0) += 1;
    }
    map
}

#[derive(Debug, Clone, Copy, Default)]
struct SuccessStat {
    successes: u32,
    attempts: u32,
}

/// PRODUCT.4.B / ADR-0053: per-action (successes, attempts) over the window. A
/// `None` outcome (legacy row) counts as a success, since pre-ADR recording only
/// happened on success.
fn build_success_stats(
    rows: &[workflow_memory::WorkflowHistoryRow],
) -> HashMap<String, SuccessStat> {
    let mut map: HashMap<String, SuccessStat> = HashMap::new();
    for row in rows {
        let stat = map
            .entry(transition_key(&row.route, &row.action_label))
            .or_default();
        stat.attempts += 1;
        if row.succeeded != Some(false) {
            stat.successes += 1;
        }
    }
    map
}

fn current_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64)
}

fn is_stale(row: &workflow_memory::WorkflowHistoryRow, now: i64) -> bool {
    let age = now.saturating_sub(row.executed_at);
    if age > MAX_SUGGESTION_AGE_SECS {
        return true;
    }
    replay_for_row(&row.route, &row.action_label).is_none() && age > HISTORY_ONLY_MAX_AGE_SECS
}

fn target_resolves(
    row: &workflow_memory::WorkflowHistoryRow,
    local_context: Option<&LocalContextSearcher>,
) -> bool {
    match row.route.as_str() {
        "cmd.run" => {
            let Some(name) = command_name(&row.action_label) else {
                return false;
            };
            let Some(local_context) = local_context else {
                return true;
            };
            let Ok(registry) = local_context.builtin_registry.lock() else {
                return false;
            };
            registry.list().iter().any(|meta| meta.name == name)
        }
        "capability.call" => row
            .action_label
            .split_whitespace()
            .next()
            .and_then(CapabilityId::parse)
            .is_some(),
        "action.run" => !row.action_label.trim().is_empty(),
        _ => false,
    }
}

fn is_suggest_next_self_row(row: &workflow_memory::WorkflowHistoryRow) -> bool {
    row.route == "capability.call" && row.action_label.starts_with("suggest_next")
}

/// Recency + route + context base. PRODUCT.4.A rescales recency to ~half its old
/// magnitude (and drops the early clamp) so the transition / frequency /
/// workspace signals are not drowned out by recency saturating at the top; the
/// final blend is clamped once by the caller.
fn score_row(route: &str, recency_index: usize, same_context: bool) -> f32 {
    let recency = (0.5 - (recency_index as f32 * 0.06)).max(0.1);
    let route_bonus = match route {
        "cmd.run" => 0.18,
        "action.run" => 0.12,
        "capability.call" => 0.08,
        _ => 0.04,
    };
    let context_bonus = if same_context { 0.1 } else { 0.0 };
    recency + route_bonus + context_bonus
}

fn build_subtitle(
    route: &str,
    recency_index: usize,
    same_context: bool,
    predicted: bool,
    frequency: u32,
) -> String {
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
    // PRODUCT.4.A: surface habit strength so the ranking stays explainable.
    let used = if frequency > 1 {
        format!(" · used {frequency}×")
    } else {
        String::new()
    };
    if predicted {
        format!("likely next · {route} · {scope}{used}")
    } else {
        format!("{route} · {freshness} · {scope}{used}")
    }
}

fn build_rationale(route: &str, same_context: bool, predicted: bool) -> String {
    if predicted {
        let base = "usually follows your last action";
        return if same_context {
            format!("{base} in this workspace")
        } else {
            base.to_string()
        };
    }
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

    let name = command_name(title)?;
    let trimmed = title.trim().strip_prefix('/')?;
    let mut parts = trimmed.splitn(2, ' ');
    let _ = parts.next();
    let args = parts.next().unwrap_or("").trim();
    Some(ReplayActionDescriptor {
        route: route.to_string(),
        payload: json!({
            "name": name,
            "args": args,
        }),
    })
}

fn command_name(title: &str) -> Option<&str> {
    let trimmed = title.trim().strip_prefix('/')?;
    let name = trimmed.split_whitespace().next()?.trim();
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
    use crate::core::ai_capability::test_fixtures::{
        assert_invalid_payload, assert_safe_primary_output,
    };
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

    fn deps_without_store() -> CapabilityDeps {
        CapabilityDeps {
            chat: Arc::new(UnusedProvider),
            local_context: None,
            knowledge_store: None,
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
                succeeded: None,
                project_root: None,
            });
            store.try_log_workflow_history(WorkflowHistoryEntry {
                context_hash: Some("ctx-a".into()),
                route: "capability.call".into(),
                action_label: "explain rust ownership".into(),
                payload_digest: None,
                workspace_id: Some(7),
                succeeded: None,
                project_root: None,
            });

            let resp = call(
                req(serde_json::json!({ "ctx": { "limit": 5 } }), Some("ctx-a")),
                &deps_with_store(store),
            )
            .unwrap();
            assert_eq!(resp.id, CapabilityId::SuggestNext);
            assert!(!resp.risk_tag.requires_confirmation);
            assert_safe_primary_output(&resp);
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
    fn rejects_malformed_payload_as_typed_error() {
        let err = call(
            req(serde_json::json!({ "ctx": { "limit": "many" } }), None),
            &deps_without_store(),
        )
        .unwrap_err();
        assert_invalid_payload(err);
    }

    fn cmd_row(id: i64, label: &str, executed_at: i64) -> workflow_memory::WorkflowHistoryRow {
        workflow_memory::WorkflowHistoryRow {
            id,
            context_hash: Some("ctx".into()),
            route: "cmd.run".into(),
            action_label: label.into(),
            payload_digest: None,
            workspace_id: Some(1),
            succeeded: None,
            project_root: None,
            executed_at,
        }
    }

    #[test]
    fn predicts_action_that_historically_followed_the_anchor() {
        // History (newest-first): the anchor `/build` was repeatedly followed by
        // `/test`. Even though `/build` is the most recent (highest recency),
        // `/test` should rank first as the predicted next step. (ADR-0052)
        let rows = vec![
            cmd_row(5, "/build", 105),
            cmd_row(4, "/test", 104),
            cmd_row(3, "/build", 103),
            cmd_row(2, "/test", 102),
            cmd_row(1, "/build", 101),
        ];
        let items = rank_rows_at(rows, Some("ctx"), None, 200);
        assert_eq!(items.len(), 2, "two distinct actions");
        assert_eq!(items[0].title, "/test", "predicted next ranks above the echo");
        assert!(items[0].subtitle.contains("likely next"));
        assert!(items[0].confidence > items[1].confidence);
    }

    #[test]
    fn falls_back_to_recency_when_anchor_has_no_followups() {
        // Anchor `/build` (newest) never had a recorded successor, so ranking
        // degrades to the existing recency/route ordering: the anchor stays on
        // top, no "likely next" label.
        let rows = vec![cmd_row(2, "/build", 102), cmd_row(1, "/deploy", 101)];
        let items = rank_rows_at(rows, Some("ctx"), None, 200);
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| !i.subtitle.contains("likely next")));
    }

    fn action_row(
        id: i64,
        label: &str,
        executed_at: i64,
        workspace_id: i64,
    ) -> workflow_memory::WorkflowHistoryRow {
        workflow_memory::WorkflowHistoryRow {
            id,
            context_hash: Some("ctx".into()),
            route: "action.run".into(),
            action_label: label.into(),
            payload_digest: None,
            workspace_id: Some(workspace_id),
            succeeded: None,
            project_root: None,
            executed_at,
        }
    }

    #[test]
    fn frequency_lifts_a_habitual_action_above_a_more_recent_rare_one() {
        // `/common` is older than `/rare` but used 3×; PRODUCT.4.A frequency
        // should rank it first. (context filtered out via active=None so only
        // recency + frequency + workspace decide.)
        let rows = vec![
            action_row(6, "/anchor", 106, 1),
            action_row(5, "/rare", 105, 1),
            action_row(4, "/common", 104, 1),
            action_row(3, "/common", 103, 1),
            action_row(2, "/common", 102, 1),
        ];
        let items = rank_rows_at(rows, None, None, 200);
        assert_eq!(items[0].title, "/common", "habitual action ranks first");
        assert!(items[0].subtitle.contains("used 3×"));
        let rare = items.iter().find(|i| i.title == "/rare").expect("rare present");
        assert!(items[0].confidence > rare.confidence);
    }

    fn proj_row(
        id: i64,
        label: &str,
        executed_at: i64,
        project: &str,
    ) -> workflow_memory::WorkflowHistoryRow {
        workflow_memory::WorkflowHistoryRow {
            id,
            context_hash: Some("ctx".into()),
            route: "cmd.run".into(),
            action_label: label.into(),
            payload_digest: None,
            workspace_id: Some(1),
            succeeded: None,
            project_root: Some(project.into()),
            executed_at,
        }
    }

    #[test]
    fn profile_scopes_to_active_project_and_ranks_by_frequency() {
        // Head row is in /projA; the profile must only contain /projA commands
        // (excluding /projB) and rank the more frequent one first. (ADR-0054)
        let rows = vec![
            proj_row(6, "/a", 106, "/projA"),
            proj_row(5, "/b", 105, "/projA"),
            proj_row(4, "/a", 104, "/projA"),
            proj_row(3, "/a", 103, "/projA"),
            proj_row(2, "/c", 102, "/projB"),
        ];
        let items = rank_profile(rows, None, 200);
        assert_eq!(items.len(), 2, "only /projA commands");
        assert!(!items.iter().any(|i| i.title == "/c"), "other project excluded");
        assert_eq!(items[0].title, "/a", "most-frequent command first");
        assert!(items[0].subtitle.contains("used 3×"));
    }

    #[test]
    fn profile_falls_back_to_slot_when_no_project_root() {
        // No project_root → scope by the head row's workspace_id (slot 1),
        // excluding slot 2.
        let rows = vec![
            action_row(3, "/a", 103, 1),
            action_row(2, "/a", 102, 1),
            action_row(1, "/other", 101, 2),
        ];
        let items = rank_profile(rows, None, 200);
        assert!(items.iter().any(|i| i.title == "/a"));
        assert!(!items.iter().any(|i| i.title == "/other"), "other slot excluded");
    }

    #[test]
    fn same_workspace_action_outranks_other_workspace() {
        // Equal frequency/recency-class actions: the one in the anchor's
        // workspace (9) should outrank the one in another workspace (5).
        let rows = vec![
            action_row(3, "/anchor", 103, 9),
            action_row(2, "/same", 102, 9),
            action_row(1, "/other", 101, 5),
        ];
        let items = rank_rows_at(rows, None, None, 200);
        assert_eq!(items[0].title, "/same", "same-workspace action ranks first");
        let same = items.iter().find(|i| i.title == "/same").unwrap();
        let other = items.iter().find(|i| i.title == "/other").unwrap();
        assert!(same.confidence > other.confidence);
    }

    fn outcome_row(
        id: i64,
        label: &str,
        executed_at: i64,
        succeeded: bool,
    ) -> workflow_memory::WorkflowHistoryRow {
        workflow_memory::WorkflowHistoryRow {
            id,
            context_hash: Some("ctx".into()),
            route: "action.run".into(),
            action_label: label.into(),
            payload_digest: None,
            workspace_id: Some(1),
            succeeded: Some(succeeded),
            project_root: None,
            executed_at,
        }
    }

    #[test]
    fn higher_success_rate_outranks_a_more_recent_flaky_action() {
        // `/good` (2/2 success) should beat `/bad` (1/2) even though `/bad` is
        // more recent and they share frequency/workspace. (ADR-0053)
        let rows = vec![
            outcome_row(5, "/anchor", 105, true),
            outcome_row(4, "/bad", 104, true),
            outcome_row(3, "/bad", 103, false),
            outcome_row(2, "/good", 102, true),
            outcome_row(1, "/good", 101, true),
        ];
        let items = rank_rows_at(rows, None, None, 200);
        assert_eq!(items[0].title, "/good", "reliable action ranks first");
        let good = items.iter().find(|i| i.title == "/good").unwrap();
        let bad = items.iter().find(|i| i.title == "/bad").unwrap();
        assert!(good.confidence > bad.confidence);
    }

    #[test]
    fn drops_a_reliably_failing_action() {
        // `/broken`: 3 attempts, 0 successes → never suggested.
        let rows = vec![
            outcome_row(4, "/anchor", 104, true),
            outcome_row(3, "/broken", 103, false),
            outcome_row(2, "/broken", 102, false),
            outcome_row(1, "/broken", 101, false),
        ];
        let items = rank_rows_at(rows, None, None, 200);
        assert!(
            !items.iter().any(|i| i.title == "/broken"),
            "reliably-failing action is dropped"
        );
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
                succeeded: None,
                project_root: None,
                executed_at: 20,
            },
            workflow_memory::WorkflowHistoryRow {
                id: 1,
                context_hash: Some("ctx".into()),
                route: "cmd.run".into(),
                action_label: "/help".into(),
                payload_digest: None,
                workspace_id: Some(1),
                succeeded: None,
                project_root: None,
                executed_at: 10,
            },
        ];
        let items = rank_rows_at(rows, Some("ctx"), None, 30);
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
                succeeded: None,
                project_root: None,
                executed_at: 20,
            },
            workflow_memory::WorkflowHistoryRow {
                id: 1,
                context_hash: Some("ctx".into()),
                route: "cmd.run".into(),
                action_label: "/help".into(),
                payload_digest: None,
                workspace_id: Some(1),
                succeeded: None,
                project_root: None,
                executed_at: 10,
            },
        ];
        let items = rank_rows_at(rows, Some("ctx"), None, 30);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "/help");
    }

    #[test]
    fn drops_very_old_replayable_rows() {
        let now = 4_000_000;
        let rows = vec![workflow_memory::WorkflowHistoryRow {
            id: 1,
            context_hash: Some("ctx".into()),
            route: "cmd.run".into(),
            action_label: "/help".into(),
            payload_digest: None,
            workspace_id: Some(1),
            succeeded: None,
            project_root: None,
            executed_at: now - MAX_SUGGESTION_AGE_SECS - 1,
        }];
        assert!(rank_rows_at(rows, Some("ctx"), None, now).is_empty());
    }

    #[test]
    fn ages_history_only_rows_out_sooner() {
        let now = 4_000_000;
        let rows = vec![workflow_memory::WorkflowHistoryRow {
            id: 1,
            context_hash: Some("ctx".into()),
            route: "action.run".into(),
            action_label: "Open main.rs".into(),
            payload_digest: None,
            workspace_id: Some(1),
            succeeded: None,
            project_root: None,
            executed_at: now - HISTORY_ONLY_MAX_AGE_SECS - 1,
        }];
        assert!(rank_rows_at(rows, Some("ctx"), None, now).is_empty());
    }

    #[test]
    fn drops_rows_without_a_resolvable_route_target() {
        let now = 100;
        let rows = vec![
            workflow_memory::WorkflowHistoryRow {
                id: 1,
                context_hash: Some("ctx".into()),
                route: "cmd.run".into(),
                action_label: "missing slash".into(),
                payload_digest: None,
                workspace_id: Some(1),
                succeeded: None,
                project_root: None,
                executed_at: now,
            },
            workflow_memory::WorkflowHistoryRow {
                id: 2,
                context_hash: Some("ctx".into()),
                route: "capability.call".into(),
                action_label: "removed_capability old input".into(),
                payload_digest: None,
                workspace_id: Some(1),
                succeeded: None,
                project_root: None,
                executed_at: now,
            },
        ];
        assert!(rank_rows_at(rows, Some("ctx"), None, now).is_empty());
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
