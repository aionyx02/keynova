use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex};

use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::{prepare_observation, AgentAuditEntry, AgentObservationPolicy, AppEvent};
use crate::managers::ai_manager::{
    AiMessage, AiToolDefinition, AiToolTurn, ToolCallError, ToolCallProvider,
};
use crate::models::action::ActionRisk;
use crate::models::agent::{AgentApproval, AgentRun, AgentRunStatus, AgentStep, ContextVisibility};

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
        registry.register(tool_spec::<LearningMaterialReviewToolParams, LearningMaterialReviewToolResult>(
            "learning_material.review",
            "Scan workspace directories for learning materials (notes, reports, presentations, certificates, projects). \
             Metadata-only — no recursive content indexing. Requires agent.local_context.enabled = true and user approval.",
            ActionRisk::Medium,
            AgentToolApprovalPolicy::Required,
            vec![ContextVisibility::UserPrivate],
            15_000,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LearningMaterialReviewToolParams {
    /// Absolute paths to scan. Defaults to workspace project root when empty.
    pub roots: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LearningMaterialReviewToolResult {
    pub roots: Vec<String>,
    pub candidate_count: usize,
    pub scanned_count: usize,
    pub filtered_count: usize,
    pub markdown_summary: String,
}

/// Receives evicted agent runs when the runtime FIFO cap is exceeded.
/// Backing impls: `NoopArchiveSink` (tests / pre-FEAT.11 callers) and
/// `KnowledgeStoreArchiveSink` (production wire-up).
pub trait AgentArchiveSink: Send + Sync {
    fn archive(&self, run: &AgentRun);
}

pub struct NoopArchiveSink;

impl AgentArchiveSink for NoopArchiveSink {
    fn archive(&self, _run: &AgentRun) {}
}

/// In-memory run cache plus an insertion-order log used for FIFO eviction.
/// The two collections are kept under one mutex to avoid lock-ordering bugs.
struct RunStore {
    runs: HashMap<String, AgentRun>,
    order: VecDeque<String>,
}

impl RunStore {
    fn new() -> Self {
        Self {
            runs: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

pub struct AgentRuntime {
    store: Mutex<RunStore>,
    tools: AgentToolRegistry,
    publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
    /// Notified whenever a run is updated or cancelled; used by approval waiters.
    run_notify: (Mutex<()>, Condvar),
    /// Maximum number of runs kept in memory before the oldest is evicted.
    run_cap: usize,
    /// Sink for evicted runs.
    archive_sink: Arc<dyn AgentArchiveSink>,
}

const DEFAULT_RUN_CAP: usize = 20;

impl AgentRuntime {
    pub fn new(publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>) -> Self {
        Self::with_archive(publish_event, DEFAULT_RUN_CAP, Arc::new(NoopArchiveSink))
    }

    pub fn with_archive(
        publish_event: Arc<dyn Fn(AppEvent) + Send + Sync>,
        run_cap: usize,
        archive_sink: Arc<dyn AgentArchiveSink>,
    ) -> Self {
        Self {
            store: Mutex::new(RunStore::new()),
            tools: AgentToolRegistry::with_default_readonly_tools(),
            publish_event,
            run_notify: (Mutex::new(()), Condvar::new()),
            run_cap: run_cap.max(1),
            archive_sink,
        }
    }

    pub fn next_run_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn insert_run(&self, run: AgentRun) -> Result<AgentRun, String> {
        let run_id = run.id.clone();
        let mut evicted: Option<AgentRun> = None;
        {
            let mut store = self.store.lock().map_err(|e| e.to_string())?;
            // FIFO eviction: if at cap, archive and drop the oldest entry before
            // inserting the new one. The order log is the source of truth for
            // "oldest"; insertion-order, not status, governs eviction.
            while store.runs.len() >= self.run_cap {
                let Some(oldest_id) = store.order.pop_front() else {
                    break;
                };
                if let Some(old_run) = store.runs.remove(&oldest_id) {
                    evicted = Some(old_run);
                }
            }
            store.runs.insert(run_id.clone(), run.clone());
            store.order.push_back(run_id);
        }
        if let Some(old_run) = evicted.as_ref() {
            self.archive_sink.archive(old_run);
            self.publish("agent.run.archived", old_run);
        }
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
        {
            let mut store = self.store.lock().map_err(|e| e.to_string())?;
            store.runs.insert(run_id, run.clone());
        }
        self.publish(topic, &run);
        self.run_notify.1.notify_all();
        Ok(run)
    }

    pub fn cancel(&self, run_id: &str) -> Result<AgentRun, String> {
        let updated = {
            let mut store = self.store.lock().map_err(|e| e.to_string())?;
            let run = store
                .runs
                .get_mut(run_id)
                .ok_or_else(|| format!("agent run '{run_id}' not found"))?;
            run.status = AgentRunStatus::Cancelled;
            run.output = Some("Run cancelled before execution completed.".into());
            run.clone()
        };
        self.publish("agent.run.failed", &updated);
        self.run_notify.1.notify_all();
        Ok(updated)
    }

    pub fn get(&self, run_id: &str) -> Result<Option<AgentRun>, String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        Ok(store.runs.get(run_id).cloned())
    }

    pub fn clear_runs(&self) -> Result<(), String> {
        let mut store = self.store.lock().map_err(|e| e.to_string())?;
        store.runs.clear();
        store.order.clear();
        Ok(())
    }

    pub fn recent_runs(&self, limit: usize) -> Result<Vec<AgentRun>, String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        let mut runs: Vec<_> = store.runs.values().cloned().collect();
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
    /// Seconds the ReAct loop will block on a single approval gate before
    /// returning `TimedOut`. Bound to setting `agent.approval_timeout_secs`.
    pub approval_timeout_secs: u64,
    pub system_prompt: String,
    /// Optional callback invoked for each audit event during the loop.
    pub audit_log: Option<Arc<dyn Fn(AgentAuditEntry) + Send + Sync>>,
}

impl Default for ReactLoopConfig {
    fn default() -> Self {
        Self {
            max_steps: 8,
            max_tokens: 4096,
            timeout_secs: 30,
            approval_timeout_secs: 300,
            system_prompt: concat!(
                "You are a helpful assistant integrated into Keynova. ",
                "Use the provided tools to answer the user's request. ",
                "Once you have enough information, provide a final answer ",
                "without calling any more tools."
            )
            .into(),
            audit_log: None,
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

/// Fire-and-forget audit helper; no-op when `config.audit_log` is `None`.
fn maybe_audit(
    config: &ReactLoopConfig,
    run_id: &str,
    event_type: &str,
    status: &str,
    summary: &str,
    payload: Option<Value>,
) {
    if let Some(log) = &config.audit_log {
        log(AgentAuditEntry {
            run_id: run_id.to_string(),
            event_type: event_type.to_string(),
            status: status.to_string(),
            summary: summary.to_string(),
            payload_json: payload.map(|v| v.to_string()),
        });
    }
}

/// Outcome of waiting for a ReAct gate approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReactApprovalOutcome {
    Approved,
    Rejected,
    Cancelled,
    TimedOut,
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

    maybe_audit(
        config,
        run_id,
        "react_started",
        "ok",
        "ReAct loop started",
        Some(json!({
            "prompt_chars": prompt.chars().count(),
            "tool_count": tools.len(),
            "tools": tools.iter().map(|t| t.llm_name.as_str()).collect::<Vec<_>>(),
        })),
    );

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
            Ok(Some(run)) if run.status == AgentRunStatus::Cancelled => {
                maybe_audit(config, run_id, "react_cancelled", "cancelled", "Run cancelled.", None);
                return;
            }
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
                let reason = match &e {
                    ToolCallError::DaemonNotRunning { .. } => e.to_string(),
                    _ => format!("Provider error at step {step_idx}: {e}"),
                };
                maybe_audit(config, run_id, "react_failed", "error", &reason, None);
                react_fail(runtime, run_id, &reason);
                return;
            }
        };

        match turn {
            AiToolTurn::FinalText { content } => {
                maybe_audit(
                    config,
                    run_id,
                    "react_completed",
                    "ok",
                    "Final answer produced",
                    Some(json!({
                        "step": step_idx,
                        "answer_chars": content.chars().count(),
                        "answer_preview": truncate_preview(&content, 200),
                    })),
                );
                react_complete(runtime, run_id, step_idx, content);
                return;
            }
            AiToolTurn::ToolCalls { tool_calls } => {
                let mut obs_messages: Vec<AiMessage> = Vec::new();

                for call in &tool_calls {
                    let spec = tool_map.get(&call.name);

                    // Approval-required tools pause the loop and wait for user response.
                    let approval_required = spec
                        .map(|s| s.approval_policy == AgentToolApprovalPolicy::Required)
                        .unwrap_or(false);
                    if approval_required {
                        // Run-scoped "approve and remember" short-circuit: if any prior
                        // approval on this run matches the same tool and was approved
                        // with `remember_for_run=true`, skip the approval gate entirely.
                        let remembered = runtime
                            .get(run_id)
                            .ok()
                            .flatten()
                            .map(|run| {
                                run.approvals.iter().any(|a| {
                                    a.tool_name.as_deref() == Some(call.name.as_str())
                                        && a.remember_for_run
                                        && a.status == "approved"
                                })
                            })
                            .unwrap_or(false);

                        let outcome = if remembered {
                            maybe_audit(
                                config,
                                run_id,
                                "approval_auto_approved",
                                "ok",
                                &format!("Auto-approved '{}' via remember_for_run", call.name),
                                None,
                            );
                            ReactApprovalOutcome::Approved
                        } else {
                            let approval_id = format!("react-gate:{}", Uuid::new_v4());
                            let summary = format!(
                                "Agent wants to run '{}' with args: {}",
                                call.name,
                                truncate_preview(&call.arguments.to_string(), 200)
                            );
                            let deadline_unix_ms = current_unix_ms()
                                .map(|now| now + (config.approval_timeout_secs as i64) * 1000);
                            let approval = AgentApproval {
                                id: approval_id.clone(),
                                action_ref: None,
                                planned_action: None,
                                risk: spec.map(|s| s.risk).unwrap_or(ActionRisk::Medium),
                                summary: summary.clone(),
                                status: "pending".into(),
                                tool_name: Some(call.name.clone()),
                                deadline_unix_ms,
                                remember_for_run: false,
                            };

                            maybe_audit(
                                config,
                                run_id,
                                "approval_required",
                                "pending",
                                &summary,
                                Some(json!({
                                    "tool_name": call.name,
                                    "args_preview": truncate_preview(&call.arguments.to_string(), 200),
                                })),
                            );

                            // Attach the approval to the run and set WaitingApproval status.
                            if let Ok(Some(mut run)) = runtime.get(run_id) {
                                run.approvals.push(approval);
                                run.status = AgentRunStatus::WaitingApproval;
                                let _ = runtime.update_run(run, "agent.approval.required");
                            }
                            emit_react_step(runtime, run_id, step_idx, &call.name, "waiting_approval");

                            wait_for_react_approval(
                                runtime,
                                run_id,
                                &approval_id,
                                config.approval_timeout_secs,
                            )
                        };

                        let (obs, step_status) = match outcome {
                            ReactApprovalOutcome::Approved => {
                                maybe_audit(
                                    config,
                                    run_id,
                                    "approval_approved",
                                    "ok",
                                    &format!("Approved '{}'", call.name),
                                    None,
                                );
                                // Restore Running status before executing the tool.
                                if let Ok(Some(mut run)) = runtime.get(run_id) {
                                    run.status = AgentRunStatus::Running;
                                    let _ = runtime.update_run(run, "agent.step");
                                }
                                let obs_policy =
                                    spec.map(|s| s.observation_policy.clone()).unwrap_or_default();
                                // Inject approval token so dispatch-layer permission checks pass.
                                let mut approved_args = call.arguments.clone();
                                if let Some(obj) = approved_args.as_object_mut() {
                                    obj.insert("__approved".to_string(), json!(true));
                                }
                                let obs_text = match dispatch(&call.name, &approved_args) {
                                    Ok(val) => {
                                        let raw =
                                            serde_json::to_string_pretty(&val).unwrap_or_default();
                                        prepare_observation(&raw, &obs_policy).content
                                    }
                                    Err(e) => {
                                        format!("Error from tool '{}': {}", call.name, e)
                                    }
                                };
                                maybe_audit(
                                    config,
                                    run_id,
                                    "tool_observation",
                                    "ok",
                                    &format!("Observation from '{}'", call.name),
                                    Some(json!({
                                        "tool_name": call.name,
                                        "observation_preview": truncate_preview(&obs_text, 200),
                                    })),
                                );
                                (obs_text, "approved_executed")
                            }
                            ReactApprovalOutcome::Rejected => {
                                maybe_audit(
                                    config,
                                    run_id,
                                    "approval_rejected",
                                    "cancelled",
                                    &format!("Rejected '{}'", call.name),
                                    None,
                                );
                                (
                                    format!(
                                        "User rejected execution of '{}'. The tool was not run.",
                                        call.name
                                    ),
                                    "approval_rejected",
                                )
                            }
                            ReactApprovalOutcome::Cancelled => {
                                maybe_audit(
                                    config,
                                    run_id,
                                    "react_cancelled",
                                    "cancelled",
                                    "Run cancelled during approval wait.",
                                    None,
                                );
                                return;
                            }
                            ReactApprovalOutcome::TimedOut => {
                                maybe_audit(
                                    config,
                                    run_id,
                                    "approval_timeout",
                                    "error",
                                    &format!("Approval timed out for '{}'", call.name),
                                    None,
                                );
                                (
                                    format!(
                                        "Approval for '{}' timed out. The tool was not run.",
                                        call.name
                                    ),
                                    "approval_timeout",
                                )
                            }
                        };

                        emit_react_step(runtime, run_id, step_idx, &call.name, step_status);
                        obs_messages.push(AiMessage {
                            role: "user".into(),
                            content: format!("[Tool: {}]\n{}", call.name, obs),
                        });
                        continue;
                    }

                    maybe_audit(
                        config,
                        run_id,
                        "tool_called",
                        "ok",
                        &format!("Calling '{}'", call.name),
                        Some(json!({
                            "tool_name": call.name,
                            "args_preview": truncate_preview(&call.arguments.to_string(), 200),
                            "step": step_idx,
                        })),
                    );

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

                    maybe_audit(
                        config,
                        run_id,
                        "tool_observation",
                        "ok",
                        &format!("Observation from '{}'", call.name),
                        Some(json!({
                            "tool_name": call.name,
                            "observation_preview": truncate_preview(&observation, 200),
                        })),
                    );

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

    let reason = "Maximum steps exceeded without a final answer.";
    maybe_audit(
        config,
        run_id,
        "react_failed",
        "error",
        reason,
        Some(json!({ "max_steps": config.max_steps })),
    );
    react_fail(runtime, run_id, reason);
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

/// Best-effort unix-ms timestamp for approval deadlines. Returns `None` if the
/// system clock is set before the epoch (effectively impossible on modern OSes).
fn current_unix_ms() -> Option<i64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
}

fn truncate_preview(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        format!("{}…", chars[..max_chars].iter().collect::<String>())
    }
}

/// Block until the given approval changes from "pending", the run is cancelled,
/// or the deadline is reached.  Uses `AgentRuntime::run_notify` condvar so the
/// ReAct loop thread does not spin; worst-case latency is 1 s (the fallback
/// timeout passed to `wait_timeout`), not 100 ms per poll.
fn wait_for_react_approval(
    runtime: &AgentRuntime,
    run_id: &str,
    approval_id: &str,
    timeout_secs: u64,
) -> ReactApprovalOutcome {
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs.max(1));
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            // Mark the approval as timed out and emit a dedicated event so the
            // panel countdown pill can flip to "Timed out" without polling.
            if let Ok(Some(mut run)) = runtime.get(run_id) {
                if let Some(approval) = run.approvals.iter_mut().find(|a| a.id == approval_id) {
                    approval.status = "approval_timeout".into();
                }
                let _ = runtime.update_run(run, "agent.approval.timeout");
            }
            return ReactApprovalOutcome::TimedOut;
        }
        match runtime.get(run_id) {
            Ok(Some(run)) => {
                if run.status == AgentRunStatus::Cancelled {
                    return ReactApprovalOutcome::Cancelled;
                }
                if let Some(approval) = run.approvals.iter().find(|a| a.id == approval_id) {
                    match approval.status.as_str() {
                        "approved" => return ReactApprovalOutcome::Approved,
                        "rejected" => return ReactApprovalOutcome::Rejected,
                        _ => {}
                    }
                }
            }
            _ => return ReactApprovalOutcome::Cancelled,
        }
        let guard = runtime.run_notify.0.lock().unwrap_or_else(|e| e.into_inner());
        let _ = runtime
            .run_notify
            .1
            .wait_timeout(guard, remaining.min(std::time::Duration::from_secs(1)));
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
            context_bundle: None,
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
    fn approval_timeout_sets_status_and_emits_event() {
        use std::time::Duration;

        let topics: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let topics_clone = Arc::clone(&topics);
        let runtime = AgentRuntime::new(Arc::new(move |event| {
            topics_clone.lock().unwrap().push(event.topic);
        }));

        let approval_id = "react-gate:test-timeout".to_string();
        let mut run = test_run("rt-1", AgentRunStatus::WaitingApproval);
        run.approvals.push(AgentApproval {
            id: approval_id.clone(),
            action_ref: None,
            planned_action: None,
            risk: ActionRisk::Medium,
            summary: "timeout test".into(),
            status: "pending".into(),
            tool_name: Some("git.status".into()),
            deadline_unix_ms: None,
            remember_for_run: false,
        });
        runtime.insert_run(run).unwrap();

        let outcome = wait_for_react_approval(&runtime, "rt-1", &approval_id, 1);
        std::thread::sleep(Duration::from_millis(50));

        assert_eq!(outcome, ReactApprovalOutcome::TimedOut);
        let updated = runtime
            .get("rt-1")
            .unwrap()
            .unwrap()
            .approvals
            .into_iter()
            .find(|a| a.id == approval_id)
            .unwrap();
        assert_eq!(updated.status, "approval_timeout");

        let topic_log = topics.lock().unwrap();
        assert!(
            topic_log.iter().any(|t| t.as_str() == "agent.approval.timeout"),
            "expected agent.approval.timeout event, got {:?}",
            topic_log
        );
    }

    #[test]
    fn insert_run_evicts_fifo_when_over_cap_and_archives() {
        struct CountingSink(Mutex<Vec<String>>);
        impl AgentArchiveSink for CountingSink {
            fn archive(&self, run: &AgentRun) {
                self.0.lock().unwrap().push(run.id.clone());
            }
        }
        let sink = Arc::new(CountingSink(Mutex::new(Vec::new())));
        let topics: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let topics_clone = Arc::clone(&topics);
        let runtime = AgentRuntime::with_archive(
            Arc::new(move |event| {
                topics_clone.lock().unwrap().push(event.topic);
            }),
            3,
            sink.clone() as Arc<dyn AgentArchiveSink>,
        );

        for i in 0..5 {
            runtime
                .insert_run(test_run(&format!("run-{i}"), AgentRunStatus::Completed))
                .unwrap();
        }

        // run-0 and run-1 should have been evicted; runs 2/3/4 should remain.
        assert!(runtime.get("run-0").unwrap().is_none());
        assert!(runtime.get("run-1").unwrap().is_none());
        assert!(runtime.get("run-2").unwrap().is_some());
        assert!(runtime.get("run-4").unwrap().is_some());

        let evicted = sink.0.lock().unwrap();
        assert_eq!(evicted.as_slice(), &["run-0".to_string(), "run-1".to_string()]);

        let topic_log = topics.lock().unwrap();
        let archived_count = topic_log
            .iter()
            .filter(|t| t.as_str() == "agent.run.archived")
            .count();
        assert_eq!(archived_count, 2);
    }

    #[test]
    fn clear_runs_removes_all_cached_agent_runs() {
        let runtime = AgentRuntime::new(Arc::new(|_| {}));
        runtime
            .insert_run(test_run("run-a", AgentRunStatus::Completed))
            .expect("insert run-a");
        runtime
            .insert_run(test_run("run-b", AgentRunStatus::Completed))
            .expect("insert run-b");

        runtime.clear_runs().expect("clear runs");

        assert!(runtime.recent_runs(10).expect("recent runs").is_empty());
    }

    #[test]
    fn default_tool_registry_is_sorted_and_read_only() {
        let runtime = AgentRuntime::new(Arc::new(|_| {}));
        let tools = runtime.list_tools();

        assert_eq!(tools.len(), 6);
        assert_eq!(tools[0].name, "filesystem.read");
        assert_eq!(tools[1].name, "filesystem.search");
        assert_eq!(tools[2].name, "git.status");
        assert_eq!(tools[3].name, "keynova.search");
        assert_eq!(tools[4].name, "learning_material.review");
        assert_eq!(tools[5].name, "web.search");
        assert!(tools
            .iter()
            .all(|tool| tool.name != "execute_shell_command"));
        assert!(tools.iter().all(|tool| tool.name != "execute_bash_command"));
        assert_eq!(tools[2].approval_policy, AgentToolApprovalPolicy::Required);
        assert_eq!(tools[4].approval_policy, AgentToolApprovalPolicy::Required);
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

    // ─── ReAct loop tests (5.5.B1 / B2 / B3) ───────────────────────────────

    use crate::managers::fake_provider::FakeToolCallProvider;

    fn react_runtime() -> AgentRuntime {
        AgentRuntime::new(Arc::new(|_| {}))
    }

    fn react_runtime_arc() -> Arc<AgentRuntime> {
        Arc::new(AgentRuntime::new(Arc::new(|_| {})))
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
            context_bundle: None,
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
                    let outcome = search_system_index(query, &[root.clone()], 10, None);
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
    fn react_loop_approval_gate_approved_executes_tool() {
        use std::time::{Duration, Instant};

        let runtime = react_runtime_arc();
        runtime.insert_run(running_run("gate1")).expect("insert run");

        let provider: Arc<dyn crate::managers::ai_manager::ToolCallProvider> =
            Arc::new(FakeToolCallProvider::tool_then_final(
                "git_status",
                "call-git-approve",
                serde_json::json!({}),
                "Git status shows a clean working directory.",
            ));
        let dispatch: Arc<ToolDispatch> =
            Arc::new(|name: &str, _args: &Value| -> Result<Value, String> {
                if name == "git_status" {
                    Ok(json!({ "cwd": ".", "stdout": "", "stderr": "", "exit_code": 0 }))
                } else {
                    Err(format!("unexpected tool: {name}"))
                }
            });

        let tools = AgentToolRegistry::with_default_readonly_tools().list();
        runtime.spawn_react_loop(
            "gate1".into(),
            provider,
            tools,
            ReactLoopConfig::default(),
            dispatch,
        );

        // Wait until the loop sets WaitingApproval and adds the gate approval.
        let approval_id = {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                assert!(Instant::now() < deadline, "timed out waiting for approval");
                let run = runtime.get("gate1").unwrap().unwrap();
                if run.status == AgentRunStatus::WaitingApproval {
                    let id = run.approvals.last().unwrap().id.clone();
                    break id;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        };

        // Approve: set approval to "approved" and restore Running status.
        {
            let mut run = runtime.get("gate1").unwrap().unwrap();
            let approval = run.approvals.iter_mut().find(|a| a.id == approval_id).unwrap();
            approval.status = "approved".into();
            run.status = AgentRunStatus::Running;
            runtime.update_run(run, "agent.run.updated").unwrap();
        }

        // Wait for the loop to complete.
        let deadline = Instant::now() + Duration::from_secs(5);
        let run = loop {
            assert!(Instant::now() < deadline, "timed out waiting for completion");
            let run = runtime.get("gate1").unwrap().unwrap();
            if !matches!(
                run.status,
                AgentRunStatus::Running | AgentRunStatus::WaitingApproval
            ) {
                break run;
            }
            std::thread::sleep(Duration::from_millis(20));
        };

        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run
            .output
            .as_deref()
            .unwrap_or("")
            .contains("clean working directory"));
    }

    #[test]
    fn react_loop_approval_gate_rejected_informs_llm_and_continues() {
        use std::time::{Duration, Instant};

        let runtime = react_runtime_arc();
        runtime.insert_run(running_run("gate2")).expect("insert run");

        // Provider: first calls git_status (requires approval), then returns final text.
        let provider: Arc<dyn crate::managers::ai_manager::ToolCallProvider> =
            Arc::new(FakeToolCallProvider::tool_then_final(
                "git_status",
                "call-git-reject",
                serde_json::json!({}),
                "I cannot check git status (request was denied), but the project looks clean.",
            ));
        let dispatch: Arc<ToolDispatch> =
            Arc::new(|_name: &str, _args: &Value| -> Result<Value, String> {
                panic!("dispatch must NOT be called when tool is rejected")
            });

        let tools = AgentToolRegistry::with_default_readonly_tools().list();
        runtime.spawn_react_loop(
            "gate2".into(),
            provider,
            tools,
            ReactLoopConfig::default(),
            dispatch,
        );

        // Wait for WaitingApproval.
        let approval_id = {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                assert!(Instant::now() < deadline, "timed out waiting for approval");
                let run = runtime.get("gate2").unwrap().unwrap();
                if run.status == AgentRunStatus::WaitingApproval {
                    break run.approvals.last().unwrap().id.clone();
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        };

        // Reject: mark rejected, set Running so loop can continue.
        {
            let mut run = runtime.get("gate2").unwrap().unwrap();
            let approval = run.approvals.iter_mut().find(|a| a.id == approval_id).unwrap();
            approval.status = "rejected".into();
            run.status = AgentRunStatus::Running;
            runtime.update_run(run, "agent.run.updated").unwrap();
        }

        // Loop should resume, send rejection observation to LLM, get final text, complete.
        let deadline = Instant::now() + Duration::from_secs(5);
        let run = loop {
            assert!(Instant::now() < deadline, "timed out waiting for completion");
            let run = runtime.get("gate2").unwrap().unwrap();
            if !matches!(
                run.status,
                AgentRunStatus::Running | AgentRunStatus::WaitingApproval
            ) {
                break run;
            }
            std::thread::sleep(Duration::from_millis(20));
        };

        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run
            .output
            .as_deref()
            .unwrap_or("")
            .contains("cannot check git status"));
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

    #[test]
    fn react_loop_provider_network_error_fails_run() {
        let runtime = react_runtime();
        runtime
            .insert_run(running_run("r5"))
            .expect("insert run");

        // An empty FakeToolCallProvider returns ToolCallError::Network on the first call.
        let provider = FakeToolCallProvider::new([]);
        react_loop_body(
            &runtime,
            "r5",
            &provider,
            &[],
            &ReactLoopConfig::default(),
            &noop_dispatch,
        );

        let run = runtime.get("r5").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Failed);
        assert!(run.error.as_deref().unwrap_or("").contains("Provider error"));
    }

    #[test]
    fn react_loop_unknown_tool_produces_error_observation_and_loop_completes() {
        let runtime = react_runtime();
        runtime
            .insert_run(running_run("r6"))
            .expect("insert run");

        // Provider calls an unregistered tool (not in the spec list, not in dispatch).
        // The loop must not panic; it should surface an error observation and the LLM
        // returns final text on the next turn, completing the run.
        let provider = FakeToolCallProvider::tool_then_final(
            "execute_shell_command",
            "call-unsafe",
            json!({ "cmd": "echo hello" }),
            "I tried to run a shell command but it was not allowed; I found the answer another way.",
        );

        let deny_dispatch = |name: &str, _args: &Value| -> Result<Value, String> {
            Err(format!("unknown react tool '{name}'"))
        };

        react_loop_body(
            &runtime,
            "r6",
            &provider,
            &[], // empty spec list: no approval_required, just hits dispatch and gets Err
            &ReactLoopConfig::default(),
            &deny_dispatch,
        );

        let run = runtime.get("r6").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run
            .output
            .as_deref()
            .unwrap_or("")
            .contains("not allowed"));
    }

    // ─── P0.A: Two-step file find then read ─────────────────────────────────

    #[test]
    fn react_loop_two_step_search_then_read() {
        use crate::managers::ai_manager::AiToolCallRequest;
        use uuid::Uuid;

        let tmp_root = std::env::temp_dir()
            .join(format!("keynova-2step-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_root).expect("create tmp dir");
        let json_file = tmp_root.join("config.json");
        std::fs::write(&json_file, r#"{"mode":"local"}"#).expect("write file");
        let json_path = json_file.display().to_string();

        let path_for_read = json_path.clone();
        let provider = FakeToolCallProvider::new([
            AiToolTurn::ToolCalls {
                tool_calls: vec![AiToolCallRequest {
                    id: "call-search".into(),
                    name: "filesystem_search".into(),
                    arguments: json!({ "query": "config.json" }),
                }],
            },
            AiToolTurn::ToolCalls {
                tool_calls: vec![AiToolCallRequest {
                    id: "call-read".into(),
                    name: "filesystem_read".into(),
                    arguments: json!({ "path": path_for_read }),
                }],
            },
            AiToolTurn::FinalText {
                content: "The config.json file sets mode to local.".into(),
            },
        ]);

        let dispatch = move |name: &str, args: &Value| -> Result<Value, String> {
            match name {
                "filesystem_search" => Ok(json!({
                    "sources": [{
                        "title": "config.json",
                        "snippet": json_path,
                        "uri": json_path,
                        "source_type": "file",
                    }]
                })),
                "filesystem_read" => {
                    let path_str = args.get("path").and_then(Value::as_str).unwrap_or("");
                    let content = std::fs::read_to_string(path_str)
                        .map_err(|e| format!("read error: {e}"))?;
                    Ok(json!({
                        "sources": [{
                            "title": "config.json",
                            "snippet": content,
                            "uri": path_str,
                            "source_type": "file_read",
                        }]
                    }))
                }
                other => Err(format!("unexpected tool: {other}")),
            }
        };

        let runtime = react_runtime();
        runtime.insert_run(running_run("two-step-1")).expect("insert run");
        let tools = AgentToolRegistry::with_default_readonly_tools().list();
        react_loop_body(
            &runtime,
            "two-step-1",
            &provider,
            &tools,
            &ReactLoopConfig::default(),
            &dispatch,
        );

        let run = runtime.get("two-step-1").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run.output.as_deref().unwrap_or("").contains("local"));

        let _ = std::fs::remove_dir_all(tmp_root);
    }

    // ─── P0.A: Web search round-trip ─────────────────────────────────────────

    #[test]
    fn react_loop_web_search_returns_grounded_answer() {
        let provider = FakeToolCallProvider::tool_then_final(
            "web_search",
            "call-web",
            json!({ "query": "Rust async book" }),
            "According to web results, the async book is at async.rs.",
        );

        let dispatch = |name: &str, _args: &Value| -> Result<Value, String> {
            if name == "web_search" {
                Ok(json!({
                    "sources": [{
                        "title": "Rust Async Book",
                        "snippet": "async.rs is the official async Rust resource.",
                        "uri": "https://async.rs",
                        "source_type": "web",
                    }]
                }))
            } else {
                Err(format!("unexpected tool: {name}"))
            }
        };

        let runtime = react_runtime();
        runtime.insert_run(running_run("web1")).expect("insert run");
        let tools = AgentToolRegistry::with_default_readonly_tools().list();
        react_loop_body(
            &runtime,
            "web1",
            &provider,
            &tools,
            &ReactLoopConfig::default(),
            &dispatch,
        );

        let run = runtime.get("web1").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run.output.as_deref().unwrap_or("").contains("async.rs"));
    }

    // ─── P0.A: Stale index warning surfaces in observation ────────────────────

    #[test]
    fn react_loop_stale_index_warning_surfaced() {
        let provider = FakeToolCallProvider::tool_then_final(
            "filesystem_search",
            "call-stale",
            json!({ "query": "notes" }),
            "The search index may be stale, no results found.",
        );

        let dispatch = |name: &str, _args: &Value| -> Result<Value, String> {
            if name == "filesystem_search" {
                Ok(json!({
                    "sources": [],
                    "stale_index": true,
                    "stale_reason": "Index has not been updated in 24 hours.",
                }))
            } else {
                Err(format!("unexpected tool: {name}"))
            }
        };

        let runtime = react_runtime();
        runtime.insert_run(running_run("stale1")).expect("insert run");
        let tools = AgentToolRegistry::with_default_readonly_tools().list();
        react_loop_body(
            &runtime,
            "stale1",
            &provider,
            &tools,
            &ReactLoopConfig::default(),
            &dispatch,
        );

        let run = runtime.get("stale1").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run.output.as_deref().unwrap_or("").contains("stale"));
    }

    // ─── P0.B: No-tool run does not invoke dispatch ───────────────────────────

    #[test]
    fn react_loop_no_tool_call_dispatch_never_invoked() {
        let provider = FakeToolCallProvider::single_final("Direct answer, no tools needed.");
        let never_called = |name: &str, _args: &Value| -> Result<Value, String> {
            panic!("dispatch must NOT be invoked for a no-tool run, but called: {name}");
        };

        let runtime = react_runtime();
        runtime.insert_run(running_run("no-tool-1")).expect("insert run");
        react_loop_body(
            &runtime,
            "no-tool-1",
            &provider,
            &[],
            &ReactLoopConfig::default(),
            &never_called,
        );

        let run = runtime.get("no-tool-1").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
        assert!(run.steps.iter().all(|s| s.tool_calls.is_empty()));
    }

    #[test]
    fn react_loop_large_dispatch_result_does_not_panic() {
        let runtime = react_runtime();
        runtime
            .insert_run(running_run("r7"))
            .expect("insert run");

        // Dispatch returns a 50 KB sources blob to exercise the observation truncation path.
        let large_snippet = "x".repeat(50_000);
        let large_result = json!({
            "sources": [{
                "title": "Big file",
                "snippet": large_snippet,
                "source_type": "file",
            }]
        });
        let dispatch_data = large_result.clone();
        let big_dispatch = move |name: &str, _args: &Value| -> Result<Value, String> {
            if name == "keynova_search" {
                Ok(dispatch_data.clone())
            } else {
                Err(format!("unexpected tool: {name}"))
            }
        };

        let provider = FakeToolCallProvider::tool_then_final(
            "keynova_search",
            "call-big",
            json!({ "query": "large" }),
            "The large result was processed successfully.",
        );

        let tools = AgentToolRegistry::with_default_readonly_tools().list();
        react_loop_body(
            &runtime,
            "r7",
            &provider,
            &tools,
            &ReactLoopConfig::default(),
            &big_dispatch,
        );

        let run = runtime.get("r7").unwrap().unwrap();
        assert_eq!(run.status, AgentRunStatus::Completed);
    }
}
