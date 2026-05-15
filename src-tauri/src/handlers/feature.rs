use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::{CommandHandler, CommandResult};
use crate::managers::terminal_manager::{start_prewarm, TerminalManager};

/// Handles `feature.activate` — triggers lazy service initialization for a
/// named feature key. This is the backend counterpart to the frontend
/// `FeatureContext.activate()` call. Each activation is idempotent at the
/// service level (e.g. `start_prewarm` checks for an existing warm session).
pub struct FeatureHandler {
    terminal_manager: Arc<Mutex<TerminalManager>>,
}

impl FeatureHandler {
    pub fn new(terminal_manager: Arc<Mutex<TerminalManager>>) -> Self {
        Self { terminal_manager }
    }
}

impl CommandHandler for FeatureHandler {
    fn namespace(&self) -> &'static str {
        "feature"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "activate" => {
                let key = payload
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                match key.as_str() {
                    "terminal" => {
                        // Deferred from bootstrap — prewarm starts on first panel open.
                        start_prewarm(Arc::clone(&self.terminal_manager));
                    }
                    "ai" | "agent" | "notes" | "nvim" | "system_monitor" => {
                        // These services initialize on demand; activation is a no-op here.
                    }
                    other => {
                        return Err(format!("unknown feature key '{other}'"));
                    }
                }
                Ok(json!({ "ok": true, "key": key }))
            }
            _ => Err(format!("unknown feature command '{command}'")),
        }
    }
}