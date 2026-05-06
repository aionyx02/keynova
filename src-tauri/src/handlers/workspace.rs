use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::{CommandHandler, CommandResult};
use crate::managers::workspace_manager::WorkspaceManager;

/// 處理 `workspace.*` 指令：switch、get_current、save_current、get_all。
pub struct WorkspaceHandler {
    manager: Arc<Mutex<WorkspaceManager>>,
}

impl WorkspaceHandler {
    pub fn new(manager: Arc<Mutex<WorkspaceManager>>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for WorkspaceHandler {
    fn namespace(&self) -> &'static str {
        "workspace"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "switch" => {
                let slot = payload
                    .get("slot")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| "missing 'slot'".to_string())? as usize;
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let state = mgr.switch_to(slot)?;
                Ok(json!(state))
            }
            "get_current" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                Ok(json!(mgr.current()))
            }
            "save_current" => {
                let query = payload
                    .get("query")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let mode = payload
                    .get("mode")
                    .and_then(Value::as_str)
                    .unwrap_or("search")
                    .to_string();
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.save_current(query, mode);
                Ok(json!({ "ok": true }))
            }
            "get_all" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                Ok(json!(mgr.all()))
            }
            _ => Err(format!("unknown workspace command '{command}'")),
        }
    }
}