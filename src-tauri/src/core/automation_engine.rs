#![allow(dead_code)]

use serde_json::json;

use crate::models::workflow::{WorkflowDefinition, WorkflowExecutionLog};

pub struct AutomationEngine;

impl AutomationEngine {
    pub fn parse_toml(input: &str) -> Result<WorkflowDefinition, String> {
        toml::from_str::<WorkflowDefinition>(input).map_err(|e| e.to_string())
    }

    pub fn validate(definition: &WorkflowDefinition) -> Result<(), String> {
        if definition.name.trim().is_empty() {
            return Err("workflow name is required".to_string());
        }
        if definition.actions.is_empty() {
            return Err("workflow must contain at least one action".to_string());
        }
        for action in &definition.actions {
            if !action.route.contains('.') {
                return Err(format!("invalid action route '{}'", action.route));
            }
        }
        Ok(())
    }

    pub fn dry_run(definition: &WorkflowDefinition) -> Result<WorkflowExecutionLog, String> {
        Self::validate(definition)?;
        Ok(WorkflowExecutionLog {
            workflow_name: definition.name.clone(),
            status: "validated".into(),
            action_count: definition.actions.len(),
            error: None,
        })
    }

    pub fn error_payload(error: impl Into<String>) -> serde_json::Value {
        json!({
            "ok": false,
            "error": error.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_workflow_definition() {
        let workflow = AutomationEngine::parse_toml(
            r#"
            name = "open settings"
            version = 1

            [[actions]]
            route = "cmd.run"
            payload = { name = "setting", args = "" }
            "#,
        )
        .unwrap();
        assert_eq!(workflow.name, "open settings");
        assert_eq!(
            AutomationEngine::dry_run(&workflow).unwrap().action_count,
            1
        );
    }
}
