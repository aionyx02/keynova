//! REF.4 live integration tests against a local Ollama daemon.
//!
//! Gated by the `live-ai` cargo feature AND `#[ignore]` so they only run
//! when the developer explicitly opts in:
//!
//! ```bash
//! cargo test --features live-ai -- --ignored ai_capability_live --nocapture
//! ```
//!
//! Model defaults to ADR-0029 §8's `qwen2.5:1.5b` reference default (the
//! 2026-06-06 amendment; `qwen2.5:7b` stays a higher-quality option). Override
//! via the `KEYNOVA_LIVE_AI_MODEL` env var when smoke-testing against another
//! local model; the recorded latency is then for that model, not the canonical
//! reference reading.
//!
//! Tests print the observed latency in milliseconds; the tiered §8 P50 targets
//! (CPU-host < 5000 ms, GPU/ideal < 800 ms) are recorded in the session log /
//! §10 rather than asserted here — REF.7 owns the quantitative gate.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use crate::core::ai_capability::{
    self, AiManagerChatProvider, CapabilityDeps, CapabilityId, CapabilityOutput, CapabilityRequest,
    ChatProvider,
};
use crate::managers::ai_manager::{AiManager, AiProvider, AiRuntimeConfig};

fn live_ai_model() -> String {
    std::env::var("KEYNOVA_LIVE_AI_MODEL").unwrap_or_else(|_| "qwen2.5:1.5b".into())
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
        allow_memory_grounding: false,
    }
}

fn deps_with_chat_streaming(
    chat: Arc<dyn ChatProvider>,
    chunks: Arc<std::sync::Mutex<Vec<String>>>,
) -> CapabilityDeps {
    let stream_chunk: crate::core::ai_capability::contract::StreamChunkFn = {
        let chunks = Arc::clone(&chunks);
        Arc::new(move |delta: &str| {
            if let Ok(mut buf) = chunks.lock() {
                buf.push(delta.to_string());
            }
        })
    };
    CapabilityDeps {
        chat,
        local_context: None,
        knowledge_store: None,
        cancel: Arc::new(AtomicBool::new(false)),
        stream_chunk: Some(stream_chunk),
        allow_memory_grounding: false,
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
fn ai_capability_live_explain_streams_chunks() {
    // REF.6.A path: this mirrors what `useCapability({ stream: true })`
    // exercises. Verifies that streaming chunks fire AND a final response
    // text accumulates — not just the non-streaming return.
    let chunks = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let req = CapabilityRequest {
        id: CapabilityId::Explain,
        payload: serde_json::json!({ "text": "what does the unix command `ls` do?" }),
        context_hash: None,
    };
    let started = Instant::now();
    let resp = ai_capability::call_capability(
        req,
        &deps_with_chat_streaming(build_chat(), Arc::clone(&chunks)),
    )
    .expect("explain should succeed with streaming against live Ollama");
    let elapsed_ms = started.elapsed().as_millis();

    let captured = chunks.lock().unwrap();
    let chunk_count = captured.len();
    let stream_text: String = captured.concat();
    drop(captured);

    println!(
        "[ai_capability_live] model={} explain STREAM latency = {elapsed_ms} ms, chunks = {chunk_count}, stream_text_len = {}",
        live_ai_model(),
        stream_text.len()
    );

    // The Ctrl+E user surface receives chunks > 0 for any non-trivial reply.
    assert!(
        chunk_count > 0,
        "expected ≥1 stream chunk, got 0 — frontend would stay on Streaming…"
    );
    match &resp.output {
        CapabilityOutput::Text { text } => {
            assert!(!text.trim().is_empty(), "final accumulated text was empty");
            // Sanity: the streamed chunks concatenate to the same final text.
            assert_eq!(
                text.trim(),
                stream_text.trim(),
                "stream concat must equal final reply"
            );
        }
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

#[test]
#[ignore]
fn ai_capability_live_gen_command_returns_structured_output() {
    let req = CapabilityRequest {
        id: CapabilityId::GenCommand,
        payload: serde_json::json!({
            "intent": "show the current git branch status",
            "ctx": { "shell": "powershell", "os": "windows" }
        }),
        context_hash: None,
    };
    let started = Instant::now();
    let resp = ai_capability::call_capability(req, &deps_with_chat(build_chat()))
        .expect("gen_command should succeed against live Ollama");
    let elapsed_ms = started.elapsed().as_millis();
    println!(
        "[ai_capability_live] model={} gen_command latency = {elapsed_ms} ms",
        live_ai_model()
    );
    match resp.output {
        CapabilityOutput::Structured { value } => {
            let command = value
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let confidence = value
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let rationale = value
                .get("rationale")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            assert!(!command.is_empty(), "expected non-empty generated command");
            assert!(
                (0.0..=1.0).contains(&confidence),
                "confidence must stay within [0, 1]"
            );
            assert!(!rationale.is_empty(), "expected non-empty rationale");
        }
        _ => panic!("expected structured output"),
    }
}
