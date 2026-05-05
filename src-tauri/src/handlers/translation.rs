use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::ai_manager::AiProvider;
use crate::managers::translation_manager::{TranslateRequest, TranslationManager};

/// 處理 `translation.*` 指令：translate。
pub struct TranslationHandler {
    manager: Arc<TranslationManager>,
    config: Arc<Mutex<ConfigManager>>,
}

impl TranslationHandler {
    pub fn new(manager: Arc<TranslationManager>, config: Arc<Mutex<ConfigManager>>) -> Self {
        Self { manager, config }
    }

    fn get_config(&self) -> Result<(AiProvider, u64), String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        let provider_name = cfg
            .get("translation.provider")
            .unwrap_or_else(|| "claude".into());
        let model = cfg
            .get("translation.model")
            .unwrap_or_else(|| "claude-sonnet-4-6".into());
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
            other => return Err(format!("unsupported translation.provider '{other}'")),
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
        Ok((provider, timeout))
    }
}

impl CommandHandler for TranslationHandler {
    fn namespace(&self) -> &'static str {
        "translation"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "translate" => {
                let request_id = payload
                    .get("request_id")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
                    .to_string();
                let src = payload
                    .get("src")
                    .and_then(Value::as_str)
                    .unwrap_or("auto")
                    .to_string();
                let dst = payload
                    .get("dst")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'dst'".to_string())?
                    .to_string();
                let text = payload
                    .get("text")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'text'".to_string())?
                    .to_string();

                let (provider, timeout) = self.get_config()?;
                self.manager.translate_async(TranslateRequest {
                    request_id: request_id.clone(),
                    src_lang: src,
                    dst_lang: dst,
                    text,
                    provider,
                    timeout_secs: timeout,
                });
                Ok(json!({ "status": "pending", "request_id": request_id }))
            }
            _ => Err(format!("unknown translation command '{command}'")),
        }
    }
}
