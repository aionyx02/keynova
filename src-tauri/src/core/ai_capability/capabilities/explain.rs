//! Capability: explain a focused result row, command, or text snippet using
//! local context (workspace metadata, recent notes/history) for grounding.
//!
//! Single-shot. `audit = true`, `requires_confirmation = false`.

use std::sync::atomic::Ordering;

use serde::Deserialize;

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::prompt::{build_prompt, maybe_audit};
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

    let task = match &payload.question {
        Some(q) if !q.trim().is_empty() => format!(
            "Explain: {}\n\nUser's focused question: {}",
            payload.text, q
        ),
        _ => format!("Explain succinctly: {}", payload.text),
    };
    let prompt = build_prompt(SYSTEM, &sources, &task);

    if deps.cancel.load(Ordering::SeqCst) {
        return Err(CapabilityError::Cancelled);
    }

    let reply = match &deps.stream_chunk {
        Some(on_chunk) => deps
            .chat
            .chat_stream(&prompt, on_chunk.as_ref(), &deps.cancel),
        None => deps.chat.chat(&prompt, &deps.cancel),
    };

    let audit = meta(CapabilityId::Explain).audit;
    match reply {
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

    fn deps(chat: Arc<dyn ChatProvider>) -> CapabilityDeps {
        CapabilityDeps {
            chat,
            local_context: None,
            knowledge_store: None,
            cancel: Arc::new(AtomicBool::new(false)),
            stream_chunk: None,
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
        let err = call(req(serde_json::json!({ "text": "" })), &deps(Arc::new(EchoProvider))).unwrap_err();
        assert!(matches!(err, CapabilityError::InvalidPayload(_)));
    }

    #[test]
    fn happy_path_returns_text_with_no_confirm() {
        let resp = call(
            req(serde_json::json!({ "text": "what is rg?", "question": "why use it" })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap();
        assert!(!resp.risk_tag.requires_confirmation);
        assert_eq!(resp.id, CapabilityId::Explain);
    }

    #[test]
    fn provider_error_propagates() {
        let err = call(req(serde_json::json!({ "text": "x" })), &deps(Arc::new(ErrProvider))).unwrap_err();
        assert!(matches!(err, CapabilityError::ProviderError(_)));
    }
}
