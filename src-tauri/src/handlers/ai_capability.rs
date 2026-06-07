//! REF.4 — IPC namespace `capability.*` for the stateless AI capability layer.
//!
//! Mirrors `handlers/ai.rs` for cancel + async response patterns. Capability
//! calls of distinct `request_id`s may run in parallel (no global single-
//! flight); per-chip disable is the UI's job (ADR-0029 §4).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::ai_capability::{
    self, contract::StreamChunkFn, AiManagerChatProvider, CapabilityDeps, CapabilityError,
    CapabilityId, CapabilityOutput, CapabilityRequest, ChatProvider,
};
use crate::core::config_manager::ConfigManager;
use crate::core::event_bus::EventBus;
use crate::core::knowledge_store::KnowledgeStoreHandle;
use crate::core::local_context::LocalContextSearcher;
use crate::core::AppEvent;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::ai_manager::{resolve_ai_runtime_config, AiManager, AiProvider};

pub struct AiCapabilityHandler {
    ai: Arc<AiManager>,
    config: Arc<Mutex<ConfigManager>>,
    local_context: LocalContextSearcher,
    knowledge_store: KnowledgeStoreHandle,
    event_bus: Arc<EventBus>,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

pub struct AiCapabilityHandlerDeps {
    pub ai: Arc<AiManager>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub local_context: LocalContextSearcher,
    pub knowledge_store: KnowledgeStoreHandle,
    pub event_bus: Arc<EventBus>,
}

impl AiCapabilityHandler {
    pub fn new(deps: AiCapabilityHandlerDeps) -> Self {
        Self {
            ai: deps.ai,
            config: deps.config,
            local_context: deps.local_context,
            knowledge_store: deps.knowledge_store,
            event_bus: deps.event_bus,
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn list(&self) -> Value {
        let metas: Vec<Value> = ai_capability::all()
            .iter()
            .map(|m| {
                json!({
                    "id": m.id.as_str(),
                    "audit": m.audit,
                    "accepts_context_hash": m.accepts_context_hash,
                })
            })
            .collect();
        json!({ "capabilities": metas })
    }

    fn cancel(&self, payload: Value) -> CommandResult {
        let request_id = payload
            .get("request_id")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing 'request_id'".to_string())?
            .to_string();
        let removed = self
            .cancel_flags
            .lock()
            .map_err(|e| e.to_string())?
            .remove(&request_id);
        let cancelled = match removed {
            Some(flag) => {
                flag.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        };
        Ok(json!({ "ok": true, "cancelled": cancelled, "request_id": request_id }))
    }

    fn call(&self, payload: Value) -> CommandResult {
        let id_str = payload
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing 'id'".to_string())?;
        let capability_id = CapabilityId::parse(id_str)
            .ok_or_else(|| format!("unknown capability id '{id_str}'"))?;
        let request_id = payload
            .get("request_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("cap-{}", Uuid::new_v4()));
        let inner_payload = payload.get("payload").cloned().unwrap_or(Value::Null);
        let context_hash = payload
            .get("context_hash")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let stream = payload
            .get("stream")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // Resolve provider config now so we fail fast before spawning. The same
        // lock also gates the whole inline-AI surface on `features.ai` (the new
        // capability layer was previously ungated); mirrors `handlers/ai.rs`.
        let runtime = {
            let cfg = self.config.lock().map_err(|e| e.to_string())?;
            let enabled = cfg
                .get("features.ai")
                .as_deref()
                .map(|v| !v.eq_ignore_ascii_case("false"))
                .unwrap_or(true);
            if !enabled {
                return Err("AI 功能已停用。請前往 /setting → Features 開啟。".into());
            }
            resolve_ai_runtime_config(|key| cfg.get(key))?
        };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.cancel_flags
            .lock()
            .map_err(|e| e.to_string())?
            .insert(request_id.clone(), Arc::clone(&cancel_flag));

        let request = CapabilityRequest {
            id: capability_id,
            payload: inner_payload,
            context_hash,
        };

        // Privacy boundary (ADR-0043): personal memory is injected into prompts
        // only for a local provider. Computed before `runtime` is moved into the
        // chat provider below.
        let allow_memory_grounding = matches!(runtime.provider, AiProvider::Ollama { .. });

        let chat: Arc<dyn ChatProvider> = Arc::new(AiManagerChatProvider {
            ai: Arc::clone(&self.ai),
            runtime,
        });
        let local_context = self.local_context.clone();
        let knowledge_store = self.knowledge_store.clone();
        let event_bus = Arc::clone(&self.event_bus);
        let cancel_flags = Arc::clone(&self.cancel_flags);
        let request_id_thread = request_id.clone();

        std::thread::spawn(move || {
            let stream_chunk: Option<StreamChunkFn> = if stream {
                let bus = Arc::clone(&event_bus);
                let rid = request_id_thread.clone();
                Some(Arc::new(move |delta: &str| {
                    let _ = bus.publish(AppEvent::new(
                        "capability.stream.chunk",
                        json!({ "request_id": rid, "delta": delta }),
                    ));
                }))
            } else {
                None
            };

            let deps = CapabilityDeps {
                chat,
                local_context: Some(local_context),
                knowledge_store: Some(knowledge_store),
                cancel: Arc::clone(&cancel_flag),
                stream_chunk,
                allow_memory_grounding,
            };

            let event = match ai_capability::call_capability(request, &deps) {
                Ok(resp) => {
                    let output = match resp.output {
                        CapabilityOutput::Text { text } => {
                            json!({ "kind": "text", "text": text })
                        }
                        CapabilityOutput::Structured { value } => {
                            json!({ "kind": "structured", "value": value })
                        }
                    };
                    json!({
                        "request_id": request_id_thread,
                        "ok": true,
                        "id": resp.id.as_str(),
                        "output": output,
                        "risk_tag": resp.risk_tag,
                        "sources": resp.sources,
                    })
                }
                Err(err) => {
                    let cancelled = matches!(err, CapabilityError::Cancelled);
                    json!({
                        "request_id": request_id_thread,
                        "ok": false,
                        "cancelled": cancelled,
                        "error": err.to_string(),
                    })
                }
            };

            let _ = event_bus.publish(AppEvent::new("capability.response", event));

            if let Ok(mut flags) = cancel_flags.lock() {
                flags.remove(&request_id_thread);
            }
        });

        Ok(json!({ "status": "pending", "request_id": request_id }))
    }
}

impl CommandHandler for AiCapabilityHandler {
    fn namespace(&self) -> &'static str {
        "capability"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list" => Ok(self.list()),
            "call" => self.call(payload),
            "cancel" => self.cancel(payload),
            _ => Err(format!("unknown capability command '{command}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ai_capability::registry::CapabilityId;

    #[test]
    fn list_returns_all_capability_metas() {
        let metas = ai_capability::all();
        assert_eq!(metas.len(), 8);
        let ids: Vec<_> = metas.iter().map(|m| m.id).collect();
        assert!(ids.contains(&CapabilityId::Explain));
        assert!(ids.contains(&CapabilityId::Summarize));
        assert!(ids.contains(&CapabilityId::FixError));
        assert!(ids.contains(&CapabilityId::GenCommand));
        assert!(ids.contains(&CapabilityId::SuggestNext));
        assert!(ids.contains(&CapabilityId::Remember));
        assert!(ids.contains(&CapabilityId::Recall));
        assert!(ids.contains(&CapabilityId::WorkspaceProfile));
    }

    #[test]
    fn parse_unknown_id_is_rejected() {
        assert!(CapabilityId::parse("delete_everything").is_none());
    }
}
