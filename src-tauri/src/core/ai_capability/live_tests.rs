//! REF.4 live integration tests against a local Ollama daemon.
//!
//! Gated by the `live-ai` cargo feature AND `#[ignore]` so they only run
//! when the developer explicitly opts in:
//!
//! ```bash
//! cargo test --features live-ai -- --ignored ai_capability_live --nocapture
//! ```
//!
//! Model defaults to ADR-0029 §8's `qwen2.5:7b` target. Override via the
//! `KEYNOVA_LIVE_AI_MODEL` env var when smoke-testing against a smaller
//! local model (e.g. `qwen2.5:0.5b`); the recorded latency is then a
//! lower bound, not the canonical P50 reading.
//!
//! Tests print the observed latency in milliseconds; the 800 ms P50 target
//! is recorded in the session log rather than asserted here — REF.7 owns
//! the quantitative gate.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use crate::core::ai_capability::{
    self, AiManagerChatProvider, CapabilityDeps, CapabilityId, CapabilityOutput, CapabilityRequest,
    ChatProvider,
};
use crate::managers::ai_manager::{AiManager, AiProvider, AiRuntimeConfig};

fn live_ai_model() -> String {
    std::env::var("KEYNOVA_LIVE_AI_MODEL").unwrap_or_else(|_| "qwen2.5:7b".into())
}

fn runtime_for_ollama() -> AiRuntimeConfig {
    AiRuntimeConfig {
        provider: AiProvider::Ollama {
            base_url: "http://localhost:11434".into(),
            model: live_ai_model(),
        },
        max_tokens: 512,
        timeout_secs: 60,
        ollama_keep_alive: "5m".into(),
        stream_enabled: false,
    }
}

fn build_chat() -> Arc<dyn ChatProvider> {
    let ai = Arc::new(AiManager::new(Arc::new(|_| {})));
    Arc::new(AiManagerChatProvider {
        ai,
        runtime: runtime_for_ollama(),
    })
}

fn deps_with_chat(chat: Arc<dyn ChatProvider>) -> CapabilityDeps {
    CapabilityDeps {
        chat,
        local_context: None,
        knowledge_store: None,
        cancel: Arc::new(AtomicBool::new(false)),
        stream_chunk: None,
    }
}

#[test]
#[ignore]
fn ai_capability_live_explain_returns_text() {
    let req = CapabilityRequest {
        id: CapabilityId::Explain,
        payload: serde_json::json!({ "text": "what does `rg` do?" }),
        context_hash: None,
    };
    let started = Instant::now();
    let resp = ai_capability::call_capability(req, &deps_with_chat(build_chat()))
        .expect("explain should succeed against live Ollama");
    let elapsed_ms = started.elapsed().as_millis();
    println!(
        "[ai_capability_live] model={} explain latency = {elapsed_ms} ms",
        live_ai_model()
    );
    match resp.output {
        CapabilityOutput::Text { text } => assert!(!text.trim().is_empty(), "got empty reply"),
        _ => panic!("expected text output"),
    }
    assert!(!resp.risk_tag.requires_confirmation);
}

#[test]
#[ignore]
fn ai_capability_live_fix_error_explains_compiler_error() {
    let req = CapabilityRequest {
        id: CapabilityId::FixError,
        payload: serde_json::json!({
            "raw_output": "error[E0308]: mismatched types\n  --> src/x.rs:1:1\n  |\n1 | fn main() { let x: i32 = \"hi\"; }\n",
        }),
        context_hash: None,
    };
    let started = Instant::now();
    let resp = ai_capability::call_capability(req, &deps_with_chat(build_chat()))
        .expect("fix_error should succeed against live Ollama");
    let elapsed_ms = started.elapsed().as_millis();
    println!(
        "[ai_capability_live] model={} fix_error latency = {elapsed_ms} ms",
        live_ai_model()
    );
    match resp.output {
        CapabilityOutput::Text { text } => assert!(!text.trim().is_empty(), "got empty reply"),
        _ => panic!("expected text output"),
    }
    assert!(!resp.risk_tag.requires_confirmation);
}
