use crate::core::{CommandHandler, CommandResult};
use crate::managers::hotkey_manager::HotkeyManager;
use crate::models::hotkey::HotkeyConfig;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// 全局熱鍵系統的 IPC 處理器。
pub struct HotkeyHandler {
    manager: Arc<Mutex<HotkeyManager>>,
}

impl HotkeyHandler {
    pub fn new(manager: Arc<Mutex<HotkeyManager>>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for HotkeyHandler {
    fn namespace(&self) -> &'static str {
        "hotkey"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let hotkeys = mgr.list_hotkeys();
                Ok(serde_json::to_value(hotkeys).map_err(|e| e.to_string())?)
            }
            "register" | "unregister" => {
                Err("NotImplemented: hotkeys are managed via config.toml + restart; runtime registration is not supported".into())
            }
            "check_conflict" => {
                let config: HotkeyConfig =
                    serde_json::from_value(payload).map_err(|e| e.to_string())?;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let conflict = mgr.check_conflict(&config);
                Ok(serde_json::to_value(conflict).map_err(|e| e.to_string())?)
            }
            _ => Err(format!("hotkey: unknown command '{command}'")),
        }
    }
}
