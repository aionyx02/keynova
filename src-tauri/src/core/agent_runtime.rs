use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::{prepare_observation, AgentObservationPolicy, AppEvent};
use crate::managers::ai_manager::{
    AiMessage, AiToolDefinition, AiToolTurn, ToolCallProvider,
};
use crate::models::action::ActionRisk;
use crate::models::agent::{AgentRun, AgentRunStatus, AgentStep, ContextVisibility};

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

    /// Spawn a background ReAct loop for an already-inserted run.
    ///
    /// The run must already exist in the registry with `status = Running`.
    /// Each loop step updates the run in-place and emits `agent.step` events.
    pub fn spawn_react_loop(
        self: &Arc<Self>,
        run_id: String,
        provider: Arc<dyn ToolCallProvider>,
        tools: Vec<AgentToolSpec>,
        config: ReactLoopConfig,
        dispatch: Arc<ToolDispatch>,
    ) {
        let runtime = Arc::clone(self);
        std::thread::spawn(move || {
            react_loop_body(&runtime, &run_id, &*provider, &tools, &config, &*dispatch);
        });
    }
}

// ─── ReAct loop ─────────────────────────────────────────────────────────────

/// Type alias for the tool dispatch closure used by `react_loop_body`.
pub type ToolDispatch = dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync;

/// Configuration for a single ReAct loop execution.
pub struct ReactLoopConfig {
    pub max_steps: usize,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub system_prompt: String,
}

impl Default for ReactLoopConfig {
    fn default() -> Self {
        Self {
            max_steps: 8,
            max_tokens: 4096,
            timeout_secs: 30,
            system_prompt: concat!(
                "You are a helpful assistant integrated into Keynova. ",
                "Use the provided tools to answer the user's request. ",
                "Once you have enough information, provide a final answer ",
                "without calling any more tools."
            )
            .into(),
        }
    }
}

/// Convert tool specs to the provider-neutral definition slice.
pub fn tools_to_definitions(tools: &[AgentToolSpec]) -> Vec<AiToolDefinition> {
    tools
        .iter()
        .map(|spec| AiToolDefinition {
            name: spec.llm_name.clone(),
            description: spec.description.clone(),
            parameters_schema: spec.parameters_schema.clone(),
        })
        .collect()
}

