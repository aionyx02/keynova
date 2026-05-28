//! REF.5 — IPC namespace `workflow.*` for the workflow memory layer.
//!
//! - `workflow.recent` — top-N regardless of context (most recently used).
//! - `workflow.suggest` — top-N filtered by the current workspace's coarse
//!   `context_hash`. The handler computes the hash itself so the frontend
//!   never has to know the formula.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::core::knowledge_store::{KnowledgeStoreHandle, WorkflowHistoryRow};
use crate::core::workflow_memory;
use crate::core::{CommandHandler, CommandResult};
use crate::managers::workspace_manager::WorkspaceManager;

pub struct WorkflowMemoryHandler {
    store: KnowledgeStoreHandle,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

pub struct WorkflowMemoryHandlerDeps {
    pub store: KnowledgeStoreHandle,
    pub workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

impl WorkflowMemoryHandler {
    pub fn new(deps: WorkflowMemoryHandlerDeps) -> Self {
        Self {
            store: deps.store,
            workspace_manager: deps.workspace_manager,
        }
    }

    fn parse_limit(payload: &Value) -> usize {
        let requested = payload
            .get("limit")
            .and_then(Value::as_u64)
            .map(|n| n as usize);
        workflow_memory::clamp_limit(requested)
    }

    fn current_context_hash(&self) -> Option<String> {
        let workspace = self.workspace_manager.lock().ok()?;
        let current = workspace.current();
        Some(workflow_memory::compute_context_hash(
            current.id as i64,
            &current.mode,
            current.panel.as_deref(),
        ))
    }

    fn recent(&self, payload: Value) -> CommandResult {
        let limit = Self::parse_limit(&payload);
        let rows = workflow_memory::suggest(&self.store, None, limit)?;
        Ok(rows_to_json(rows))
    }

    fn suggest(&self, payload: Value) -> CommandResult {
        let limit = Self::parse_limit(&payload);
        let context_hash = self.current_context_hash();
        let rows = workflow_memory::suggest(&self.store, context_hash.as_deref(), limit)?;
        Ok(rows_to_json(rows))
    }
}

fn rows_to_json(rows: Vec<WorkflowHistoryRow>) -> Value {
    json!({ "rows": rows })
}

impl CommandHandler for WorkflowMemoryHandler {
    fn namespace(&self) -> &'static str {
        "workflow"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "recent" => self.recent(payload),
            "suggest" => self.suggest(payload),
            _ => Err(format!("unknown workflow command '{command}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_limit_defaults_when_missing() {
        let payload = json!({});
        assert_eq!(
            WorkflowMemoryHandler::parse_limit(&payload),
            workflow_memory::DEFAULT_LIMIT
        );
    }

    #[test]
    fn parse_limit_caps_at_max() {
        let payload = json!({ "limit": 9_999 });
        assert_eq!(
            WorkflowMemoryHandler::parse_limit(&payload),
            workflow_memory::MAX_LIMIT
        );
    }

    #[test]
    fn parse_limit_zero_becomes_one() {
        let payload = json!({ "limit": 0 });
        assert_eq!(WorkflowMemoryHandler::parse_limit(&payload), 1);
    }
}
