use serde_json::{json, Value};

use crate::core::{CommandHandler, CommandResult};

/// Handles `feature.activate` lazy-initialization hints from the frontend.
///
/// Activation is intentionally side-effect-light. In particular, terminal
/// activation no longer prewarms a shell because terminal sessions now require
/// backend-issued launch specs.
pub struct FeatureHandler;

impl FeatureHandler {
    pub fn new() -> Self {
        Self
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
                    "terminal" | "ai" | "agent" | "notes" | "nvim" | "system_monitor" => {}
                    other => return Err(format!("unknown feature key '{other}'")),
                }
                Ok(json!({ "ok": true, "key": key }))
            }
            _ => Err(format!("unknown feature command '{command}'")),
        }
    }
}
