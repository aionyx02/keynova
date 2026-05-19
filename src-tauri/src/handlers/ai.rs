use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::ai_manager::{resolve_ai_runtime_config, AiManager, AiProvider, AiRuntimeConfig};
use crate::managers::model_manager::ModelManager;
use crate::managers::workspace_manager::WorkspaceManager;

const CHECK_SETUP_TTL: Duration = Duration::from_secs(300);

struct SetupCache {
    key: String,
    result: Value,
    at: Instant,
}

/// 處理 `ai.*` 指令：chat、clear_history、get_history、check_setup、unload。
pub struct AiHandler {
    manager: Arc<AiManager>,
    config: Arc<Mutex<ConfigManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    model_manager: Arc<ModelManager>,
    setup_cache: Mutex<Option<SetupCache>>,
    /// True while a chat request is in progress — rejects concurrent requests.
    in_flight: Arc<AtomicBool>,
    /// Per-request cancel tokens keyed by `request_id`. Entry inserted on `ai.chat`,
    /// removed either by `ai.cancel` (after setting the flag) or by `chat_async`'s
    /// completion path. Shared into the spawned chat thread for self-cleanup.
    cancel_flags: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl AiHandler {
    pub fn new(
        manager: Arc<AiManager>,
        config: Arc<Mutex<ConfigManager>>,
        workspace_manager: Arc<Mutex<WorkspaceManager>>,
        model_manager: Arc<ModelManager>,
    ) -> Self {
        Self {
            manager,
            config,
            workspace_manager,
            model_manager,
            setup_cache: Mutex::new(None),
            in_flight: Arc::new(AtomicBool::new(false)),
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn get_ai_config(&self) -> Result<AiRuntimeConfig, String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        resolve_ai_runtime_config(|key| cfg.get(key))
    }

    fn setup_cache_key(config: &AiRuntimeConfig) -> String {
        match &config.provider {
            AiProvider::Ollama { base_url, model } => format!("ollama:{}:{}", base_url, model),
            AiProvider::Claude { model, .. } => format!("claude:{}", model),
            AiProvider::OpenAI { base_url, model, .. } => format!("openai:{}:{}", base_url, model),
        }
    }

    fn check_setup_impl(&self) -> Value {
        let Ok(ai_config) = self.get_ai_config() else {
            return json!({
                "needs_setup": true,
                "reason": "Failed to read AI configuration",
                "provider": "unknown",
                "target_model": "",
                "ollama_reachable": false,
                "model_available": false,
                "recommended_model": "qwen2.5:7b",
            });
        };

        let cache_key = Self::setup_cache_key(&ai_config);
        if let Ok(cache) = self.setup_cache.lock() {
            if let Some(entry) = cache.as_ref() {
                if entry.key == cache_key && entry.at.elapsed() < CHECK_SETUP_TTL {
                    return entry.result.clone();
                }
            }
        }

        let hardware = self.model_manager.detect_hardware();
        let recommended = self
            .model_manager
            .recommend_models(&hardware)
            .into_iter()
            .next()
            .map(|c| c.name)
            .unwrap_or_else(|| "qwen2.5:7b".into());

        let result = match &ai_config.provider {
            AiProvider::Ollama { base_url, model } => {
                let (reachable, available, reason) =
                    check_ollama(base_url, model);
                let needs_setup = !reachable || !available;
                json!({
                    "needs_setup": needs_setup,
                    "provider": "ollama",
                    "target_model": model,
                    "ollama_reachable": reachable,
                    "model_available": available,
                    "recommended_model": recommended,
                    "reason": reason,
                })
            }
            AiProvider::Claude { api_key, model } => {
                let missing_key = api_key.trim().is_empty();
                json!({
                    "needs_setup": missing_key,
                    "provider": "claude",
                    "target_model": model,
                    "ollama_reachable": false,
                    "model_available": !missing_key,
                    "recommended_model": recommended,
                    "reason": if missing_key { "Claude API key is not set" } else { "" },
                })
            }
            AiProvider::OpenAI { api_key, model, .. } => {
                let missing_key = api_key.trim().is_empty();
                json!({
                    "needs_setup": missing_key,
                    "provider": "openai",
                    "target_model": model,
                    "ollama_reachable": false,
                    "model_available": !missing_key,
                    "recommended_model": recommended,
                    "reason": if missing_key { "OpenAI API key is not set" } else { "" },
                })
            }
        };

        if let Ok(mut cache) = self.setup_cache.lock() {
            *cache = Some(SetupCache {
                key: cache_key,
                result: result.clone(),
                at: Instant::now(),
            });
        }
        result
    }
}

/// Send keep_alive=0 to Ollama to unload the model from GPU/RAM.
/// Fire-and-forget — errors are silently ignored.
fn unload_ollama_model(base_url: &str, model: &str) {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let model = model.to_string();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build();
        if let Ok(client) = client {
            let _ = client
                .post(&url)
                .json(&serde_json::json!({ "model": model, "keep_alive": 0 }))
                .send();
        }
    });
}

