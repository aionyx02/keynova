use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::ai_manager::{AiManager, AiProvider};

/// 處理 `ai.*` 指令：chat、clear_history、get_history。
pub struct AiHandler {
    manager: Arc<AiManager>,
    config: Arc<Mutex<ConfigManager>>,
}

impl AiHandler {
    pub fn new(manager: Arc<AiManager>, config: Arc<Mutex<ConfigManager>>) -> Self {
        Self { manager, config }
    }

    fn get_ai_config(&self) -> Result<(AiProvider, u32, u64), String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        let provider_name = cfg.get("ai.provider").unwrap_or_else(|| "claude".into());
        let model = cfg
            .get("ai.model")
            .unwrap_or_else(|| "claude-sonnet-4-6".into());
        let max_tokens = cfg
            .get("ai.max_tokens")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(4096);
        let provider = match provider_name.as_str() {
            "ollama" => AiProvider::Ollama {
                base_url: cfg
                    .get("ai.ollama_url")
                    .unwrap_or_else(|| "http://localhost:11434".into()),
                model,
            },
            "openai" => AiProvider::OpenAI {
                api_key: cfg.get("ai.openai_api_key").unwrap_or_default(),
                base_url: cfg
                    .get("ai.openai_base_url")
                    .unwrap_or_else(|| "https://api.openai.com/v1".into()),
                model,
            },
            "claude" => AiProvider::Claude {
                api_key: cfg.get("ai.api_key").unwrap_or_default(),
                model,
            },
            other => return Err(format!("unsupported ai.provider '{other}'")),
        };
        let timeout_key = if provider_name == "ollama" {
            "ai.ollama_timeout_secs"
        } else {
            "ai.timeout_secs"
        };
        let timeout = cfg
            .get(timeout_key)
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(if provider_name == "ollama" { 120 } else { 30 });
        Ok((provider, max_tokens, timeout))
    }
}

impl CommandHandler for AiHandler {
    fn namespace(&self) -> &'static str {
        "ai"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "chat" => {
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

                let (provider, max_tokens, timeout) = self.get_ai_config()?;
                self.manager
                    .chat_async(request_id.clone(), prompt, provider, max_tokens, timeout);
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
