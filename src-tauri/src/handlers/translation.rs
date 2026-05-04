use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::{CommandHandler, CommandResult};
use crate::managers::translation_manager::{TranslateRequest, TranslationManager};
use crate::core::config_manager::ConfigManager;

/// 處理 `translation.*` 指令：translate。
pub struct TranslationHandler {
    manager: Arc<TranslationManager>,
    config: Arc<Mutex<ConfigManager>>,
}

impl TranslationHandler {
    pub fn new(
        manager: Arc<TranslationManager>,
        config: Arc<Mutex<ConfigManager>>,
    ) -> Self {
        Self { manager, config }
    }

    fn get_config(&self) -> Result<(String, String, u64), String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        let api_key = cfg.get("ai.api_key").unwrap_or_default();
        let model = cfg.get("ai.model").unwrap_or_else(|| "claude-sonnet-4-6".into());
        let timeout = cfg
            .get("ai.timeout_secs")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);
        Ok((api_key, model, timeout))
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

                let (api_key, model, timeout) = self.get_config()?;
                self.manager.translate_async(TranslateRequest {
                    request_id: request_id.clone(),
                    src_lang: src,
                    dst_lang: dst,
                    text,
                    api_key,
                    model,
                    timeout_secs: timeout,
                });
                Ok(json!({ "status": "pending", "request_id": request_id }))
            }
            _ => Err(format!("unknown translation command '{command}'")),
        }
    }
}