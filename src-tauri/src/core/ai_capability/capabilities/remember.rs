//! Capability: organize a raw personal note into a titled memory and persist it
//! locally so it can be recalled and (with a local model) used to ground future
//! answers.
//!
//! The LLM is used only to structure the note into `{title, content}`; storage
//! is local (`agent_memories`, scope `personal`). The stored memory never
//! leaves the device on its own — see `recall` for read-back and
//! `local_context::push_memory_sources` for opt-in, local-model-only grounding.

use std::sync::atomic::Ordering;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::parse::extract_first_json_object;
use crate::core::ai_capability::prompt::{build_prompt, maybe_audit};
use crate::core::ai_capability::registry::{meta, CapabilityId};
use crate::core::knowledge_store::AgentMemoryEntry;
use crate::models::unified_result::RiskTag;

/// Scope value separating user-curated personal memory from the dormant agent's
/// `long_term` rows in the shared `agent_memories` table. Reusing the table
/// avoids a schema migration; the distinct scope keeps the two sets isolated.
pub(crate) const PERSONAL_MEMORY_SCOPE: &str = "personal";
const MAX_TITLE_CHARS: usize = 120;
const MAX_CONTENT_CHARS: usize = 2000;

const SYSTEM: &str = "You organize a user's personal note into a durable memory. \
     Reply with strict JSON only: {\"title\": string, \"content\": string}. The \
     title is a short label (max ~10 words). The content is a tidy markdown \
     summary of the key points as bullet lines, preserving concrete facts \
     (names, numbers, preferences). Do not invent information. Never wrap the \
     JSON in markdown fences.";

#[derive(Debug, Deserialize)]
struct Payload {
    text: String,
}

#[derive(Debug, Default, Deserialize)]
struct ModelOutput {
    #[serde(default)]
    title: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RememberOutput {
    pub id: String,
    pub title: String,
    pub content: String,
    pub saved: bool,
}

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload)
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    let text = payload.text.trim().to_string();
    if text.is_empty() {
        return Err(CapabilityError::InvalidPayload("text is empty".into()));
    }
    if deps.cancel.load(Ordering::SeqCst) {
        return Err(CapabilityError::Cancelled);
    }

    let audit = meta(CapabilityId::Remember).audit;
    let prompt = build_prompt(SYSTEM, &[], &format!("Personal note to organize:\n{text}"));

    let reply = match deps.chat.chat(&prompt, &deps.cancel) {
        Ok(reply) => reply,
        Err(error) => {
            maybe_audit(
                deps.knowledge_store.as_ref(),
                audit,
                CapabilityId::Remember.as_str(),
                "error",
                &error,
                None,
            );
            return Err(CapabilityError::ProviderError(error));
        }
    };

    let (title, content) = parse_model_output(&reply, &text);

    let id = Uuid::new_v4().to_string();
    let saved = if let Some(store) = deps.knowledge_store.as_ref() {
        store.try_store_agent_memory(AgentMemoryEntry {
            id: id.clone(),
            scope: PERSONAL_MEMORY_SCOPE.to_string(),
            workspace_id: None,
            title: title.clone(),
            content: content.clone(),
            visibility: "private".to_string(),
        });
        true
    } else {
        false
    };

    maybe_audit(
        deps.knowledge_store.as_ref(),
        audit,
        CapabilityId::Remember.as_str(),
        "ok",
        "capability:remember stored personal memory",
        None,
    );

    let output = RememberOutput {
        id,
        title,
        content,
        saved,
    };
    Ok(CapabilityResponse {
        id: CapabilityId::Remember,
        risk_tag: RiskTag::none(),
        output: CapabilityOutput::Structured {
            value: serde_json::to_value(output).expect("RememberOutput must serialize to JSON"),
        },
    })
}

/// Parse the model reply into `(title, content)`, falling back to the original
/// note so a malformed reply still stores something useful.
fn parse_model_output(reply: &str, original: &str) -> (String, String) {
    let parsed = serde_json::from_str::<ModelOutput>(reply.trim())
        .ok()
        .or_else(|| {
            extract_first_json_object(reply)
                .and_then(|json| serde_json::from_str::<ModelOutput>(json).ok())
        });

    let (mut title, mut content) = match parsed {
        Some(m) => (m.title.trim().to_string(), m.content.trim().to_string()),
        None => (String::new(), String::new()),
    };

    if content.is_empty() {
        content = original.trim().to_string();
    }
    if title.is_empty() {
        title = derive_title(original);
    }

    truncate_chars(&mut title, MAX_TITLE_CHARS);
    truncate_chars(&mut content, MAX_CONTENT_CHARS);
    (title, content)
}

