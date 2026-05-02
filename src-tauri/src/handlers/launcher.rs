use crate::core::{CommandHandler, CommandResult};
use crate::managers::app_manager::AppManager;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// 應用程式啟動器的 IPC 處理器。
pub struct LauncherHandler {
    manager: Arc<Mutex<AppManager>>,
}

impl LauncherHandler {
    pub fn new(manager: Arc<Mutex<AppManager>>) -> Self {
        Self { manager }
    }
}

impl CommandHandler for LauncherHandler {
    fn namespace(&self) -> &'static str {
        "launcher"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "list_all" => {
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let apps = mgr.list_all();
                Ok(serde_json::to_value(apps).map_err(|e| e.to_string())?)
            }
            "search" => {
                let query = payload["query"]
                    .as_str()
                    .ok_or("missing field: query")?
                    .to_string();
                let mgr = self.manager.lock().map_err(|e| e.to_string())?;
                let results = mgr.search_apps(&query);
                Ok(serde_json::to_value(results).map_err(|e| e.to_string())?)
            }
            "launch" => {
                let path = payload["path"]
                    .as_str()
                    .ok_or("missing field: path")?
                    .to_string();
                let mut mgr = self.manager.lock().map_err(|e| e.to_string())?;
                mgr.launch_app(&path)?;
                Ok(Value::Null)
            }
            _ => Err(format!("launcher: unknown command '{command}'")),
        }
    }
}
