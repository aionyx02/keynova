use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::core::{CommandHandler, CommandResult};
use crate::managers::search_manager::SearchManager;

/// 統一搜尋 IPC 處理器（App + 檔案）。
pub struct SearchHandler {
    manager: Arc<Mutex<SearchManager>>,
}

impl SearchHandler {
    pub fn new(manager: Arc<Mutex<SearchManager>>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for SearchHandler {
    fn namespace(&self) -> &'static str {
        "search"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "query" => {
                let query = payload["query"]
                    .as_str()
                    .ok_or("missing field: query")?
                    .to_string();
                let limit = payload["limit"].as_u64().unwrap_or(10) as usize;
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let results = mgr.search(&query, limit);
                Ok(serde_json::to_value(results).map_err(|e| e.to_string())?)
            }
            "backend" => {
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                Ok(Value::String(mgr.active_backend_name().to_string()))
            }
            _ => Err(format!("search: unknown command '{command}'")),
        }
    }
}
