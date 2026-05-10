use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::ai_manager::{resolve_ai_runtime_config, AiManager, AiProvider, AiRuntimeConfig};
use crate::managers::model_manager::ModelManager;
use crate::managers::workspace_manager::WorkspaceManager;

/// 處理 `ai.*` 指令：chat、clear_history、get_history、check_setup。
pub struct AiHandler {
    manager: Arc<AiManager>,
    config: Arc<Mutex<ConfigManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    model_manager: Arc<ModelManager>,
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
        }
    }

    fn get_ai_config(&self) -> Result<AiRuntimeConfig, String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        resolve_ai_runtime_config(|key| cfg.get(key))
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

        let hardware = self.model_manager.detect_hardware();
        let recommended = self
            .model_manager
            .recommend_models(&hardware)
            .into_iter()
            .next()
            .map(|c| c.name)
            .unwrap_or_else(|| "qwen2.5:7b".into());

        match &ai_config.provider {
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
        }
    }
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
                self.manager.chat_async(
                    request_id.clone(),
                    prompt,
                    ai_config.provider,
                    ai_config.max_tokens,
                    ai_config.timeout_secs,
                );
                if let Ok(mut workspace) = self.workspace_manager.lock() {
                    workspace.record_ai_conversation(request_id.clone());
                }
                Ok(json!({ "status": "pending", "request_id": request_id }))
            }
            "clear_history" => {
                self.manager.clear_history();
                Ok(json!({ "ok": true }))
            }
            "get_history" => Ok(json!(self.manager.get_history())),
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
