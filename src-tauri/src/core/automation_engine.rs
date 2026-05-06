#![allow(dead_code)]

use serde_json::{json, Value};

use crate::models::workflow::{
    WorkflowActionExecution, WorkflowDefinition, WorkflowExecutionLog, WorkflowExecutionReport,
};

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

    pub fn execute<F>(definition: &WorkflowDefinition, mut run_action: F) -> WorkflowExecutionReport
    where
        F: FnMut(&str, Value) -> Result<Value, String>,
    {
        if let Err(error) = Self::validate(definition) {
            return Self::report(definition, "invalid", Some(error), Vec::new());
        }

        let mut actions = Vec::new();
        for (index, action) in definition.actions.iter().enumerate() {
            if action.route == "automation.execute" {
                let error = "recursive automation.execute is not allowed".to_string();
                actions.push(WorkflowActionExecution {
                    index,
                    route: action.route.clone(),
                    status: "failed".into(),
                    output: None,
                    error: Some(error.clone()),
                });
                return Self::report(definition, "failed", Some(error), actions);
            }

            match run_action(&action.route, action.payload.clone()) {
                Ok(output) => actions.push(WorkflowActionExecution {
                    index,
                    route: action.route.clone(),
                    status: "completed".into(),
                    output: Some(output),
                    error: None,
                }),
                Err(error) => {
                    actions.push(WorkflowActionExecution {
                        index,
                        route: action.route.clone(),
                        status: "failed".into(),
                        output: None,
                        error: Some(error.clone()),
                    });
                    return Self::report(definition, "failed", Some(error), actions);
                }
            }
        }

        Self::report(definition, "completed", None, actions)
    }

    pub fn error_payload(error: impl Into<String>) -> serde_json::Value {
        json!({
            "ok": false,
            "error": error.into(),
        })
    }

    fn report(
        definition: &WorkflowDefinition,
        status: &str,
        error: Option<String>,
        actions: Vec<WorkflowActionExecution>,
    ) -> WorkflowExecutionReport {
        WorkflowExecutionReport {
            log: WorkflowExecutionLog {
                workflow_name: definition.name.clone(),
                status: status.into(),
                action_count: definition.actions.len(),
                error,
            },
            actions,
        }
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

    #[test]
    fn executes_actions_in_order() {
        let workflow = AutomationEngine::parse_toml(
            r#"
            name = "chain"

            [[actions]]
            route = "cmd.run"
            payload = { name = "setting", args = "" }

            [[actions]]
            route = "setting.list_all"
            "#,
        )
        .unwrap();
        let mut routes = Vec::new();
        let report = AutomationEngine::execute(&workflow, |route, payload| {
            routes.push(route.to_string());
            Ok(json!({ "route": route, "payload": payload }))
        });

        assert_eq!(report.log.status, "completed");
        assert_eq!(routes, vec!["cmd.run", "setting.list_all"]);
        assert_eq!(report.actions.len(), 2);
        assert!(report.actions.iter().all(|action| action.error.is_none()));
    }

    #[test]
    fn stops_on_first_failure() {
        let workflow = AutomationEngine::parse_toml(
            r#"
            name = "chain"

            [[actions]]
            route = "cmd.run"

            [[actions]]
            route = "setting.list_all"
            "#,
        )
        .unwrap();
        let mut routes = Vec::new();
        let report = AutomationEngine::execute(&workflow, |route, _payload| {
            routes.push(route.to_string());
            Err("boom".into())
        });

        assert_eq!(report.log.status, "failed");
        assert_eq!(report.log.error.as_deref(), Some("boom"));
        assert_eq!(routes, vec!["cmd.run"]);
        assert_eq!(report.actions.len(), 1);
        assert_eq!(report.actions[0].status, "failed");
    }

    #[test]
    fn rejects_recursive_automation_execute() {
        let workflow = AutomationEngine::parse_toml(
            r#"
            name = "recursive"

            [[actions]]
            route = "automation.execute"
            "#,
        )
        .unwrap();
        let report = AutomationEngine::execute(&workflow, |_route, _payload| {
            panic!("recursive route must be rejected before dispatch")
        });

        assert_eq!(report.log.status, "failed");
        assert_eq!(
            report.log.error.as_deref(),
            Some("recursive automation.execute is not allowed")
        );
        assert_eq!(report.actions.len(), 1);
    }
}