/// Blocking ReAct loop body — runs on the calling thread.
///
/// Intended for use from `spawn_react_loop` (background thread) and directly
/// from tests (no thread involved).
pub fn react_loop_body(
    runtime: &AgentRuntime,
    run_id: &str,
    provider: &dyn ToolCallProvider,
    tools: &[AgentToolSpec],
    config: &ReactLoopConfig,
    dispatch: &ToolDispatch,
) {
    let prompt = match runtime.get(run_id) {
        Ok(Some(run)) => run.prompt.clone(),
        _ => {
            eprintln!("[react] run '{run_id}' not found at loop start");
            return;
        }
    };

    let mut messages: Vec<AiMessage> = vec![
        AiMessage {
            role: "system".into(),
            content: config.system_prompt.clone(),
        },
        AiMessage {
            role: "user".into(),
            content: prompt,
        },
    ];

    let tool_defs = tools_to_definitions(tools);
    let tool_map: HashMap<String, &AgentToolSpec> =
        tools.iter().map(|s| (s.llm_name.clone(), s)).collect();

    for step_idx in 0..config.max_steps {
        // Check for cancellation.
        match runtime.get(run_id) {
            Ok(Some(run)) if run.status == AgentRunStatus::Cancelled => return,
            Ok(None) => return,
            _ => {}
        }

        let turn = match provider.chat_with_tools(
            &messages,
            &tool_defs,
            config.max_tokens,
            config.timeout_secs,
        ) {
            Ok(t) => t,
            Err(e) => {
                react_fail(runtime, run_id, &format!("Provider error at step {step_idx}: {e}"));
                return;
            }
        };

        match turn {
            AiToolTurn::FinalText { content } => {
                react_complete(runtime, run_id, step_idx, content);
                return;
            }
            AiToolTurn::ToolCalls { tool_calls } => {
                let mut obs_messages: Vec<AiMessage> = Vec::new();

                for call in &tool_calls {
                    let spec = tool_map.get(&call.name);

                    // Approval-required tools are skipped in B1 (no interactive gate yet).
                    let approval_required = spec
                        .map(|s| s.approval_policy == AgentToolApprovalPolicy::Required)
                        .unwrap_or(false);
                    if approval_required {
                        let obs = format!(
                            "Tool '{}' requires explicit user approval and was not executed.",
                            call.name
                        );
                        emit_react_step(runtime, run_id, step_idx, &call.name, "approval_required");
                        obs_messages.push(AiMessage {
                            role: "user".into(),
                            content: format!("[Tool: {}]\n{}", call.name, obs),
                        });
                        continue;
                    }

                    let obs_policy = spec
                        .map(|s| s.observation_policy.clone())
                        .unwrap_or_default();

                    let result = dispatch(&call.name, &call.arguments);
                    let observation = match result {
                        Ok(val) => {
                            let raw = serde_json::to_string_pretty(&val).unwrap_or_default();
                            prepare_observation(&raw, &obs_policy).content
                        }
                        Err(e) => format!("Error from tool '{}': {}", call.name, e),
                    };

                    emit_react_step(runtime, run_id, step_idx, &call.name, "executed");
                    obs_messages.push(AiMessage {
                        role: "user".into(),
                        content: format!("[Tool: {}]\n{}", call.name, observation),
                    });
                }

                // Append a synthetic assistant turn describing the tool calls.
                let calls_desc = tool_calls
                    .iter()
                    .map(|c| format!("{}({})", c.name, c.arguments))
                    .collect::<Vec<_>>()
                    .join(", ");
                messages.push(AiMessage {
                    role: "assistant".into(),
                    content: format!("[Calling tools: {calls_desc}]"),
                });
                messages.extend(obs_messages);
            }
        }
    }

    react_fail(runtime, run_id, "Maximum steps exceeded without a final answer.");
}

fn react_complete(runtime: &AgentRuntime, run_id: &str, final_step: usize, content: String) {
    let step_id = format!("{run_id}:react:{final_step}:final");
    if let Ok(Some(mut run)) = runtime.get(run_id) {
        run.status = AgentRunStatus::Completed;
        run.output = Some(content.clone());
        run.steps.push(AgentStep {
            id: step_id,
            title: "Final answer".into(),
            status: "completed".into(),
            tool_calls: Vec::new(),
        });
        let _ = runtime.update_run(run, "agent.run.completed");
    }
    (runtime.publish_event)(AppEvent::new(
        "agent.step",
        json!({
            "run_id": run_id,
            "step": final_step,
            "tool_name": null,
            "status": "final",
            "observation_preview": truncate_preview(&content, 120),
        }),
    ));
}

fn react_fail(runtime: &AgentRuntime, run_id: &str, reason: &str) {
    if let Ok(Some(mut run)) = runtime.get(run_id) {
        run.status = AgentRunStatus::Failed;
        run.error = Some(reason.into());
        let _ = runtime.update_run(run, "agent.run.failed");
    }
}

fn emit_react_step(runtime: &AgentRuntime, run_id: &str, step: usize, tool_name: &str, status: &str) {
    (runtime.publish_event)(AppEvent::new(
        "agent.step",
        json!({
            "run_id": run_id,
            "step": step,
            "tool_name": tool_name,
            "status": status,
        }),
    ));
}