fn derive_title(text: &str) -> String {
    let first_line = text.lines().next().unwrap_or("").trim();
    let mut title = if first_line.is_empty() {
        "Untitled memory".to_string()
    } else {
        first_line.to_string()
    };
    truncate_chars(&mut title, MAX_TITLE_CHARS);
    title
}

fn truncate_chars(s: &mut String, max: usize) {
    if s.chars().count() > max {
        *s = s.chars().take(max).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::contract::ChatProvider;
    use crate::core::knowledge_store::KnowledgeStoreHandle;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    struct JsonProvider;
    impl ChatProvider for JsonProvider {
        fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok(r#"{"title":"Coffee order","content":"- Oat flat white\n- No sugar"}"#.into())
        }
    }

    struct ProseProvider;
    impl ChatProvider for ProseProvider {
        fn chat(&self, _prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
            Ok("I could not produce JSON, sorry.".into())
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

    fn deps(chat: Arc<dyn ChatProvider>, store: Option<KnowledgeStoreHandle>) -> CapabilityDeps {
        CapabilityDeps {
            chat,
            local_context: None,
            knowledge_store: store,
            cancel: Arc::new(AtomicBool::new(false)),
            stream_chunk: None,
        }
    }

    fn req(payload: serde_json::Value) -> CapabilityRequest {
        CapabilityRequest {
            id: CapabilityId::Remember,
            payload,
            context_hash: None,
        }
    }

    #[test]
    fn rejects_empty_text() {
        let err = call(
            req(serde_json::json!({ "text": "   " })),
            &deps(Arc::new(JsonProvider), None),
        )
        .unwrap_err();
        assert!(matches!(err, CapabilityError::InvalidPayload(_)));
    }

    #[test]
    fn organizes_and_stores_personal_memory() {
        let path = temp_db_path("remember");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            let resp = call(
                req(serde_json::json!({ "text": "I drink oat flat whites, no sugar" })),
                &deps(Arc::new(JsonProvider), Some(store.clone())),
            )
            .unwrap();
            assert_eq!(resp.id, CapabilityId::Remember);
            let out: RememberOutput = match resp.output {
                CapabilityOutput::Structured { value } => serde_json::from_value(value).unwrap(),
                _ => panic!("expected structured output"),
            };
            assert_eq!(out.title, "Coffee order");
            assert!(out.saved);

            // Read-back: the worker processes the queued write before this read.
            let stored = store
                .agent_memories_blocking(Some(PERSONAL_MEMORY_SCOPE.into()), None, 10)
                .unwrap();
            assert_eq!(stored.len(), 1);
            assert_eq!(stored[0].title, "Coffee order");
            assert_eq!(stored[0].scope, PERSONAL_MEMORY_SCOPE);
        }
        cleanup(&path);
    }

    #[test]
    fn falls_back_to_original_when_model_returns_no_json() {
        let (title, content) =
            parse_model_output("I could not produce JSON, sorry.", "remember my dog is Mochi");
        assert_eq!(title, "remember my dog is Mochi");
        assert_eq!(content, "remember my dog is Mochi");
    }

    #[test]
    fn prose_provider_still_stores_via_fallback() {
        let path = temp_db_path("remember-fallback");
        {
            let store = KnowledgeStoreHandle::new(path.clone(), 8);
            let resp = call(
                req(serde_json::json!({ "text": "my dog is named Mochi" })),
                &deps(Arc::new(ProseProvider), Some(store)),
            )
            .unwrap();
            let out: RememberOutput = match resp.output {
                CapabilityOutput::Structured { value } => serde_json::from_value(value).unwrap(),
                _ => panic!("expected structured output"),
            };
            assert!(out.saved);
            assert_eq!(out.content, "my dog is named Mochi");
        }
        cleanup(&path);
    }
}
