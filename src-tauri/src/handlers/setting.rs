use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::models::settings_schema::is_sensitive_key;

/// 處理 `setting.*` 命名空間的 IPC 請求，代理至 ConfigManager。
pub struct SettingHandler {
    config: Arc<Mutex<ConfigManager>>,
}

impl SettingHandler {
    pub fn new(config: Arc<Mutex<ConfigManager>>) -> Self {
        Self { config }
    }
}

impl CommandHandler for SettingHandler {
    fn namespace(&self) -> &'static str {
        "setting"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "get" => {
                let key = payload
                    .get("key")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'key' field".to_string())?;
                let cfg = self.config.lock().map_err(|e| e.to_string())?;
                let value = cfg.get(key).map(|raw| {
                    if is_sensitive_key(key) && !raw.is_empty() {
                        "********".to_string()
                    } else {
                        raw
                    }
                });
                Ok(json!({ "value": value, "sensitive": is_sensitive_key(key) }))
            }
            "set" => {
                let key = payload
                    .get("key")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'key' field".to_string())?;
                let value = payload
                    .get("value")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'value' field".to_string())?;
                let mut cfg = self.config.lock().map_err(|e| e.to_string())?;
                cfg.set(key, value)?;
                Ok(json!({ "ok": true }))
            }
            "list_all" => {
                let cfg = self.config.lock().map_err(|e| e.to_string())?;
                let pairs: Vec<_> = cfg
                    .list_all_redacted()
                    .into_iter()
                    .map(
                        |(k, v, sensitive)| json!({ "key": k, "value": v, "sensitive": sensitive }),
                    )
                    .collect();
                Ok(json!(pairs))
            }
            "schema" => {
                let cfg = self.config.lock().map_err(|e| e.to_string())?;
                Ok(json!(cfg.schema()))
            }
            _ => Err(format!("unknown setting command '{command}'")),
        }
    }
}
