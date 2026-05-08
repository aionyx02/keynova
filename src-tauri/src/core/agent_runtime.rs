use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::{AgentObservationPolicy, AppEvent};
use crate::models::action::ActionRisk;
use crate::models::agent::{AgentRun, AgentRunStatus, ContextVisibility};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolApprovalPolicy {
    Never,
    Required,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolSpec {
    pub name: String,
    pub llm_name: String,
    pub description: String,
    pub risk: ActionRisk,
    pub approval_policy: AgentToolApprovalPolicy,
    pub allowed_visibility: Vec<ContextVisibility>,
    pub timeout_ms: u64,
    pub result_limit: usize,
    pub parameters_schema: Value,
    pub result_schema: Value,
    pub observation_policy: AgentObservationPolicy,
}

impl AgentToolSpec {
    pub fn openai_tool_schema(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.llm_name,
                "description": self.description,
                "parameters": self.parameters_schema,
            }
        })
    }
}

#[derive(Default)]
pub struct AgentToolRegistry {
    tools: HashMap<String, AgentToolSpec>,
}

impl AgentToolRegistry {
    pub fn with_default_readonly_tools() -> Self {
        let mut registry = Self::default();
        registry.register(tool_spec::<WebSearchToolParams, ToolSourcesResult>(
            "web.search",
            "Search the public web using a structured web-search provider. Never include private project files, secrets, or local-only context in the query.",
            ActionRisk::Low,
            AgentToolApprovalPolicy::Never,
            vec![
                ContextVisibility::PublicContext,
                ContextVisibility::UserPrivate,
            ],
            5000,
            5,
        ));
        registry.register(tool_spec::<KeynovaSearchToolParams, ToolSourcesResult>(
            "keynova.search",
            "Search Keynova commands, settings, workspace metadata, notes, models, and history visible to the agent.",
            ActionRisk::Low,
            AgentToolApprovalPolicy::Never,
            vec![
                ContextVisibility::PublicContext,
                ContextVisibility::UserPrivate,
            ],
            1000,
            10,
        ));
        registry.register(tool_spec::<FilesystemSearchToolParams, ToolSourcesResult>(
            "filesystem.search",
            "Search local filesystem paths through indexed providers first, returning bounded metadata only.",
            ActionRisk::Low,
            AgentToolApprovalPolicy::Never,
            vec![ContextVisibility::UserPrivate],
            3500,
            20,
        ));
        registry.register(tool_spec::<FilesystemReadToolParams, ToolSourcesResult>(
            "filesystem.read",
            "Read a bounded text preview from a user-private file without modifying it.",
            ActionRisk::Low,
            AgentToolApprovalPolicy::Never,
            vec![ContextVisibility::UserPrivate],
            1000,
            1,
        ));
        registry.register(tool_spec::<GitStatusToolParams, GitStatusToolResult>(
            "git.status",
            "Run a fixed read-only git status command in an approved workspace. This is a typed tool, not a generic shell.",
            ActionRisk::Medium,
            AgentToolApprovalPolicy::Required,
            vec![ContextVisibility::UserPrivate],
            3000,
            1,
        ));
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

fn tool_spec<P, R>(
    name: &str,
    description: &str,
    risk: ActionRisk,
    approval_policy: AgentToolApprovalPolicy,
    allowed_visibility: Vec<ContextVisibility>,
    timeout_ms: u64,
    result_limit: usize,
) -> AgentToolSpec
where
    P: JsonSchema,
    R: JsonSchema,
{
    AgentToolSpec {
        name: name.to_string(),
        llm_name: llm_tool_name(name),
        description: description.to_string(),
        risk,
        approval_policy,
        allowed_visibility,
        timeout_ms,
        result_limit,
        parameters_schema: schema_value::<P>(),
        result_schema: schema_value::<R>(),
        observation_policy: AgentObservationPolicy::default(),
    }
}

fn schema_value<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("schemars schema should serialize")
}

fn llm_tool_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct KeynovaSearchToolParams {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct WebSearchToolParams {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FilesystemSearchToolParams {
    pub query: String,
    pub roots: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FilesystemReadToolParams {
    pub path: String,
    pub max_chars: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GitStatusToolParams {
    pub cwd: Option<String>,
    pub include_untracked: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ToolSourcesResult {
    pub sources: Vec<ToolSourceResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ToolSourceResult {
    pub title: String,
    pub snippet: String,
    pub uri: Option<String>,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GitStatusToolResult {
    pub cwd: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
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
        } else if matches!(
            run.status,
            AgentRunStatus::Completed | AgentRunStatus::Failed
        ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_run(id: &str, status: AgentRunStatus) -> AgentRun {
        AgentRun {
            id: id.into(),
            prompt: "test prompt".into(),
            status,
            plan: Vec::new(),
            steps: Vec::new(),
            approvals: Vec::new(),
            memory_refs: Vec::new(),
            sources: Vec::new(),
            prompt_audit: None,
            command_result: None,
            output: None,
            error: None,
        }
    }

    #[test]
    fn cancel_updates_run_and_publishes_failed_topic() {
        let events = Arc::new(Mutex::new(Vec::<String>::new()));
        let sink = events.clone();
        let runtime = AgentRuntime::new(Arc::new(move |event| {
            sink.lock()
                .expect("events mutex poisoned")
                .push(event.topic);
        }));

        runtime
            .insert_run(test_run("run-1", AgentRunStatus::WaitingApproval))
            .expect("insert run");
        let cancelled = runtime.cancel("run-1").expect("cancel run");

        assert_eq!(cancelled.status, AgentRunStatus::Cancelled);
        assert_eq!(
            runtime
                .get("run-1")
                .expect("get run")
                .expect("run exists")
                .status,
            AgentRunStatus::Cancelled
        );
        assert_eq!(
            events.lock().expect("events mutex poisoned").as_slice(),
            &[
                "agent.run.started".to_string(),
                "agent.approval.required".to_string(),
                "agent.run.failed".to_string(),
            ]
        );
    }

    #[test]
    fn default_tool_registry_is_sorted_and_read_only() {
        let runtime = AgentRuntime::new(Arc::new(|_| {}));
        let tools = runtime.list_tools();

        assert_eq!(tools.len(), 5);
        assert_eq!(tools[0].name, "filesystem.read");
        assert_eq!(tools[1].name, "filesystem.search");
        assert_eq!(tools[2].name, "git.status");
        assert_eq!(tools[3].name, "keynova.search");
        assert_eq!(tools[4].name, "web.search");
        assert!(tools
            .iter()
            .all(|tool| tool.name != "execute_shell_command"));
        assert!(tools.iter().all(|tool| tool.name != "execute_bash_command"));
        assert_eq!(tools[2].approval_policy, AgentToolApprovalPolicy::Required);
    }

    #[test]
    fn tool_schemas_are_generated_and_openai_safe() {
        let runtime = AgentRuntime::new(Arc::new(|_| {}));
        let tools = runtime.list_tools();
        let filesystem = tools
            .iter()
            .find(|tool| tool.name == "filesystem.read")
            .expect("filesystem.read tool");

        assert_eq!(filesystem.llm_name, "filesystem_read");
        assert_eq!(
            filesystem.parameters_schema["properties"]["path"]["type"],
            "string"
        );
        let openai_schema = filesystem.openai_tool_schema();
        assert_eq!(openai_schema["type"], "function");
        assert_eq!(
            openai_schema["function"]["name"],
            filesystem.llm_name.as_str()
        );
    }
}
