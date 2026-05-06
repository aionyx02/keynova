use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};

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
                Ok(json!({ "value": cfg.get(key) }))
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
                    .list_all()
                    .into_iter()
                    .map(|(k, v)| json!({ "key": k, "value": v }))
                    .collect();
                Ok(json!(pairs))
            }
            _ => Err(format!("unknown setting command '{command}'")),
        }
    }
}
