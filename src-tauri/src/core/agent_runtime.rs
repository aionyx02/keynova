use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::core::AppEvent;
use crate::models::action::ActionRisk;
use crate::models::agent::{AgentRun, AgentRunStatus, ContextVisibility};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolSpec {
    pub name: String,
    pub risk: ActionRisk,
    pub allowed_visibility: Vec<ContextVisibility>,
    pub timeout_ms: u64,
    pub result_limit: usize,
}

#[derive(Default)]
pub struct AgentToolRegistry {
    tools: HashMap<String, AgentToolSpec>,
}

impl AgentToolRegistry {
    pub fn with_default_readonly_tools() -> Self {
        let mut registry = Self::default();
        registry.register(AgentToolSpec {
            name: "web.search".into(),
            risk: ActionRisk::Low,
            allowed_visibility: vec![
                ContextVisibility::PublicContext,
                ContextVisibility::UserPrivate,
            ],
            timeout_ms: 5000,
            result_limit: 5,
        });
        registry.register(AgentToolSpec {
            name: "keynova.search".into(),
            risk: ActionRisk::Low,
            allowed_visibility: vec![
                ContextVisibility::PublicContext,
                ContextVisibility::UserPrivate,
            ],
            timeout_ms: 1000,
            result_limit: 10,
        });
        registry
    }

    pub fn register(&mut self, spec: AgentToolSpec) {
        self.tools.insert(spec.name.clone(), spec);
    }

    pub fn list(&self) -> Vec<AgentToolSpec> {
        let mut tools: Vec<_> = self.tools.values().cloned().collect();
        tools.sort_by(|left, right| left.name.cmp(&right.name));
        tools
    }
}

pub struct AgentRuntime {
    runs: Mutex<HashMap<String, AgentRun>>,
    tools: AgentToolRegistry,
    publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
}

impl AgentRuntime {
    pub fn new(publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>) -> Self {
        Self {
            runs: Mutex::new(HashMap::new()),
            tools: AgentToolRegistry::with_default_readonly_tools(),
            publish_event,
        }
    }

    pub fn next_run_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn insert_run(&self, run: AgentRun) -> Result<AgentRun, String> {
        let run_id = run.id.clone();
        let mut guard = self.runs.lock().map_err(|e| e.to_string())?;
        guard.insert(run_id, run.clone());
        drop(guard);
        self.publish("agent.run.started", &run);
        if run.status == AgentRunStatus::WaitingApproval {
            self.publish("agent.approval.required", &run);
        } else if matches!(run.status, AgentRunStatus::Completed | AgentRunStatus::Failed) {
            self.publish("agent.run.completed", &run);
        }
        Ok(run)
    }

    pub fn update_run(&self, run: AgentRun, topic: &str) -> Result<AgentRun, String> {
        let run_id = run.id.clone();
        let mut guard = self.runs.lock().map_err(|e| e.to_string())?;
        guard.insert(run_id, run.clone());
        drop(guard);
        self.publish(topic, &run);
        Ok(run)
    }

    pub fn cancel(&self, run_id: &str) -> Result<AgentRun, String> {
        let mut guard = self.runs.lock().map_err(|e| e.to_string())?;
        let run = guard
            .get_mut(run_id)
            .ok_or_else(|| format!("agent run '{run_id}' not found"))?;
        run.status = AgentRunStatus::Cancelled;
        run.output = Some("Run cancelled before execution completed.".into());
        let updated = run.clone();
        drop(guard);
        self.publish("agent.run.failed", &updated);
        Ok(updated)
    }

    pub fn get(&self, run_id: &str) -> Result<Option<AgentRun>, String> {
        let guard = self.runs.lock().map_err(|e| e.to_string())?;
        Ok(guard.get(run_id).cloned())
    }

    pub fn recent_runs(&self, limit: usize) -> Result<Vec<AgentRun>, String> {
        let guard = self.runs.lock().map_err(|e| e.to_string())?;
        let mut runs: Vec<_> = guard.values().cloned().collect();
        runs.sort_by(|left, right| right.id.cmp(&left.id));
        runs.truncate(limit);
        Ok(runs)
    }

    pub fn list_tools(&self) -> Vec<AgentToolSpec> {
        self.tools.list()
    }

    fn publish(&self, topic: &str, run: &AgentRun) {
        (self.publish_event)(AppEvent::new(
            topic,
            json!({
                "run_id": run.id,
                "status": run.status,
                "run": run,
            }),
        ));
    }
}
