//! Capability contract types (REF.4 / ADR-0029 §4 / ADR-0030).
//!
//! Request, response, error, and dependency types handed to
//! `core::ai_capability::call_capability`. Capability execution is single-
//! shot — no session memory, no chaining.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::knowledge_store::KnowledgeStoreHandle;
use crate::core::local_context::LocalContextSearcher;
use crate::managers::ai_manager::{AiManager, AiRuntimeConfig};
use crate::models::unified_result::RiskTag;

use super::registry::CapabilityId;

/// Caller-supplied request to `call_capability`.
#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityRequest {
    pub id: CapabilityId,
    pub payload: Value,
    /// Optional REF.5 workflow-memory context hash. Initial three capabilities
    /// ignore the value (kept for forward compat per ADR-0029 §4).
    #[serde(default)]
    pub context_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapabilityOutput {
    Text { text: String },
    Structured { value: Value },
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityResponse {
    pub id: CapabilityId,
    pub output: CapabilityOutput,
    pub risk_tag: RiskTag,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum CapabilityError {
    UnknownId,
    InvalidPayload(String),
    ProviderError(String),
    Cancelled,
    UnsupportedAction(String),
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownId => write!(f, "unknown capability id"),
            Self::InvalidPayload(msg) => write!(f, "invalid payload: {msg}"),
            Self::ProviderError(msg) => write!(f, "provider error: {msg}"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::UnsupportedAction(msg) => write!(f, "unsupported action: {msg}"),
        }
    }
}

/// Provider abstraction so capability code can be unit-tested without hitting
/// a live LLM. Production impl is `AiManagerChatProvider`; tests stub this.
pub trait ChatProvider: Send + Sync {
    fn chat(&self, prompt: &str, cancel: &AtomicBool) -> Result<String, String>;

    /// Streaming variant. Default falls back to non-streaming and emits the
    /// full reply as a single chunk so callers always have one event path.
    fn chat_stream(
        &self,
        prompt: &str,
        on_chunk: &dyn Fn(&str),
        cancel: &AtomicBool,
    ) -> Result<String, String> {
        let reply = self.chat(prompt, cancel)?;
        on_chunk(&reply);
        Ok(reply)
    }
}

/// Production adapter wrapping `AiManager::chat_sync` / `chat_sync_stream`.
pub struct AiManagerChatProvider {
    pub ai: Arc<AiManager>,
    pub runtime: AiRuntimeConfig,
}

impl ChatProvider for AiManagerChatProvider {
    fn chat(&self, prompt: &str, _cancel: &AtomicBool) -> Result<String, String> {
        self.ai.chat_sync(
            prompt,
            &self.runtime.provider,
            self.runtime.max_tokens,
            self.runtime.timeout_secs,
            &self.runtime.ollama_keep_alive,
        )
    }

    fn chat_stream(
        &self,
        prompt: &str,
        on_chunk: &dyn Fn(&str),
        cancel: &AtomicBool,
    ) -> Result<String, String> {
        use std::sync::atomic::Ordering;
        let cancel_check = || cancel.load(Ordering::SeqCst);
        self.ai.chat_sync_stream(
            prompt,
            &self.runtime.provider,
            self.runtime.max_tokens,
            self.runtime.timeout_secs,
            &self.runtime.ollama_keep_alive,
            on_chunk,
            &cancel_check,
        )
    }
}

/// Streaming callback handed to a capability for incremental chunk delivery.
pub type StreamChunkFn = Arc<dyn Fn(&str) + Send + Sync>;

/// Shared dependencies handed to every capability call. Owned values — the
/// handler builds one of these per request inside the worker thread.
pub struct CapabilityDeps {
    pub chat: Arc<dyn ChatProvider>,
    pub local_context: Option<LocalContextSearcher>,
    pub knowledge_store: Option<KnowledgeStoreHandle>,
    pub cancel: Arc<AtomicBool>,
    /// When set, the capability emits incremental chunks via this callback in
    /// addition to returning the accumulated reply.
    pub stream_chunk: Option<StreamChunkFn>,
    /// Privacy boundary (ADR-0043): personal memory is injected into prompts
    /// only when the resolved provider is local (ollama). The handler sets this
    /// from `AiRuntimeConfig.provider`; cloud providers leave it `false` so
    /// personal memory never leaves the device via a cloud prompt.
    pub allow_memory_grounding: bool,
}
