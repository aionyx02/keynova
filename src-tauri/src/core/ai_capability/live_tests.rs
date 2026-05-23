//! REF.4 live integration tests against a local Ollama daemon (`qwen2.5:7b`).
//!
//! Gated by the `live-ai` cargo feature AND `#[ignore]` so they only run
//! when the developer explicitly opts in:
//!
//! ```bash
//! cargo test --features live-ai -- --ignored ai_capability_live
//! ```
//!
//! Tests print the observed latency in milliseconds; ADR-0029 §8 target is
//! P50 < 800 ms. Any gap is recorded in the session log rather than failing
//! the test — REF.7 owns the quantitative gate.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use crate::core::ai_capability::{
    self, AiManagerChatProvider, CapabilityDeps, CapabilityId, CapabilityOutput, CapabilityRequest,
    ChatProvider,
};
use crate::managers::ai_manager::{AiManager, AiProvider, AiRuntimeConfig};

fn runtime_for_ollama() -> AiRuntimeConfig {
    AiRuntimeConfig {
        provider: AiProvider::Ollama {
            base_url: "http://localhost:11434".into(),
            model: "qwen2.5:7b".into(),
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
    println!("[ai_capability_live] explain latency = {elapsed_ms} ms");
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
    println!("[ai_capability_live] fix_error latency = {elapsed_ms} ms");
    match resp.output {
        CapabilityOutput::Text { text } => assert!(!text.trim().is_empty(), "got empty reply"),
        _ => panic!("expected text output"),
    }
    assert!(!resp.risk_tag.requires_confirmation);
}
