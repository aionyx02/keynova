use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::models::ipc_requests::{SettingGetRequest, SettingSetRequest};
use crate::models::settings_schema::is_sensitive_key;

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
                let req: SettingGetRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid setting.get request: {e}"))?;
                let cfg = self.config.lock().map_err(|e| e.to_string())?;
                let value = cfg.get(&req.key).map(|raw| {
                    if is_sensitive_key(&req.key) && !raw.is_empty() {
                        "********".to_string()
                    } else {
                        raw
                    }
                });
                Ok(json!({ "value": value, "sensitive": is_sensitive_key(&req.key) }))
            }
            "set" => {
                let req: SettingSetRequest = serde_json::from_value(payload)
                    .map_err(|e| format!("invalid setting.set request: {e}"))?;
                let mut cfg = self.config.lock().map_err(|e| e.to_string())?;
                cfg.set(&req.key, &req.value)?;
                Ok(json!({ "ok": true }))
            }
            "list_all" => {
                let cfg = self.config.lock().map_err(|e| e.to_string())?;
                let pairs: Vec<_> = cfg
                    .list_all_redacted()
                    .into_iter()
                    .map(|(k, v, sensitive)| {
                        json!({ "key": k, "value": v, "sensitive": sensitive })
                    })
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