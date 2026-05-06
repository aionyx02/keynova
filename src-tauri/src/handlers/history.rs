use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::{CommandHandler, CommandResult};
use crate::managers::history_manager::HistoryManager;

/// 處理 `history.*` 指令：list、search、delete、pin、unpin、clear。
pub struct HistoryHandler {
    manager: Arc<Mutex<HistoryManager>>,
}

impl HistoryHandler {
    pub fn new(manager: Arc<Mutex<HistoryManager>>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for HistoryHandler {
    fn namespace(&self) -> &'static str {
        "history"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                Ok(json!(mgr.list()))
            }
            "search" => {
                let q = payload.get("q").and_then(Value::as_str).unwrap_or("");
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let results: Vec<_> = mgr.search(q).into_iter().collect();
                Ok(json!(results))
            }
            "delete" => {
                let id = require_str(&payload, "id")?;
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let ok = mgr.delete(id);
                Ok(json!({ "ok": ok }))
            }
            "pin" => {
                let id = require_str(&payload, "id")?;
                let pinned = payload.get("pinned").and_then(Value::as_bool).unwrap_or(true);
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let ok = mgr.pin(id, pinned);
                Ok(json!({ "ok": ok }))
            }
            "clear" => {
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.clear_unpinned();
                Ok(json!({ "ok": true }))
            }
            _ => Err(format!("unknown history command '{command}'")),
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