//! REF.5 — workflow memory: record + suggest over schema v4
//! `workflow_history`.
//!
//! Thin pure-logic layer over [`KnowledgeStoreHandle`]. Recording is
//! fire-and-forget; suggestion is a blocking read with the SQLite actor's
//! built-in 2-second timeout.
//!
//! ADR-0029 §4: workflow_history is additive; this module only consumes
//! the schema, the migration itself is owned by `core/knowledge_store.rs`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde_json::Value;

use crate::core::knowledge_store::KnowledgeStoreHandle;
pub use crate::core::knowledge_store::{WorkflowHistoryEntry, WorkflowHistoryRow};

/// `suggest` / `recent` limit caps to keep IPC + render paths bounded.
pub const DEFAULT_LIMIT: usize = 5;
pub const MAX_LIMIT: usize = 50;

/// Compute the coarse REF.5 context hash. Stays stable across query
/// changes — the workspace identity + mode + active panel together form
/// the "what is the user up to" signal; the actual query / typed payload
/// belongs in per-call payload_digest if matching is ever needed.
///
/// Uses [`DefaultHasher`] for determinism within a process run; the hash
/// is opaque to consumers and only needs to be equal for equal inputs.
pub fn compute_context_hash(workspace_id: i64, mode: &str, panel: Option<&str>) -> String {
    let mut h = DefaultHasher::new();
    workspace_id.hash(&mut h);
    mode.hash(&mut h);
    panel.unwrap_or("").hash(&mut h);
    format!("{:x}", h.finish())
}

/// Short deterministic digest of an arbitrary payload. Stored alongside
/// each record for future repeat-detection in v2 ranking; v1 only writes
/// it and does not consume it.
pub fn digest_payload(payload: &Value) -> String {
    // Serialize in compact canonical form so reorderings of equivalent
    // JSON objects produce the same digest input. `serde_json::to_string`
    // walks fields in BTreeMap order for objects internally — sufficient
    // for the determinism we need this batch.
    let serialized = serde_json::to_string(payload).unwrap_or_default();
    let mut h = DefaultHasher::new();
    serialized.hash(&mut h);
    format!("{:x}", h.finish())
}

/// Clamp a caller-supplied limit into the supported range.
pub fn clamp_limit(requested: Option<usize>) -> usize {
    requested.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}

/// Record a workflow-history row. Best-effort; failures are absorbed by
/// the knowledge-store actor's dropped-log counter.
pub fn record(store: &KnowledgeStoreHandle, entry: WorkflowHistoryEntry) {
    store.try_log_workflow_history(entry);
}

/// Read recent workflow rows for the active context. When `context_hash`
/// is `Some`, filters to matching rows; when `None`, returns the global
/// recency tail.
pub fn suggest(
    store: &KnowledgeStoreHandle,
    context_hash: Option<&str>,
    limit: usize,
) -> Result<Vec<WorkflowHistoryRow>, String> {
    store.recent_workflows_blocking(context_hash.map(str::to_owned), limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

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

    fn make_entry(ctx: Option<&str>, route: &str, label: &str) -> WorkflowHistoryEntry {
        WorkflowHistoryEntry {
            context_hash: ctx.map(str::to_owned),
            route: route.into(),
            action_label: label.into(),
            payload_digest: None,
            workspace_id: Some(0),
            succeeded: None,
            project_root: None,
        }
    }

    #[test]
    fn compute_context_hash_is_deterministic() {
        let a = compute_context_hash(1, "search", Some("notes"));
        let b = compute_context_hash(1, "search", Some("notes"));
        assert_eq!(a, b);
    }

    #[test]
    fn compute_context_hash_differs_per_input() {
        let base = compute_context_hash(1, "search", Some("notes"));
        assert_ne!(base, compute_context_hash(2, "search", Some("notes")));
        assert_ne!(base, compute_context_hash(1, "command", Some("notes")));
        assert_ne!(base, compute_context_hash(1, "search", Some("settings")));
        assert_ne!(base, compute_context_hash(1, "search", None));
    }

    #[test]
    fn digest_payload_is_stable_across_reserialization() {
        let v = serde_json::json!({ "id": "explain", "text": "hello" });
        let a = digest_payload(&v);
        // Round-trip via string and back; the digest must still match.
        let serialized = serde_json::to_string(&v).unwrap();
        let reparsed: Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(a, digest_payload(&reparsed));
    }

    #[test]
    fn clamp_limit_uses_default_when_none() {
        assert_eq!(clamp_limit(None), DEFAULT_LIMIT);
    }

    #[test]
    fn clamp_limit_clamps_to_max() {
        assert_eq!(clamp_limit(Some(usize::MAX)), MAX_LIMIT);
    }

    #[test]
    fn clamp_limit_clamps_below_one_to_one() {
        assert_eq!(clamp_limit(Some(0)), 1);
    }

    #[test]
    fn record_then_suggest_roundtrip_filters_by_context_hash() {
        let path = temp_db_path("workflow-roundtrip");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            record(&store, make_entry(Some("ctx-a"), "cmd.run", "help"));
            record(
                &store,
                make_entry(Some("ctx-a"), "capability.call", "explain"),
            );
            record(&store, make_entry(Some("ctx-b"), "cmd.run", "setting"));

            // Allow the worker to drain by issuing a synchronous read; the
            // read itself blocks until the worker reaches it.
            let in_a = suggest(&store, Some("ctx-a"), 10).expect("read ctx-a");
            assert_eq!(in_a.len(), 2);
            let labels: Vec<_> = in_a.iter().map(|r| r.action_label.as_str()).collect();
            assert!(labels.contains(&"help"));
            assert!(labels.contains(&"explain"));

            let in_b = suggest(&store, Some("ctx-b"), 10).expect("read ctx-b");
            assert_eq!(in_b.len(), 1);
            assert_eq!(in_b[0].action_label, "setting");

            // No filter returns everything.
            let all = suggest(&store, None, 10).expect("read all");
            assert_eq!(all.len(), 3);
        }
        cleanup(&path);
    }

    #[test]
    fn suggest_orders_by_recency_desc() {
        let path = temp_db_path("workflow-recency");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            record(&store, make_entry(Some("ctx"), "cmd.run", "first"));
            // Force serialization ordering at the worker via an interleaved read
            // before queueing the next write. `executed_at` is server-stamped
            // at insert time, so an in-between blocking read pins the prior
            // write's timestamp.
            let _ = suggest(&store, Some("ctx"), 1);
            std::thread::sleep(std::time::Duration::from_secs(1));
            record(&store, make_entry(Some("ctx"), "cmd.run", "second"));

            let rows = suggest(&store, Some("ctx"), 10).expect("read");
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].action_label, "second", "newest first");
            assert_eq!(rows[1].action_label, "first");
        }
        cleanup(&path);
    }
}
