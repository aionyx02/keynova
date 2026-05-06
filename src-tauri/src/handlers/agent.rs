use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::{AgentRuntime, CommandHandler, CommandResult};

/// Handles local agent runtime lifecycle commands.
pub struct AgentHandler {
    runtime: Arc<AgentRuntime>,
}

impl AgentHandler {
    pub fn new(runtime: Arc<AgentRuntime>) -> Self {
        Self { runtime }
    }
}

impl CommandHandler for AgentHandler {
    fn namespace(&self) -> &'static str {
        "agent"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "start" => {
                let prompt = payload
                    .get("prompt")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "missing 'prompt'".to_string())?
                    .to_string();
                Ok(json!(self.runtime.start(prompt)?))
            }
            "cancel" => {
                let run_id = require_str(&payload, "run_id")?;
                Ok(json!(self.runtime.cancel(run_id)?))
            }
            "resume" => {
                let run_id = require_str(&payload, "run_id")?;
                Ok(json!({
                    "ok": true,
                    "run": self.runtime.get(run_id)?,
                    "status": "resume_not_required"
                }))
            }
            "approve" => {
                let run_id = require_str(&payload, "run_id")?;
                Ok(json!({
                    "ok": true,
                    "run_id": run_id,
                    "status": "approval_recorded"
                }))
            }
            "reject" => {
                let run_id = require_str(&payload, "run_id")?;
                Ok(json!({
                    "ok": true,
                    "run_id": run_id,
                    "status": "approval_rejected"
                }))
            }
            "tools" => Ok(json!(self.runtime.list_tools())),
            _ => Err(format!("unknown agent command '{command}'")),
        }
    }
}

fn require_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str, String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing or empty '{key}'"))
}
