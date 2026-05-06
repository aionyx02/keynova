use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::{CommandHandler, CommandResult};
use crate::managers::system_manager::SystemManager;

/// 處理 `system.*` 指令：ping、volume.get/set/mute、brightness.get/set、wifi.info。
pub struct SystemControlHandler {
    manager: Arc<Mutex<SystemManager>>,
}

impl SystemControlHandler {
    pub fn new(manager: Arc<Mutex<SystemManager>>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for SystemControlHandler {
    fn namespace(&self) -> &'static str {
        "system"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "ping" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or("Developer");
                Ok(json!({ "message": format!("Pong from Rust CommandRouter, {}!", name) }))
            }
            "volume.get" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let info = mgr.get_volume()?;
                Ok(json!(info))
            }
            "volume.set" => {
                let level = payload
                    .get("level")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| "missing 'level' (0.0–1.0)".to_string())?
                    as f32;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.set_volume(level)?;
                Ok(json!({ "ok": true, "level": level }))
            }
            "volume.mute" => {
                let muted = payload
                    .get("muted")
                    .and_then(Value::as_bool)
                    .unwrap_or(true);
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.set_mute(muted)?;
                Ok(json!({ "ok": true, "muted": muted }))
            }
            "brightness.get" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let level = mgr.get_brightness()?;
                Ok(json!({ "level": level }))
            }
            "brightness.set" => {
                let level = payload
                    .get("level")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "missing 'level' (0–100)".to_string())?
                    as u32;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.set_brightness(level)?;
                Ok(json!({ "ok": true, "level": level }))
            }
            "wifi.info" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let info = mgr.get_wifi_info()?;
                Ok(json!(info))
            }
            _ => Err(format!("unknown system command '{command}'")),
        }
    }
}
