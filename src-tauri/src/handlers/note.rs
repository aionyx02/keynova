use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::{CommandHandler, CommandResult};
use crate::managers::note_manager::NoteManager;

/// 處理 `note.*` 指令：list、get、save、create、delete、rename。
pub struct NoteHandler {
    manager: Arc<Mutex<NoteManager>>,
}

impl NoteHandler {
    pub fn new(manager: Arc<Mutex<NoteManager>>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for NoteHandler {
    fn namespace(&self) -> &'static str {
        "note"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                Ok(json!(mgr.list()))
            }
            "get" => {
                let name = require_str(&payload, "name")?;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let content = mgr.get(name)?;
                Ok(json!({ "name": name, "content": content }))
            }
            "save" => {
                let name = require_str(&payload, "name")?;
                let content = payload
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.save(name, content)?;
                Ok(json!({ "ok": true }))
            }
            "create" => {
                let name = require_str(&payload, "name")?;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.create(name)?;
                Ok(json!({ "ok": true, "name": name }))
            }
            "delete" => {
                let name = require_str(&payload, "name")?;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.delete(name)?;
                Ok(json!({ "ok": true }))
            }
            "rename" => {
                let old_name = require_str(&payload, "old_name")?;
                let new_name = require_str(&payload, "new_name")?;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.rename(old_name, new_name)?;
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("unknown note command '{command}'")),
        }
    }
}

fn require_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str, String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("missing or empty '{key}'"))
}