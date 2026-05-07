use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::ai_manager::{resolve_ai_runtime_config, AiManager, AiRuntimeConfig};
use crate::managers::workspace_manager::WorkspaceManager;

/// 處理 `ai.*` 指令：chat、clear_history、get_history。
pub struct AiHandler {
    manager: Arc<AiManager>,
    config: Arc<Mutex<ConfigManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

impl AiHandler {
    pub fn new(
        manager: Arc<AiManager>,
        config: Arc<Mutex<ConfigManager>>,
        workspace_manager: Arc<Mutex<WorkspaceManager>>,
    ) -> Self {
        Self {
            manager,
            config,
            workspace_manager,
        }
    }

    fn get_ai_config(&self) -> Result<AiRuntimeConfig, String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        resolve_ai_runtime_config(|key| cfg.get(key))
    }
}

impl CommandHandler for AiHandler {
    fn namespace(&self) -> &'static str {
        "ai"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "chat" => {
                let request_id = payload
                    .get("request_id")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
                    .to_string();
                let prompt = payload
                    .get("prompt")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'prompt'".to_string())?
                    .to_string();

                let ai_config = self.get_ai_config()?;
                self.manager.chat_async(
                    request_id.clone(),
                    prompt,
                    ai_config.provider,
                    ai_config.max_tokens,
                    ai_config.timeout_secs,
                );
                if let Ok(mut workspace) = self.workspace_manager.lock() {
                    workspace.record_ai_conversation(request_id.clone());
                }
                Ok(json!({ "status": "pending", "request_id": request_id }))
            }
            "clear_history" => {
                self.manager.clear_history();
                Ok(json!({ "ok": true }))
            }
            "get_history" => Ok(json!(self.manager.get_history())),
            _ => Err(format!("unknown ai command '{command}'")),
        }
    }
}
