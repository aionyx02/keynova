//! Capability: summarize a free-form text input.
//!
//! Pure transform — no local context lookup. `audit = false`,
//! `requires_confirmation = false` per ADR-0030 §4.

use std::sync::atomic::Ordering;

use serde::Deserialize;

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::prompt::build_prompt;
use crate::core::ai_capability::registry::CapabilityId;
use crate::models::unified_result::RiskTag;

#[derive(Debug, Deserialize)]
struct Payload {
    text: String,
    #[serde(default)]
    max_sentences: Option<u32>,
}

const SYSTEM: &str =
    "You are a concise summarizer integrated into Keynova, a keyboard-first launcher. \
     Summarize the user's text in plain prose. Never invent facts that are not in the input.";

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload)
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    if payload.text.trim().is_empty() {
        return Err(CapabilityError::InvalidPayload("text is empty".into()));
    }
    if deps.cancel.load(Ordering::SeqCst) {
        return Err(CapabilityError::Cancelled);
    }

    let task = match payload.max_sentences {
        Some(n) if n > 0 => format!(
            "Summarize the following in at most {n} sentence(s):\n\n{}",
            payload.text
        ),
        _ => format!("Summarize the following:\n\n{}", payload.text),
    };
    let prompt = build_prompt(SYSTEM, &[], &task);

    let reply = match &deps.stream_chunk {
        Some(on_chunk) => deps
            .chat
            .chat_stream(&prompt, on_chunk.as_ref(), &deps.cancel),
        None => deps.chat.chat(&prompt, &deps.cancel),
    }
    .map_err(CapabilityError::ProviderError)?;

    Ok(CapabilityResponse {
        id: CapabilityId::Summarize,
        output: CapabilityOutput::Text { text: reply },
        risk_tag: RiskTag::none(),
    })
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
            Ok(format!("summary: {}", prompt.lines().last().unwrap_or("")))
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
            id: CapabilityId::Summarize,
            payload,
            context_hash: None,
        }
    }

    #[test]
    fn rejects_empty_text() {
        let err = call(
            req(serde_json::json!({ "text": "   " })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap_err();
        assert!(matches!(err, CapabilityError::InvalidPayload(_)));
    }

    #[test]
    fn happy_path_returns_text_with_no_confirm() {
        let resp = call(
            req(serde_json::json!({ "text": "hello world" })),
            &deps(Arc::new(EchoProvider)),
        )
        .unwrap();
        assert!(!resp.risk_tag.requires_confirmation);
        match resp.output {
            CapabilityOutput::Text { text } => assert!(text.starts_with("summary:")),
            _ => panic!("expected text output"),
        }
    }

    #[test]
    fn cancel_before_call_returns_cancelled() {
        let cancel = Arc::new(AtomicBool::new(true));
        let d = CapabilityDeps {
            chat: Arc::new(EchoProvider),
            local_context: None,
            knowledge_store: None,
            cancel,
            stream_chunk: None,
        };
        assert!(matches!(
            call(req(serde_json::json!({ "text": "hello" })), &d).unwrap_err(),
            CapabilityError::Cancelled
        ));
    }
}
