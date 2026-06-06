//! Capability: explain a focused result row, command, or text snippet using
//! local context (workspace metadata, recent notes/history) for grounding.
//!
//! Single-shot. `audit = true`, `requires_confirmation = false`.

use std::sync::atomic::Ordering;

use serde::Deserialize;

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
    CapabilitySource,
};
use crate::core::ai_capability::memory::push_memory_sources;
use crate::core::ai_capability::prompt::{build_prompt_with_sources, maybe_audit};
use crate::core::ai_capability::registry::{meta, CapabilityId};
use crate::models::agent::GroundingSource;
use crate::models::unified_result::RiskTag;

#[derive(Debug, Deserialize)]
struct Payload {
    /// What to explain. Free-form: a search-result title, a code snippet,
    /// a command name, etc.
    text: String,
    /// Optional focused question. When absent, the model is asked to
    /// "explain succinctly".
    #[serde(default)]
    question: Option<String>,
}

const SYSTEM: &str = "You are a senior developer's explainer assistant integrated into Keynova, a \
     keyboard-first launcher. Explain things directly and concretely. Prefer three short \
     paragraphs over headings. Never invent file paths or APIs that are not in the context.";

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload.clone())
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    if payload.text.trim().is_empty() {
        return Err(CapabilityError::InvalidPayload("text is empty".into()));
    }
    if deps.cancel.load(Ordering::SeqCst) {
        return Err(CapabilityError::Cancelled);
    }

    // Local grounding is best-effort — failures must not block an explanation.
    let mut sources: Vec<GroundingSource> = Vec::new();
    if let Some(lc) = deps.local_context.as_ref() {
        lc.push_workspace_source(&mut sources);
        let q = payload.text.to_lowercase();
        let _ = lc.push_command_sources(&q, &mut sources);
        let _ = lc.push_note_sources(&q, &mut sources);
        let _ = lc.push_history_sources(&q, &mut sources);
    }
    // Personal-memory grounding is gated to local providers (ADR-0043); the
    // handler sets `allow_memory_grounding` only for ollama.
    if deps.allow_memory_grounding {
        if let Some(store) = deps.knowledge_store.as_ref() {
            push_memory_sources(store, &payload.text, &mut sources);
        }
    }

    let task = match &payload.question {
        Some(q) if !q.trim().is_empty() => format!(
            "Explain: {}\n\nUser's focused question: {}",
            payload.text, q
        ),
        _ => format!("Explain succinctly: {}", payload.text),
    };
    let built_prompt = build_prompt_with_sources(SYSTEM, &sources, &task);

    if deps.cancel.load(Ordering::SeqCst) {
        return Err(CapabilityError::Cancelled);
    }

    let reply = match &deps.stream_chunk {
        Some(on_chunk) => {
            deps.chat
                .chat_stream(&built_prompt.text, on_chunk.as_ref(), &deps.cancel)
        }
        None => deps.chat.chat(&built_prompt.text, &deps.cancel),
    };

    let audit = meta(CapabilityId::Explain).audit;
    match reply {
        Ok(text) if text.trim().is_empty() => {
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::Explain.as_str(),
                "error",
                "provider returned an empty response",
                None,
            );
            Err(CapabilityError::ProviderError(
                "provider returned an empty response".into(),
            ))
        }
        Ok(text) => {
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::Explain.as_str(),
                "ok",
                "capability:explain completed",
                None,
            );
            Ok(CapabilityResponse {
                id: CapabilityId::Explain,
                output: CapabilityOutput::Text { text },
                risk_tag: RiskTag::none(),
                sources: CapabilitySource::from_grounding_sources(&built_prompt.included_sources),
            })
        }
        Err(e) => {
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::Explain.as_str(),
                "error",
                &e,
                None,
            );
            Err(CapabilityError::ProviderError(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
    use crate::core::ai_capability::test_fixtures::{
        assert_invalid_payload, assert_safe_primary_output, TYPESCRIPT_STACK_TRACE,
    };
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct EchoProvider;
    impl ChatProvider for EchoProvider {
        fn chat(&self, prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok(format!(
                "explanation of: {}",
                prompt.lines().last().unwrap_or("")
            ))
        }
    }
    struct ErrProvider;
    impl ChatProvider for ErrProvider {
        fn chat(&self, _prompt: &str, _c: &AtomicBool) -> Result<String, String> {
            Err("network down".into())
        }
    }
    struct EmptyProvider;
    impl ChatProvider for EmptyProvider {
        fn chat(&self, _prompt: &str, _c: &AtomicBool) -> Result<String, String> {
            Ok("   ".into())
        }
    }

    fn deps(chat: Arc<dyn ChatProvider>) -> CapabilityDeps {
        CapabilityDeps {
            chat,
            local_context: None,
            knowledge_store: None,
            cancel: Arc::new(AtomicBool::new(false)),
            stream_chunk: None,
            allow_memory_grounding: false,
        }
    }
    fn req(payload: serde_json::Value) -> CapabilityRequest {
        CapabilityRequest {
            id: CapabilityId::Explain,
            payload,
            context_hash: None,
        }
    }

    #[test]
    fn rejects_empty_text() {
        let err = call(
            req(serde_json::json!({ "text": "" })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap_err();
        assert_invalid_payload(err);
    }

    #[test]
    fn rejects_malformed_payload_as_typed_error() {
        let err = call(
            req(serde_json::json!({ "text": 42 })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap_err();
        assert_invalid_payload(err);
    }

    #[test]
    fn happy_path_returns_text_with_no_confirm() {
        let resp = call(
            req(serde_json::json!({ "text": TYPESCRIPT_STACK_TRACE, "question": "root cause?" })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap();
        assert!(!resp.risk_tag.requires_confirmation);
        assert_eq!(resp.id, CapabilityId::Explain);
        assert_safe_primary_output(&resp);
    }

    #[test]
    fn provider_error_propagates() {
        let err = call(
            req(serde_json::json!({ "text": "x" })),
            &deps(Arc::new(ErrProvider)),
        )
        .unwrap_err();
        assert!(matches!(err, CapabilityError::ProviderError(_)));
    }

    #[test]
    fn empty_provider_reply_is_typed_error() {
        let err = call(
            req(serde_json::json!({ "text": "x" })),
            &deps(Arc::new(EmptyProvider)),
        )
        .unwrap_err();
        assert!(matches!(err, CapabilityError::ProviderError(_)));
    }

    // ── memory grounding gate (ADR-0043, MEM.1.B) ──────────────────────────

    use crate::core::ai_capability::capabilities::remember::PERSONAL_MEMORY_SCOPE;
    use crate::core::knowledge_store::{AgentMemoryEntry, KnowledgeStoreHandle};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    /// Records the last prompt it was asked to complete so tests can assert
    /// what context the capability injected.
    struct CapturingProvider {
        last_prompt: Arc<Mutex<String>>,
    }
    impl ChatProvider for CapturingProvider {
        fn chat(&self, prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            *self.last_prompt.lock().unwrap() = prompt.to_string();
            Ok("ok".into())
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

    fn deps_with_memory(
        chat: Arc<dyn ChatProvider>,
        store: KnowledgeStoreHandle,
        allow: bool,
    ) -> CapabilityDeps {
        CapabilityDeps {
            chat,
            local_context: None,
            knowledge_store: Some(store),
            cancel: Arc::new(AtomicBool::new(false)),
            stream_chunk: None,
            allow_memory_grounding: allow,
        }
    }

    fn seed_memory(store: &KnowledgeStoreHandle) {
        store.try_store_agent_memory(AgentMemoryEntry {
            id: "m1".into(),
            scope: PERSONAL_MEMORY_SCOPE.into(),
            workspace_id: None,
            title: "Coffee order".into(),
            content: "oat flat white".into(),
            visibility: "private".into(),
        });
    }

    #[test]
    fn injects_personal_memory_when_grounding_allowed() {
        let path = temp_db_path("explain-mem-on");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            seed_memory(&store);
            let last_prompt = Arc::new(Mutex::new(String::new()));
            let chat = Arc::new(CapturingProvider {
                last_prompt: Arc::clone(&last_prompt),
            });
            let response = call(
                req(serde_json::json!({ "text": "coffee" })),
                &deps_with_memory(chat, store, true),
            )
            .unwrap();
            assert!(last_prompt.lock().unwrap().contains("Coffee order"));
            assert_eq!(response.sources.len(), 1);
            assert_eq!(response.sources[0].title, "Coffee order");
        }
        cleanup(&path);
    }

    #[test]
    fn omits_personal_memory_for_cloud_provider() {
        let path = temp_db_path("explain-mem-off");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            seed_memory(&store);
            let last_prompt = Arc::new(Mutex::new(String::new()));
            let chat = Arc::new(CapturingProvider {
                last_prompt: Arc::clone(&last_prompt),
            });
            call(
                req(serde_json::json!({ "text": "coffee" })),
                &deps_with_memory(chat, store, false),
            )
            .unwrap();
            assert!(!last_prompt.lock().unwrap().contains("Coffee order"));
        }
        cleanup(&path);
    }
}