/// Returns `(reachable, model_available, reason)`.
fn check_ollama(base_url: &str, target_model: &str) -> (bool, bool, &'static str) {
    let url = format!(
        "{}/api/tags",
        base_url.trim_end_matches('/')
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build();
    let Ok(client) = client else {
        return (false, false, "Failed to build HTTP client");
    };
    let Ok(resp) = client.get(&url).send() else {
        return (false, false, "Ollama is not reachable — is the daemon running?");
    };
    if !resp.status().is_success() {
        return (true, false, "Ollama responded but returned an error status");
    }
    let Ok(body) = resp.json::<serde_json::Value>() else {
        return (true, false, "Could not parse Ollama response");
    };
    let names: Vec<String> = body
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();

    // Match on prefix so "qwen2.5:7b" matches "qwen2.5:7b-instruct-q4_K_M" etc.
    let base = target_model.split(':').next().unwrap_or(target_model);
    let found = names
        .iter()
        .any(|n| n == target_model || n.starts_with(&format!("{base}:")));
    if found {
        (true, true, "")
    } else {
        (true, false, "Model not found in Ollama — run ollama pull")
    }
}

impl CommandHandler for AiHandler {
    fn namespace(&self) -> &'static str {
        "ai"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "check_setup" => Ok(self.check_setup_impl()),
            "chat" => {
                {
                    let cfg = self.config.lock().map_err(|e| e.to_string())?;
                    let enabled = cfg
                        .get("features.ai")
                        .as_deref()
                        .map(|v| !v.eq_ignore_ascii_case("false"))
                        .unwrap_or(true);
                    if !enabled {
                        return Err(
                            "AI 功能已停用。請前往 /setting → Features 開啟。".into()
                        );
                    }
                }
                if self.in_flight.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
                    return Err("另一個 AI 請求進行中，請等待回應後再試".into());
                }
                let request_id = payload
                    .get("request_id")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
                    .to_string();
                let prompt = payload
                    .get("prompt")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'prompt'".to_string())?
                    .to_string();

                let ai_config = self.get_ai_config()?;
                let cancel_flag = Arc::new(AtomicBool::new(false));
                if let Ok(mut flags) = self.cancel_flags.lock() {
                    flags.insert(request_id.clone(), Arc::clone(&cancel_flag));
                }
                self.manager.chat_async(
                    request_id.clone(),
                    prompt,
                    ai_config.provider,
                    ai_config.max_tokens,
                    ai_config.timeout_secs,
                    ai_config.ollama_keep_alive,
                    Some(Arc::clone(&self.in_flight)),
                    Some(cancel_flag),
                    Some(Arc::clone(&self.cancel_flags)),
                    ai_config.stream_enabled,
                );
                if let Ok(mut workspace) = self.workspace_manager.lock() {
                    workspace.record_ai_conversation(request_id.clone());
                }
                Ok(json!({ "status": "pending", "request_id": request_id }))
            }
            "cancel" => {
                let request_id = payload
                    .get("request_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'request_id'".to_string())?
                    .to_string();
                let removed = if let Ok(mut flags) = self.cancel_flags.lock() {
                    flags.remove(&request_id)
                } else {
                    None
                };
                let cancelled = match removed {
                    Some(flag) => {
                        flag.store(true, Ordering::SeqCst);
                        true
                    }
                    None => false,
                };
                Ok(json!({ "ok": true, "cancelled": cancelled, "request_id": request_id }))
            }
            "clear_history" => {
                self.manager.clear_history();
                Ok(json!({ "ok": true }))
            }
            "get_history" => Ok(json!(self.manager.get_history())),
            "unload" => {
                let ai_config = self.get_ai_config()?;
                if let AiProvider::Ollama { base_url, model } = &ai_config.provider {
                    unload_ollama_model(base_url, model);
                }
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("unknown ai command '{command}'")),
        }
    }
}

// ── 5.4: check_setup logic tests ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    /// Mirror of the prefix-match logic in check_ollama.
    fn model_found(names: &[&str], target: &str) -> bool {
        let base = target.split(':').next().unwrap_or(target);
        names
            .iter()
            .any(|&n| n == target || n.starts_with(&format!("{base}:")))
    }

    #[test]
    fn ollama_exact_model_match() {
        assert!(model_found(&["llama3:8b", "qwen2.5:7b"], "llama3:8b"));
    }

    #[test]
    fn ollama_prefix_variant_matches_target() {
        // "qwen2.5:7b" should match "qwen2.5:7b-instruct-q4_K_M"
        assert!(model_found(
            &["qwen2.5:7b-instruct-q4_K_M", "llama3:8b"],
            "qwen2.5:7b"
        ));
    }

    #[test]
    fn ollama_no_match_returns_false() {
        assert!(!model_found(&["llama3:8b"], "qwen2.5:7b"));
    }

    #[test]
    fn ollama_empty_model_list_is_not_found() {
        assert!(!model_found(&[], "qwen2.5:7b"));
    }

    #[test]
    fn empty_api_key_is_detected_as_missing() {
        // Claude/OpenAI providers set needs_setup = api_key.trim().is_empty()
        assert!("".trim().is_empty());
        assert!("   ".trim().is_empty());
        assert!(!"\t key\n".trim().is_empty());
    }

    #[test]
    fn model_prefix_does_not_cross_match_different_families() {
        // "qwen" should not match "qwen2.5:7b" because prefix check uses full base
        assert!(!model_found(&["qwen:7b"], "qwen2.5:7b"));
    }
}
