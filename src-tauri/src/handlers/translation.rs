use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
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

    fn timeout_secs(&self) -> Result<u64, String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        Ok(cfg
            .get("translation.timeout_secs")
            .or_else(|| cfg.get("ai.timeout_secs"))
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30))
    }
}

impl CommandHandler for TranslationHandler {
    fn namespace(&self) -> &'static str {
        "translation"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list_langs" => {
                let list: Vec<serde_json::Value> =
                    crate::managers::translation_manager::SUPPORTED_LANGS
                        .iter()
                        .map(|(code, name)| json!({ "code": code, "name": name }))
                        .collect();
                Ok(serde_json::to_value(list).map_err(|e| e.to_string())?)
            }
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

                let timeout = self.timeout_secs()?;
                self.manager.translate_async(TranslateRequest {
                    request_id: request_id.clone(),
                    src_lang: src,
                    dst_lang: dst,
                    text,
                    timeout_secs: timeout,
                });
                Ok(json!({ "status": "pending", "request_id": request_id }))
            }
            _ => Err(format!("unknown translation command '{command}'")),
        }
    }
}
