use serde_json::{json, Value};

use crate::core::automation_engine::AutomationEngine;
use crate::core::{CommandHandler, CommandResult};

/// Handles workflow parsing and dry-run validation.
pub struct AutomationHandler;

impl CommandHandler for AutomationHandler {
    fn namespace(&self) -> &'static str {
        "automation"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "parse" => {
                let source = require_str(&payload, "source")?;
                let workflow = AutomationEngine::parse_toml(source)?;
                Ok(json!(workflow))
            }
            "dry_run" => {
                let source = require_str(&payload, "source")?;
                let workflow = AutomationEngine::parse_toml(source)?;
                Ok(json!(AutomationEngine::dry_run(&workflow)?))
            }
            _ => Err(format!("unknown automation command '{command}'")),
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