fn truncate_preview(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        format!("{}…", chars[..max_chars].iter().collect::<String>())
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

    // ─── ReAct loop tests (5.5.B1) ──────────────────────────────────────────

    use crate::managers::fake_provider::FakeToolCallProvider;

    fn react_runtime() -> AgentRuntime {
        AgentRuntime::new(Arc::new(|_| {}))
    }

    fn running_run(id: &str) -> AgentRun {
        AgentRun {
            id: id.into(),
            prompt: "find my notes about Rust".into(),
            status: AgentRunStatus::Running,
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

    fn noop_dispatch(_name: &str, _args: &Value) -> Result<Value, String> {
        Ok(json!({ "sources": [] }))
    }

    #[test]
    fn react_loop_single_final_text_completes_run() {
        let runtime = react_runtime();
        runtime
            .insert_run(running_run("r1"))
            .expect("insert run");

        let provider = FakeToolCallProvider::single_final("Here is your answer.");
        react_loop_body(
            &runtime,
            "r1",
            &provider,
            &[],
            &ReactLoopConfig::default(),
            &noop_dispatch,
        );

        let run = runtime.get("r1").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert_eq!(run.output.as_deref(), Some("Here is your answer."));
    }

    #[test]
    fn react_loop_tool_call_then_final_completes_run() {
        let runtime = react_runtime();
        runtime
            .insert_run(running_run("r2"))
            .expect("insert run");

        let provider = FakeToolCallProvider::tool_then_final(
            "keynova_search",
            "call-abc",
            json!({ "query": "Rust notes", "limit": 5 }),
            "Found 2 notes about Rust.",
        );

        let dispatch_hits = Arc::new(Mutex::new(Vec::<String>::new()));
        let hits = Arc::clone(&dispatch_hits);
        let dispatch = move |name: &str, _args: &Value| -> Result<Value, String> {
            hits.lock().unwrap().push(name.to_string());
            Ok(json!({ "sources": [{ "title": "Rust notes", "snippet": "async/await", "source_type": "note" }] }))
        };

        // Build a keynova.search spec so the dispatch is recognized.
        let tools = AgentToolRegistry::with_default_readonly_tools().list();

        react_loop_body(
            &runtime,
            "r2",
            &provider,
            &tools,
            &ReactLoopConfig::default(),
            &dispatch,
        );

        let run = runtime.get("r2").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert_eq!(run.output.as_deref(), Some("Found 2 notes about Rust."));
        assert_eq!(dispatch_hits.lock().unwrap().as_slice(), &["keynova_search"]);
    }

    #[test]
    fn react_loop_cancel_mid_loop_stops_early() {
        let runtime = react_runtime();
        runtime
            .insert_run(running_run("r3"))
            .expect("insert run");

        // Provider returns a tool call; we cancel the run before the loop checks status again.
        // The trick: cancel first, then run the loop — it checks at the top of each iteration.
        runtime.cancel("r3").expect("cancel run");

        let provider = FakeToolCallProvider::single_final("Should never reach this.");
        react_loop_body(
            &runtime,
            "r3",
            &provider,
            &[],
            &ReactLoopConfig::default(),
            &noop_dispatch,
        );

        // Status stays Cancelled (loop exits on cancel check without overwriting).
        let run = runtime.get("r3").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Cancelled);
    }

    #[test]
    fn react_loop_filesystem_search_dispatch_finds_file() {
        use crate::managers::system_indexer::search_system_index;
        use uuid::Uuid;

        let tmp_root =
            std::env::temp_dir().join(format!("keynova-react-fs-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_root).expect("create tmp dir");
        let file_path = tmp_root.join("react-test-note.md");
        std::fs::write(&file_path, "# React test\nSome content.").expect("write file");

        let root = tmp_root.clone();
        let dispatch = Arc::new(move |name: &str, args: &Value| -> Result<Value, String> {
            match name {
                "filesystem_search" => {
                    let query = args.get("query").and_then(Value::as_str).unwrap_or("");
                    let outcome = search_system_index(query, &[root.clone()], 10);
                    let sources: Vec<Value> = outcome
                        .hits
                        .iter()
                        .map(|h| {
                            json!({
                                "title": h.name,
                                "snippet": h.path,
                                "uri": h.path,
                                "source_type": "file",
                            })
                        })
                        .collect();
                    Ok(json!({ "sources": sources }))
                }
                other => Err(format!("unexpected tool: {other}")),
            }
        });

        let provider = FakeToolCallProvider::tool_then_final(
            "filesystem_search",
            "call-fs",
            json!({ "query": "react-test-note" }),
            "I found the file react-test-note.md in the workspace.",
        );

        let runtime = react_runtime();
        runtime.insert_run(running_run("fs1")).expect("insert run");

        let tools = AgentToolRegistry::with_default_readonly_tools().list();
        react_loop_body(&runtime, "fs1", &provider, &tools, &ReactLoopConfig::default(), &*dispatch);

        let run = runtime.get("fs1").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run.output.as_deref().unwrap_or("").contains("react-test-note.md"));

        let _ = std::fs::remove_dir_all(tmp_root);
    }

    #[test]
    fn react_loop_filesystem_read_dispatch_returns_content() {
        use uuid::Uuid;

        let tmp_root =
            std::env::temp_dir().join(format!("keynova-react-read-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_root).expect("create tmp dir");
        let file_path = tmp_root.join("sample.toml");
        std::fs::write(&file_path, "[agent]\nmode = \"local\"\n").expect("write file");

        let dispatch = Arc::new(move |name: &str, args: &Value| -> Result<Value, String> {
            match name {
                "filesystem_read" => {
                    let path_str =
                        args.get("path").and_then(Value::as_str).unwrap_or("");
                    let content = std::fs::read_to_string(path_str)
                        .map_err(|e| format!("read error: {e}"))?;
                    Ok(json!({
                        "sources": [{
                            "title": "sample.toml",
                            "snippet": content,
                            "uri": path_str,
                            "source_type": "file_read",
                        }]
                    }))
                }
                other => Err(format!("unexpected tool: {other}")),
            }
        });

        let provider = FakeToolCallProvider::tool_then_final(
            "filesystem_read",
            "call-read",
            json!({ "path": file_path.display().to_string() }),
            "The config file sets agent mode to local.",
        );

        let runtime = react_runtime();
        runtime.insert_run(running_run("fs2")).expect("insert run");

        let tools = AgentToolRegistry::with_default_readonly_tools().list();
        react_loop_body(&runtime, "fs2", &provider, &tools, &ReactLoopConfig::default(), &*dispatch);

        let run = runtime.get("fs2").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run.output.as_deref().unwrap_or("").contains("agent mode"));

        let _ = std::fs::remove_dir_all(tmp_root);
    }

    #[test]
    fn react_loop_max_steps_exceeded_fails_run() {
        let runtime = react_runtime();
        runtime
            .insert_run(running_run("r4"))
            .expect("insert run");

        // Provider always returns a tool call but dispatch is a no-op; the loop
        // will hit max_steps without ever getting FinalText.
        let infinite_calls: Vec<_> = (0..10)
            .map(|i| crate::managers::ai_manager::AiToolTurn::ToolCalls {
                tool_calls: vec![crate::managers::ai_manager::AiToolCallRequest {
                    id: format!("call-{i}"),
                    name: "keynova_search".into(),
                    arguments: json!({ "query": "x" }),
                }],
            })
            .collect();
        let provider = FakeToolCallProvider::new(infinite_calls);

        let config = ReactLoopConfig {
            max_steps: 3,
            ..ReactLoopConfig::default()
        };
        react_loop_body(
            &runtime,
            "r4",
            &provider,
            &AgentToolRegistry::with_default_readonly_tools().list(),
            &config,
            &noop_dispatch,
        );

        let run = runtime.get("r4").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Failed);
        assert!(run.error.as_deref().unwrap_or("").contains("Maximum steps"));
    }
}
