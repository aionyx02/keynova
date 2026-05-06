use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::core::AppEvent;
use crate::models::action::ActionRisk;
use crate::models::agent::{
    AgentRun, AgentRunStatus, AgentStep, AgentToolCall, ContextVisibility, GroundingSource,
};

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

    pub fn start(&self, prompt: String) -> Result<AgentRun, String> {
        self.start_with_context(prompt, Vec::new(), Vec::new())
    }

    pub fn start_with_context(
        &self,
        prompt: String,
        mut sources: Vec<GroundingSource>,
        tool_calls: Vec<AgentToolCall>,
    ) -> Result<AgentRun, String> {
        let run_id = Uuid::new_v4().to_string();
        let plan = plan_for_prompt(&prompt);
        if sources.is_empty() {
            sources.push(GroundingSource {
                source_id: "workspace:current".into(),
                source_type: "workspace".into(),
                title: "Current workspace".into(),
                snippet: "Workspace summary only; private architecture and secrets are withheld."
                    .into(),
                uri: None,
                score: 1.0,
                visibility: ContextVisibility::PublicContext,
                redacted_reason: None,
            });
        }
        let source_summary = sources
            .iter()
            .take(5)
            .map(|source| {
                format!(
                    "- [{}] {}: {}",
                    source.source_type, source.title, source.snippet
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let run = AgentRun {
            id: run_id.clone(),
            prompt,
            status: AgentRunStatus::Completed,
            plan: plan.clone(),
            steps: vec![AgentStep {
                id: format!("{run_id}:plan"),
                title: "Prepare safe plan".into(),
                status: "completed".into(),
                tool_calls,
            }],
            approvals: Vec::new(),
            memory_refs: Vec::new(),
            sources,
            output: Some(format!(
                "Plan prepared. No local actions were executed.\n{}\n\nSources considered:\n{}",
                plan.iter()
                    .map(|step| format!("- {step}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                source_summary
            )),
            error: None,
        };
        self.publish("agent.run.started", &run);
        self.publish("agent.step.started", &run);
        self.publish("agent.run.completed", &run);
        let mut guard = self.runs.lock().map_err(|e| e.to_string())?;
        guard.insert(run_id, run.clone());
        Ok(run)
    }

    pub fn cancel(&self, run_id: &str) -> Result<AgentRun, String> {
        let mut guard = self.runs.lock().map_err(|e| e.to_string())?;
        let run = guard
            .get_mut(run_id)
            .ok_or_else(|| format!("agent run '{run_id}' not found"))?;
        run.status = AgentRunStatus::Cancelled;
        self.publish("agent.run.failed", run);
        Ok(run.clone())
    }

    pub fn get(&self, run_id: &str) -> Result<Option<AgentRun>, String> {
        let guard = self.runs.lock().map_err(|e| e.to_string())?;
        Ok(guard.get(run_id).cloned())
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

fn plan_for_prompt(prompt: &str) -> Vec<String> {
    let trimmed = prompt.trim();
    let mut plan = vec![
        "Classify requested context by visibility before building any prompt".into(),
        "Use read-only tools first and avoid shell, filesystem, clipboard, or system control"
            .into(),
    ];
    if trimmed.contains("note") || trimmed.contains("筆記") {
        plan.push("Prepare a note draft and request approval before writing".into());
    } else if trimmed.contains("test") || trimmed.contains("測試") {
        plan.push("Show the test command and wait for approval before starting a process".into());
    } else {
        plan.push("Return a suggested next-step plan without executing local actions".into());
    }
    plan
}
