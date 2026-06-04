//! Personal-memory grounding for capability prompts (MEM.1.B, ADR-0043).
//!
//! Reads `scope="personal"` rows from the knowledge store and pushes the most
//! relevant few as grounding sources. This is invoked by `explain` / `fix_error`
//! / `gen_command` only when `CapabilityDeps.allow_memory_grounding` is true,
//! which the handler sets only for a local provider — so personal memory never
//! leaves the device via a cloud prompt. `visibility_filtered_source` still
//! redacts secret-shaped content as a second line of defence.

use crate::core::ai_capability::capabilities::remember::PERSONAL_MEMORY_SCOPE;
use crate::core::grounding::visibility_filtered_source;
use crate::core::knowledge_store::KnowledgeStoreHandle;
use crate::models::agent::GroundingSource;

const MEMORY_SCAN_LIMIT: usize = 50;
const MEMORY_GROUNDING_LIMIT: usize = 3;

/// Lightweight term overlap score shared by `recall` and memory grounding.
/// Title hits weigh double; an empty query scores every memory equally so the
/// caller falls back to recency order.
pub(crate) fn term_score(query: &str, title: &str, content: &str) -> f32 {
    if query.is_empty() {
        return 1.0;
    }
    let title_l = title.to_lowercase();
    let content_l = content.to_lowercase();
    let mut score = 0.0f32;
    for term in query.split_whitespace() {
        if title_l.contains(term) {
            score += 2.0;
        }
        if content_l.contains(term) {
            score += 1.0;
        }
    }
    score
}

/// Push up to `MEMORY_GROUNDING_LIMIT` personal memories matching `query` as
/// grounding sources. Best-effort: store errors are swallowed (grounding is
/// optional and must never block a capability).
pub(crate) fn push_memory_sources(
    store: &KnowledgeStoreHandle,
    query: &str,
    sources: &mut Vec<GroundingSource>,
) {
    let Ok(rows) = store.agent_memories_blocking(
        Some(PERSONAL_MEMORY_SCOPE.to_string()),
        None,
        MEMORY_SCAN_LIMIT,
    ) else {
        return;
    };

    let q = query.trim().to_lowercase();
    let mut scored: Vec<(f32, crate::core::knowledge_store::AgentMemoryEntry)> = rows
        .into_iter()
        .map(|row| (term_score(&q, &row.title, &row.content), row))
        .collect();
    if !q.is_empty() {
        scored.retain(|(s, _)| *s > 0.0);
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    }

    for (_, row) in scored.into_iter().take(MEMORY_GROUNDING_LIMIT) {
        sources.push(visibility_filtered_source(
            format!("memory:{}", row.id),
            "memory",
            row.title,
            row.content.replace('\n', " "),
            0.9,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::knowledge_store::{AgentMemoryEntry, KnowledgeStoreHandle};
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

    fn store_memory(store: &KnowledgeStoreHandle, title: &str, content: &str) {
        store.try_store_agent_memory(AgentMemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            scope: PERSONAL_MEMORY_SCOPE.to_string(),
            workspace_id: None,
            title: title.to_string(),
            content: content.to_string(),
            visibility: "private".to_string(),
        });
    }

    #[test]
    fn pushes_matching_memory_as_source() {
        let path = temp_db_path("mem-ground");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            store_memory(&store, "Coffee order", "oat flat white, no sugar");
            store_memory(&store, "Dog", "my dog Mochi");

            let mut sources = Vec::new();
            push_memory_sources(&store, "coffee", &mut sources);
            assert_eq!(sources.len(), 1);
            assert_eq!(sources[0].source_type, "memory");
            assert!(sources[0].snippet.contains("oat flat white"));
        }
        cleanup(&path);
    }

    #[test]
    fn redacts_secret_shaped_memory() {
        let path = temp_db_path("mem-secret");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            store_memory(&store, "API token", "my api_key is sk-abc123");

            let mut sources = Vec::new();
            push_memory_sources(&store, "token", &mut sources);
            assert_eq!(sources.len(), 1);
            assert!(!sources[0].snippet.contains("sk-abc123"));
            assert_eq!(sources[0].redacted_reason.as_deref(), Some("secret"));
        }
        cleanup(&path);
    }
}
