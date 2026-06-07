//! Capability: recall stored personal memories matching a query.
//!
//! No LLM call — this is a local `agent_memories` read plus lightweight term
//! ranking, mirroring `suggest_next`. The store already returns recent-first,
//! so an empty query yields the most recent memories.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::ai_capability::capabilities::remember::PERSONAL_MEMORY_SCOPE;
use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::memory::term_score;
use crate::core::ai_capability::registry::CapabilityId;
use crate::models::unified_result::RiskTag;

/// Upper bound on rows pulled from the store before in-memory ranking. Personal
/// memory is small; a flat scan keeps the read path migration-free.
const SCAN_LIMIT: usize = 200;
const DEFAULT_RESULT_LIMIT: usize = 8;
const SNIPPET_CHARS: usize = 140;

#[derive(Debug, Default, Deserialize)]
struct Payload {
    #[serde(default)]
    query: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecalledMemory {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub content: String,
    pub score: f32,
}

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload)
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    let query = payload.query.trim().to_lowercase();
    let limit = payload.limit.unwrap_or(DEFAULT_RESULT_LIMIT).clamp(1, 50);

    let rows = match deps.knowledge_store.as_ref() {
        Some(store) => store
            .agent_memories_blocking(Some(PERSONAL_MEMORY_SCOPE.to_string()), None, SCAN_LIMIT)
            .map_err(CapabilityError::ProviderError)?,
        None => Vec::new(),
    };

    let memories = rank(rows, &query, limit);
    Ok(CapabilityResponse {
        id: CapabilityId::Recall,
        risk_tag: RiskTag::none(),
        output: CapabilityOutput::Structured {
            value: serde_json::to_value(memories).unwrap_or_else(|_| Value::Array(Vec::new())),
        },
        sources: Vec::new(),
    })
}

fn rank(
    rows: Vec<crate::core::knowledge_store::AgentMemoryEntry>,
    query: &str,
    limit: usize,
) -> Vec<RecalledMemory> {
    let mut scored: Vec<RecalledMemory> = rows
        .into_iter()
        .map(|row| {
            let score = term_score(query, &row.title, &row.content);
            RecalledMemory {
                id: row.id,
                snippet: snippet_of(&row.content),
                title: row.title,
                content: row.content,
                score,
            }
        })
        .collect();

    if !query.is_empty() {
        scored.retain(|m| m.score > 0.0);
        // Stable sort by score keeps recency order (store returns recent-first)
        // among equally-scored memories.
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    scored.truncate(limit);
    scored
}

fn snippet_of(content: &str) -> String {
    let flat = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > SNIPPET_CHARS {
        let head: String = flat.chars().take(SNIPPET_CHARS).collect();
        format!("{head}…")
    } else {
        flat
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
    use crate::core::knowledge_store::{AgentMemoryEntry, KnowledgeStoreHandle};
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

    fn deps(store: KnowledgeStoreHandle) -> CapabilityDeps {
        CapabilityDeps {
            chat: Arc::new(UnusedProvider),
            local_context: None,
            knowledge_store: Some(store),
            cancel: Arc::new(AtomicBool::new(false)),
            stream_chunk: None,
            allow_memory_grounding: false,
        }
    }

    fn req(payload: serde_json::Value) -> CapabilityRequest {
        CapabilityRequest {
            id: CapabilityId::Recall,
            payload,
            context_hash: None,
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
    fn ranks_query_matches_above_non_matches() {
        let path = temp_db_path("recall");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            store_memory(&store, "Coffee order", "oat flat white, no sugar");
            store_memory(&store, "Dog", "my dog Mochi likes walks");

            let resp = call(req(serde_json::json!({ "query": "coffee" })), &deps(store)).unwrap();
            let items: Vec<RecalledMemory> = match resp.output {
                CapabilityOutput::Structured { value } => serde_json::from_value(value).unwrap(),
                _ => panic!("expected structured output"),
            };
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].title, "Coffee order");
            assert!(items[0].score > 0.0);
        }
        cleanup(&path);
    }

    #[test]
    fn empty_query_returns_recent_memories() {
        let path = temp_db_path("recall-empty");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            store_memory(&store, "A", "first");
            store_memory(&store, "B", "second");

            let resp = call(req(serde_json::json!({ "query": "" })), &deps(store)).unwrap();
            let items: Vec<RecalledMemory> = match resp.output {
                CapabilityOutput::Structured { value } => serde_json::from_value(value).unwrap(),
                _ => panic!("expected structured output"),
            };
            assert_eq!(items.len(), 2);
        }
        cleanup(&path);
    }
}
