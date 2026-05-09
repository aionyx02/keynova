use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::agent_runtime::{ReactLoopConfig, ToolDispatch};
use crate::core::config_manager::ConfigManager;
use crate::core::{
    prepare_observation, AgentAuditEntry, AgentMemoryEntry, AgentObservationPolicy, AgentRuntime,
    BuiltinCommandRegistry, CommandHandler, CommandResult, KnowledgeStoreHandle,
};
use crate::managers::{
    ai_manager::{provider_supports_tool_calls, resolve_ai_runtime_config, ToolCallProvider},
    history_manager::HistoryManager,
    model_manager::ModelManager,
    note_manager::NoteManager,
    system_indexer::search_system_index,
    workspace_manager::WorkspaceManager,
};
use crate::models::action::ActionRisk;
use crate::models::agent::{
    AgentActionKind, AgentApproval, AgentError, AgentMemoryRef, AgentMemoryScope,
    AgentPlannedAction, AgentRun, AgentRunStatus, AgentStep, AgentToolCall,
    ContextVisibility, GroundingSource,
};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::models::settings_schema::builtin_setting_schema;
use crate::models::terminal::TerminalLaunchSpec;


mod filesystem;
mod formatting;
mod intent;
mod safety;
mod web;

#[allow(unused_imports)]
use self::filesystem::*;
#[allow(unused_imports)]
use self::formatting::*;
#[allow(unused_imports)]
use self::intent::*;
#[allow(unused_imports)]
use self::safety::*;
#[allow(unused_imports)]
use self::web::*;
const PROMPT_BUDGET_CHARS: usize = 1400;
const PROMPT_SOURCE_LIMIT: usize = 6;
const SESSION_MEMORY_LIMIT: usize = 3;
const LONG_TERM_MEMORY_LIMIT: usize = 3;

const TOOL_KEYNOVA_SEARCH: &str = "keynova_search";
const TOOL_FILESYSTEM_SEARCH: &str = "filesystem_search";
const TOOL_FILESYSTEM_READ: &str = "filesystem_read";
const TOOL_WEB_SEARCH: &str = "web_search";
const TOOL_GIT_STATUS: &str = "git_status";

#[derive(Debug, Clone, Serialize)]
struct AgentToolRunResult {
    tool_name: String,
    sources: Vec<GroundingSource>,
}

/// Handles local agent runtime lifecycle commands and approved local actions.
pub struct AgentHandler {
    runtime: Arc<AgentRuntime>,
    config: Arc<Mutex<ConfigManager>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    model_manager: Arc<ModelManager>,
    knowledge_store: KnowledgeStoreHandle,
}

pub struct AgentHandlerDeps {
    pub runtime: Arc<AgentRuntime>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub note_manager: Arc<Mutex<NoteManager>>,
    pub history_manager: Arc<Mutex<HistoryManager>>,
    pub workspace_manager: Arc<Mutex<WorkspaceManager>>,
    pub builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    pub model_manager: Arc<ModelManager>,
    pub knowledge_store: KnowledgeStoreHandle,
}

impl AgentHandler {
    pub fn new(deps: AgentHandlerDeps) -> Self {
        Self {
            runtime: deps.runtime,
            config: deps.config,
            note_manager: deps.note_manager,
            history_manager: deps.history_manager,
            workspace_manager: deps.workspace_manager,
            builtin_registry: deps.builtin_registry,
            model_manager: deps.model_manager,
            knowledge_store: deps.knowledge_store,
        }
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
                self.start_run(prompt)
                    .and_then(|run| serde_json::to_value(run).map_err(|e| e.to_string()))
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
                let approval_id = require_str(&payload, "approval_id")?;
                self.approve_run(run_id, approval_id)
                    .and_then(|run| serde_json::to_value(run).map_err(|e| e.to_string()))
            }
            "reject" => {
                let run_id = require_str(&payload, "run_id")?;
                let approval_id = require_str(&payload, "approval_id")?;
                self.reject_run(run_id, approval_id)
                    .and_then(|run| serde_json::to_value(run).map_err(|e| e.to_string()))
            }
            "tools" => Ok(json!(self.runtime.list_tools())),
            "tool" => {
                let tool_name = require_str(&payload, "name")?;
                let query = require_str(&payload, "query")?;
                let limit = payload.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
                Ok(json!(self.run_tool(tool_name, query, limit)?))
            }
            _ => Err(format!("unknown agent command '{command}'")),
        }
    }
}

impl AgentHandler {
    fn start_run(&self, prompt: String) -> Result<AgentRun, String> {
        if self.should_use_react_loop() {
            self.start_react_run(prompt)
        } else {
            self.start_heuristic_run(prompt)
        }
    }

    /// Returns true when the current AI provider supports function/tool calling
    /// and `agent.mode` has not been explicitly set to `"offline"`.
    fn should_use_react_loop(&self) -> bool {
        let Ok(config) = self.config.lock() else {
            return false;
        };
        if config.get("agent.mode").as_deref() == Some("offline") {
            return false;
        }
        match resolve_ai_runtime_config(|key| config.get(key)) {
            Ok(rt) => provider_supports_tool_calls(&rt.provider),
            Err(_) => false,
        }
    }

    /// Normal mode: insert a Running run and spawn the ReAct loop.
    /// The LLM drives tool selection; local heuristics are not involved.
    fn start_react_run(&self, prompt: String) -> Result<AgentRun, String> {
        let run_id = self.runtime.next_run_id();
        let memory_refs = self.memory_refs()?;
        // Pre-populate prompt_audit with initial local context so the UI can show
        // which sources were considered even before the first tool call completes.
        let (initial_sources, _) = self.sources_for_prompt(&prompt).unwrap_or_default();
        let prompt_audit = build_prompt_audit(&prompt, &initial_sources, PROMPT_BUDGET_CHARS);
        let run = AgentRun {
            id: run_id.clone(),
            prompt: prompt.clone(),
            status: AgentRunStatus::Running,
            plan: vec![
                "Classify context by visibility.".into(),
                "LLM selects tools via ReAct loop.".into(),
                "Return grounded final answer.".into(),
            ],
            steps: vec![AgentStep {
                id: format!("{run_id}:react"),
                title: "ReAct loop".into(),
                status: "running".into(),
                tool_calls: Vec::new(),
            }],
            approvals: Vec::new(),
            memory_refs,
            sources: initial_sources,
            prompt_audit: Some(prompt_audit),
            command_result: None,
            output: None,
            error: None,
        };
        self.log_audit(
            &run_id,
            "run_started",
            "ok",
            "ReAct loop initiated",
            Some(json!({ "prompt_chars": prompt.chars().count() })),
        );
        let rt_config = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            resolve_ai_runtime_config(|key| config.get(key))?
        };
        let provider: Arc<dyn ToolCallProvider> = Arc::new(rt_config.provider);
        let tools = self.runtime.list_tools();
        let dispatch = self.build_react_dispatch();
        let knowledge_store = self.knowledge_store.clone();
        let loop_config = ReactLoopConfig {
            audit_log: Some(Arc::new(move |entry| {
                knowledge_store.try_log_agent_audit(entry);
            })),
            ..ReactLoopConfig::default()
        };
        let inserted = self.runtime.insert_run(run)?;
        self.runtime.spawn_react_loop(
            run_id,
            provider,
            tools,
            loop_config,
            dispatch,
        );
        Ok(inserted)
    }

    /// Offline fallback: resolve sources and planned actions with local heuristics.
    /// Used when the provider does not support tool calls or `agent.mode = "offline"`.
    fn start_heuristic_run(&self, prompt: String) -> Result<AgentRun, String> {
        let (sources, tool_calls) = self.sources_for_prompt(&prompt)?;
        let memory_refs = self.memory_refs()?;
        let prompt_audit = build_prompt_audit(&prompt, &sources, PROMPT_BUDGET_CHARS);
        let approvals = self.plan_approvals(&prompt)?;
        let direct_answer = self.direct_local_answer(&prompt);
        let status = if approvals.is_empty() {
            AgentRunStatus::Completed
        } else {
            AgentRunStatus::WaitingApproval
        };
        let plan = build_plan(
            &prompt,
            approvals
                .first()
                .and_then(|approval| approval.planned_action.as_ref()),
            direct_answer.is_some(),
        );
        let run_id = self.runtime.next_run_id();
        let run = AgentRun {
            id: run_id.clone(),
            prompt: prompt.clone(),
            status,
            plan,
            steps: vec![
                AgentStep {
                    id: format!("{run_id}:prompt"),
                    title: "Build filtered prompt".into(),
                    status: "completed".into(),
                    tool_calls,
                },
                AgentStep {
                    id: format!("{run_id}:approval"),
                    title: if approvals.is_empty() {
                        "No approval required".into()
                    } else {
                        "Waiting for approval".into()
                    },
                    status: if approvals.is_empty() {
                        "completed".into()
                    } else {
                        "pending".into()
                    },
                    tool_calls: Vec::new(),
                },
            ],
            approvals,
            memory_refs,
            sources,
            prompt_audit: Some(prompt_audit.clone()),
            command_result: None,
            output: Some(direct_answer.unwrap_or_else(|| describe_run(&prompt, &prompt_audit))),
            error: None,
        };

        self.log_audit(
            &run_id,
            "run_started",
            "ok",
            "Agent run prepared",
            Some(json!({
                "prompt_chars": prompt.chars().count(),
                "included_sources": prompt_audit.included_sources.len(),
                "filtered_sources": prompt_audit.filtered_sources.len(),
                "approval_count": run.approvals.len(),
            })),
        );
        if let Some(approval) = run.approvals.first() {
            self.log_audit(
                &run_id,
                "approval_required",
                "pending",
                &approval.summary,
                approval.planned_action.as_ref().map(|action| {
                    json!({
                        "action_id": action.id,
                        "kind": action.kind,
                        "risk": action.risk,
                    })
                }),
            );
        }
        self.runtime.insert_run(run)
    }

    fn approve_run(&self, run_id: &str, approval_id: &str) -> Result<AgentRun, String> {
        let mut run = self
            .runtime
            .get(run_id)?
            .ok_or_else(|| format!("agent run '{run_id}' not found"))?;
        let approval_index = run
            .approvals
            .iter()
            .position(|approval| approval.id == approval_id)
            .ok_or_else(|| format!("approval '{approval_id}' not found"))?;
        if run.approvals[approval_index].status != "pending" {
            return Err(format!("approval '{approval_id}' is not pending"));
        }
        match run.approvals[approval_index].planned_action.clone() {
            None => {
                // ReAct gate approval — mark approved, restore Running; loop resumes.
                run.approvals[approval_index].status = "approved".into();
                run.status = AgentRunStatus::Running;
                self.log_audit(
                    run_id,
                    "approval_approved",
                    "ok",
                    &run.approvals[approval_index].summary,
                    None,
                );
                self.runtime.update_run(run, "agent.run.updated")
            }
            Some(action) => {
                // Heuristic flow — execute planned action and complete the run.
                let command_result = self.execute_planned_action(&action)?;
                run.approvals[approval_index].status = "approved".into();
                run.status = AgentRunStatus::Completed;
                run.command_result = Some(command_result.clone());
                run.output = Some(describe_execution(&action, &command_result));
                if let Some(step) = run.steps.get_mut(1) {
                    step.status = "completed".into();
                    step.title = format!("Approved: {}", action.label);
                }
                self.log_audit(
                    run_id,
                    "approval_approved",
                    "ok",
                    &action.summary,
                    Some(json!({
                        "action_id": action.id,
                        "kind": action.kind,
                        "risk": action.risk,
                    })),
                );
                if long_term_memory_opt_in(&self.config) {
                    let workspace_id =
                        self.workspace_manager.lock().ok().map(|ws| ws.current().id);
                    self.knowledge_store.try_store_agent_memory(AgentMemoryEntry {
                        id: format!("run:{run_id}"),
                        scope: "long_term".into(),
                        workspace_id,
                        title: truncate(&run.prompt, 80),
                        content: run.output.clone().unwrap_or_default(),
                        visibility: "user_private".into(),
                    });
                }
                self.runtime.update_run(run, "agent.run.completed")
            }
        }
    }

    fn reject_run(&self, run_id: &str, approval_id: &str) -> Result<AgentRun, String> {
        let mut run = self
            .runtime
            .get(run_id)?
            .ok_or_else(|| format!("agent run '{run_id}' not found"))?;
        let approval_index = run
            .approvals
            .iter()
            .position(|approval| approval.id == approval_id)
            .ok_or_else(|| format!("approval '{approval_id}' not found"))?;
        run.approvals[approval_index].status = "rejected".into();
        let summary = run.approvals[approval_index].summary.clone();
        if run.approvals[approval_index].planned_action.is_none() {
            // ReAct gate rejection — mark rejected, restore Running; loop continues.
            run.status = AgentRunStatus::Running;
            self.log_audit(run_id, "approval_rejected", "cancelled", &summary, None);
            return self.runtime.update_run(run, "agent.run.updated");
        }
        // Heuristic flow — cancel the run.
        run.status = AgentRunStatus::Cancelled;
        run.command_result = None;
        run.output = Some(format!("Approval rejected. {summary}"));
        if let Some(step) = run.steps.get_mut(1) {
            step.status = "cancelled".into();
            step.title = "Approval rejected".into();
        }
        self.log_audit(run_id, "approval_rejected", "cancelled", &summary, None);
        self.runtime.update_run(run, "agent.run.failed")
    }

    fn memory_refs(&self) -> Result<Vec<AgentMemoryRef>, String> {
        let mut refs = Vec::new();
        for run in self.runtime.recent_runs(SESSION_MEMORY_LIMIT)? {
            if run.output.as_deref().unwrap_or("").is_empty() {
                continue;
            }
            refs.push(AgentMemoryRef {
                id: format!("session:{}", run.id),
                scope: AgentMemoryScope::Session,
                visibility: ContextVisibility::UserPrivate,
                summary: truncate(
                    &format!(
                        "Recent run: {} -> {}",
                        run.prompt,
                        run.output.unwrap_or_default()
                    ),
                    140,
                ),
            });
        }

        if let Ok(workspace) = self.workspace_manager.lock() {
            let current = workspace.current();
            refs.push(AgentMemoryRef {
                id: format!("workspace:{}", current.id),
                scope: AgentMemoryScope::Workspace,
                visibility: ContextVisibility::PublicContext,
                summary: format!(
                    "Workspace {} with {} recent files, {} notes, and {} recent actions.",
                    current.name,
                    current.recent_files.len(),
                    current.note_ids.len(),
                    current.recent_actions.len()
                ),
            });
            if long_term_memory_opt_in(&self.config) {
                for memory in self.knowledge_store.agent_memories_blocking(
                    Some("long_term".into()),
                    Some(current.id),
                    LONG_TERM_MEMORY_LIMIT,
                )? {
                    refs.push(AgentMemoryRef {
                        id: memory.id,
                        scope: AgentMemoryScope::LongTerm,
                        visibility: parse_visibility(&memory.visibility),
                        summary: truncate(&format!("{}: {}", memory.title, memory.content), 140),
                    });
                }
            }
        }

        Ok(refs)
    }

    fn plan_approvals(&self, prompt: &str) -> Result<Vec<AgentApproval>, String> {
        let Some(action) = self.detect_planned_action(prompt)? else {
            return Ok(Vec::new());
        };
        Ok(vec![AgentApproval {
            id: format!("approval:{}", Uuid::new_v4()),
            action_ref: None,
            planned_action: Some(action.clone()),
            risk: action.risk,
            summary: action.summary.clone(),
            status: "pending".into(),
        }])
    }

    fn detect_planned_action(&self, prompt: &str) -> Result<Option<AgentPlannedAction>, String> {
        if let Some(action) = self.plan_note_draft(prompt) {
            return Ok(Some(action));
        }
        if let Some(action) = self.plan_setting_draft(prompt) {
            return Ok(Some(action));
        }
        if let Some(action) = self.plan_terminal_command(prompt) {
            return Ok(Some(action));
        }
        if let Some(action) = self.plan_file_write(prompt) {
            return Ok(Some(action));
        }
        if let Some(action) = self.plan_system_control(prompt) {
            return Ok(Some(action));
        }
        if let Some(action) = self.plan_model_lifecycle(prompt) {
            return Ok(Some(action));
        }
        if let Some(action) = self.plan_panel_open(prompt) {
            return Ok(Some(action));
        }
        if let Some(action) = self.plan_safe_builtin_command(prompt) {
            return Ok(Some(action));
        }
        Ok(None)
    }

    fn plan_note_draft(&self, prompt: &str) -> Option<AgentPlannedAction> {
        let lower = prompt.to_lowercase();
        if !contains_any(
            &lower,
            &[
                "note", "notes", "memo", "draft", "筆記", "笔记", "便條", "草稿",
            ],
        ) {
            return None;
        }
        if !contains_any(
            &lower,
            &["create", "draft", "write", "記下", "写", "建立", "草稿"],
        ) {
            return None;
        }

        let name = suggested_note_name(prompt);
        let content = format!(
            "# {}\n\nSource request:\n{}\n\n- [ ] Expand this draft\n",
            title_case(&name),
            prompt.trim()
        );

        Some(AgentPlannedAction {
            id: format!("agent-note-draft:{}", Uuid::new_v4()),
            kind: AgentActionKind::CreateNoteDraft,
            risk: ActionRisk::Medium,
            label: "Create note draft".into(),
            summary: format!("Open the note editor with a draft named '{name}'."),
            payload: json!({
                "draft_name": name,
                "draft_content": content,
            }),
        })
    }

    fn plan_setting_draft(&self, prompt: &str) -> Option<AgentPlannedAction> {
        let lower = prompt.to_lowercase();
        if !contains_any(
            &lower,
            &[
                "setting",
                "settings",
                "config",
                "preference",
                "設定",
                "设置",
                "偏好",
            ],
        ) {
            return None;
        }

        let schema = match_setting_schema(prompt)?;
        let value = extract_setting_value(prompt, &schema.value_type)?;
        Some(AgentPlannedAction {
            id: format!("agent-setting-draft:{}", Uuid::new_v4()),
            kind: AgentActionKind::UpdateSettingDraft,
            risk: ActionRisk::Medium,
            label: "Update setting draft".into(),
            summary: format!("Open settings with a draft change for '{}'.", schema.key),
            payload: json!({
                "key": schema.key,
                "value": value,
            }),
        })
    }

    fn plan_terminal_command(&self, prompt: &str) -> Option<AgentPlannedAction> {
        let command = extract_shell_command(prompt)?;
        let spec = build_terminal_command_spec(&self.config, &command);
        Some(AgentPlannedAction {
            id: format!("agent-terminal:{}", Uuid::new_v4()),
            kind: AgentActionKind::TerminalCommand,
            risk: ActionRisk::High,
            label: "Run terminal command".into(),
            summary: format!("Run terminal command '{command}'."),
            payload: serde_json::to_value(spec).ok()?,
        })
    }

    fn plan_file_write(&self, prompt: &str) -> Option<AgentPlannedAction> {
        let lower = prompt.to_lowercase();
        if !contains_any(
            &lower,
            &[
                "write file",
                "edit file",
                "create file",
                "修改檔案",
                "写入文件",
                "建立檔案",
            ],
        ) {
            return None;
        }
        let path = extract_path_like(prompt).unwrap_or_else(|| "(path not parsed)".into());
        Some(AgentPlannedAction {
            id: format!("agent-file-write:{}", Uuid::new_v4()),
            kind: AgentActionKind::FileWrite,
            risk: ActionRisk::High,
            label: "Prepare file write scaffold".into(),
            summary: format!("Prepare a file-write scaffold for {path}."),
            payload: json!({
                "path": path,
                "prompt": prompt,
            }),
        })
    }

    fn plan_system_control(&self, prompt: &str) -> Option<AgentPlannedAction> {
        let lower = prompt.to_lowercase();
        if !contains_any(
            &lower,
            &[
                "volume",
                "brightness",
                "wifi",
                "mute",
                "音量",
                "亮度",
                "網路",
                "静音",
            ],
        ) {
            return None;
        }
        Some(AgentPlannedAction {
            id: format!("agent-system:{}", Uuid::new_v4()),
            kind: AgentActionKind::SystemControl,
            risk: ActionRisk::High,
            label: "Open system control".into(),
            summary: "Open the system control panel after explicit approval.".into(),
            payload: json!({
                "panel": "system",
                "initial_args": prompt,
            }),
        })
    }

    fn plan_model_lifecycle(&self, prompt: &str) -> Option<AgentPlannedAction> {
        let lower = prompt.to_lowercase();
        if contains_any(
            &lower,
            &["download model", "install model", "下載模型", "下载模型"],
        ) {
            return Some(AgentPlannedAction {
                id: format!("agent-model-download:{}", Uuid::new_v4()),
                kind: AgentActionKind::ModelLifecycle,
                risk: ActionRisk::High,
                label: "Open model download".into(),
                summary: "Open model download after explicit approval.".into(),
                payload: json!({
                    "panel": "model_download",
                    "initial_args": prompt,
                }),
            });
        }
        if contains_any(
            &lower,
            &["delete model", "remove model", "刪除模型", "删除模型"],
        ) {
            return Some(AgentPlannedAction {
                id: format!("agent-model-delete:{}", Uuid::new_v4()),
                kind: AgentActionKind::ModelLifecycle,
                risk: ActionRisk::High,
                label: "Open model list".into(),
                summary: "Open model management after explicit approval.".into(),
                payload: json!({
                    "panel": "model_list",
                    "initial_args": prompt,
                }),
            });
        }
        None
    }

    fn plan_panel_open(&self, prompt: &str) -> Option<AgentPlannedAction> {
        let lower = prompt.to_lowercase();
        let (panel, label) = if contains_any(&lower, &["history", "clipboard", "歷史", "剪貼"])
        {
            ("history", "Open history panel")
        } else if contains_any(&lower, &["setting", "settings", "設定", "设置"]) {
            ("setting", "Open settings panel")
        } else if contains_any(&lower, &["note", "notes", "筆記", "笔记"]) {
            ("note", "Open notes panel")
        } else if contains_any(&lower, &["translate", "translation", "翻譯", "翻译"]) {
            ("translation", "Open translation panel")
        } else if contains_any(&lower, &["calculator", "calculate", "計算", "计算"]) {
            ("calculator", "Open calculator panel")
        } else if contains_any(&lower, &["model list", "models", "模型列表", "模型"]) {
            ("model_list", "Open model list panel")
        } else if contains_any(&lower, &["ai", "agent", "chat"]) {
            ("ai", "Open AI panel")
        } else {
            return None;
        };
        Some(AgentPlannedAction {
            id: format!("agent-panel:{panel}:{}", Uuid::new_v4()),
            kind: AgentActionKind::OpenPanel,
            risk: ActionRisk::Medium,
            label: label.into(),
            summary: format!("Open the {panel} panel."),
            payload: json!({
                "panel": panel,
                "initial_args": "",
            }),
        })
    }

    fn plan_safe_builtin_command(&self, prompt: &str) -> Option<AgentPlannedAction> {
        let lower = prompt.to_lowercase();
        let name = if contains_any(&lower, &["help", "commands", "說明", "指令"]) {
            "help"
        } else {
            return None;
        };
        Some(AgentPlannedAction {
            id: format!("agent-safe-cmd:{name}:{}", Uuid::new_v4()),
            kind: AgentActionKind::RunBuiltinCommand,
            risk: ActionRisk::Medium,
            label: format!("Run /{name}"),
            summary: format!("Run the allowlisted built-in command '/{name}'."),
            payload: json!({
                "name": name,
                "args": "",
            }),
        })
    }

    fn execute_planned_action(
        &self,
        action: &AgentPlannedAction,
    ) -> Result<BuiltinCommandResult, String> {
        match action.kind {
            AgentActionKind::OpenPanel => Ok(panel_result(
                require_str(&action.payload, "panel")?,
                action
                    .payload
                    .get("initial_args")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            )),
            AgentActionKind::CreateNoteDraft => {
                Ok(panel_result("note", action.payload.to_string()))
            }
            AgentActionKind::UpdateSettingDraft => {
                Ok(panel_result("setting", action.payload.to_string()))
            }
            AgentActionKind::RunBuiltinCommand => {
                let name = require_str(&action.payload, "name")?;
                let args = action
                    .payload
                    .get("args")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if !is_allowlisted_safe_builtin(name, args) {
                    return Err(format!("built-in command '/{name}' is not allowlisted"));
                }
                let registry = self.builtin_registry.lock().map_err(|e| e.to_string())?;
                let result = registry
                    .run(name, args)
                    .ok_or_else(|| format!("unknown built-in command '/{name}'"))?;
                if matches!(result.ui_type, CommandUiType::Terminal(_)) {
                    return Err(format!(
                        "built-in command '/{name}' is not safe for agent use"
                    ));
                }
                Ok(result)
            }
            AgentActionKind::TerminalCommand => {
                let spec: TerminalLaunchSpec =
                    serde_json::from_value(action.payload.clone()).map_err(|e| e.to_string())?;
                Ok(BuiltinCommandResult {
                    text: String::new(),
                    ui_type: CommandUiType::Terminal(spec),
                })
            }
            AgentActionKind::FileWrite => Ok(inline_result(format!(
                "Approved file-write scaffold. Direct file mutation remains blocked here. {}",
                action.summary
            ))),
            AgentActionKind::SystemControl | AgentActionKind::ModelLifecycle => Ok(panel_result(
                require_str(&action.payload, "panel")?,
                action
                    .payload
                    .get("initial_args")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            )),
        }
    }

    fn sources_for_prompt(
        &self,
        prompt: &str,
    ) -> Result<(Vec<GroundingSource>, Vec<AgentToolCall>), String> {
        if !should_run_local_search(prompt) {
            return Ok((Vec::new(), Vec::new()));
        }
        let started = Instant::now();
        let sources = self.keynova_search(prompt, 8)?;
        let tool_call = AgentToolCall {
            id: format!("tool:{}", Uuid::new_v4()),
            tool_name: "keynova.search".into(),
            risk: ActionRisk::Low,
            status: "completed".into(),
            duration_ms: Some(started.elapsed().as_millis()),
            error: None,
        };
        Ok((sources, vec![tool_call]))
    }

    fn run_tool(
        &self,
        tool_name: &str,
        query: &str,
        limit: usize,
    ) -> Result<AgentToolRunResult, String> {
        let sources = match tool_name {
            "keynova.search" => self.keynova_search(query, limit)?,
            "web.search" => self.web_search(query, limit)?,
            "filesystem.search" => self.filesystem_search_sources(query, limit),
            "filesystem.read" => self.filesystem_read_source(query)?,
            "git.status" => return Err(
                "git.status is a typed approval-gated tool and cannot be run through agent.tool"
                    .into(),
            ),
            other => return Err(format!("unknown agent tool '{other}'")),
        };
        Ok(AgentToolRunResult {
            tool_name: tool_name.to_string(),
            sources,
        })
    }

    fn direct_local_answer(&self, prompt: &str) -> Option<String> {
        let roots = self.filesystem_search_roots_for_prompt(prompt);
        answer_directory_listing(prompt, &roots)
            .or_else(|| self.answer_file_read(prompt, &roots))
            .or_else(|| self.answer_project_type_summary(prompt, &roots))
            .or_else(|| self.answer_github_trending(prompt))
            .or_else(|| self.answer_filesystem_search(prompt, &roots))
            .or_else(|| self.answer_web_search(prompt))
            .or_else(|| answer_workflow_plan(prompt))
            .or_else(|| direct_local_answer(prompt))
    }

    fn filesystem_search_roots_for_prompt(&self, prompt: &str) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Ok(workspace) = self.workspace_manager.lock() {
            if let Some(root) = workspace.current().project_root.as_deref() {
                if !root.trim().is_empty() {
                    roots.push(PathBuf::from(root));
                }
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd);
        }
        if wants_whole_computer_search(prompt) {
            roots.extend(system_search_roots());
        }
        roots.dedup();
        roots
    }

    fn answer_filesystem_search(&self, prompt: &str, roots: &[PathBuf]) -> Option<String> {
        let query = extract_filesystem_search_query(prompt)?;
        let outcome = search_system_index(&query, roots, 20);
        Some(format_system_index_search_answer(&query, &outcome))
    }

    fn answer_file_read(&self, prompt: &str, roots: &[PathBuf]) -> Option<String> {
        let target = extract_file_read_target(prompt)?;
        Some(read_file_answer(&target, roots))
    }

    fn answer_project_type_summary(&self, prompt: &str, roots: &[PathBuf]) -> Option<String> {
        if !is_project_type_summary_prompt(prompt) {
            return None;
        }
        Some(format_project_type_summary(&scan_project_types(roots)))
    }

    fn answer_github_trending(&self, prompt: &str) -> Option<String> {
        if !is_github_trending_prompt(prompt) {
            return None;
        }
        Some(match fetch_github_trending(10) {
            Ok(repos) if repos.is_empty() => {
                "我查詢了 GitHub Trending daily，但沒有解析到熱門專案。".into()
            }
            Ok(repos) => format_github_trending_answer(&repos),
            Err(error) => format!(
                "我目前無法查詢 GitHub Trending：{error}\n\n你也可以先用 web.search 查詢 `GitHub trending repositories today`。"
            ),
        })
    }

    fn answer_web_search(&self, prompt: &str) -> Option<String> {
        let query = extract_web_search_query(prompt)?;
        Some(match self.web_search(&query, 5) {
            Ok(sources) if sources.is_empty() => {
                format!("我查了網路，但沒有找到 `{query}` 的可用結果。")
            }
            Ok(sources) => format_web_search_answer(&query, &sources),
            Err(error) => format!("我目前無法完成網路查詢 `{query}`：{error}"),
        })
    }

    fn filesystem_search_sources(&self, query: &str, limit: usize) -> Vec<GroundingSource> {
        search_system_index(
            query,
            &self.filesystem_search_roots_for_prompt(query),
            limit,
        )
        .hits
        .into_iter()
        .enumerate()
        .map(|(index, hit)| {
            source(
                format!("filesystem:{}", hit.path),
                if hit.is_dir { "folder" } else { "file" },
                hit.name,
                hit.path,
                0.88 - (index as f32 * 0.01),
                ContextVisibility::UserPrivate,
            )
        })
        .collect()
    }

    fn filesystem_read_source(&self, query: &str) -> Result<Vec<GroundingSource>, String> {
        let target = extract_file_read_target(query).unwrap_or_else(|| query.trim().to_string());
        let roots = self.filesystem_search_roots_for_prompt(query);
        let (path, _) = resolve_file_target(&target, &roots);
        let path = path.ok_or_else(|| format!("file '{target}' not found"))?;
        let preview = read_text_preview(&path, 12_000)?;
        let observation = prepare_observation(
            &preview,
            &AgentObservationPolicy {
                max_chars: 4096,
                max_lines: 120,
                preserve_head_lines: 48,
                preserve_tail_lines: 48,
                redact_secrets: true,
            },
        );
        Ok(vec![source(
            format!("filesystem-read:{}", path.display()),
            "file_read",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file")
                .to_string(),
            observation.content,
            0.95,
            ContextVisibility::UserPrivate,
        )])
    }

    fn local_searcher(&self) -> LocalContextSearcher {
        LocalContextSearcher {
            workspace_manager: Arc::clone(&self.workspace_manager),
            note_manager: Arc::clone(&self.note_manager),
            history_manager: Arc::clone(&self.history_manager),
            builtin_registry: Arc::clone(&self.builtin_registry),
            model_manager: Arc::clone(&self.model_manager),
        }
    }

    fn keynova_search(&self, query: &str, limit: usize) -> Result<Vec<GroundingSource>, String> {
        let searcher = self.local_searcher();
        let mut sources = Vec::new();
        let q = query.to_lowercase();

        searcher.push_workspace_source(&mut sources);
        searcher.push_command_sources(&q, &mut sources)?;
        self.push_setting_schema_sources(&q, &mut sources);
        searcher.push_model_sources(&q, &mut sources);
        searcher.push_note_sources(&q, &mut sources)?;
        searcher.push_history_sources(query, &mut sources)?;

        sources.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.source_type.cmp(&right.source_type))
                .then_with(|| left.title.cmp(&right.title))
        });
        sources.truncate(limit.max(1));
        Ok(sources)
    }

    fn push_setting_schema_sources(&self, query: &str, sources: &mut Vec<GroundingSource>) {
        for schema in builtin_setting_schema()
            .into_iter()
            .filter(|schema| !schema.sensitive)
            .filter(|schema| {
                query.is_empty()
                    || schema.key.contains(query)
                    || schema.label.to_lowercase().contains(query)
            })
        {
            sources.push(source(
                format!("setting:{}", schema.key),
                "setting_schema",
                schema.key.to_string(),
                format!("{} default={}", schema.label, schema.default_value),
                0.72,
                ContextVisibility::PublicContext,
            ));
        }
    }

    fn web_search(&self, query: &str, limit: usize) -> Result<Vec<GroundingSource>, String> {
        let sanitized = sanitize_external_query(query)?;
        let (provider_str, searxng_url, api_key, timeout_secs) = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            (
                config
                    .get("agent.web_search_provider")
                    .unwrap_or_else(|| "disabled".into()),
                config.get("agent.searxng_url").unwrap_or_default(),
                config.get("agent.web_search_api_key").unwrap_or_default(),
                config
                    .get("agent.web_search_timeout_secs")
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(8),
            )
        };
        let provider = resolve_web_search_provider(&provider_str, &searxng_url, &api_key)?;
        provider.search(&sanitized, limit, timeout_secs)
    }

    fn log_audit(
        &self,
        run_id: &str,
        event_type: &str,
        status: &str,
        summary: &str,
        payload: Option<Value>,
    ) {
        self.knowledge_store.try_log_agent_audit(AgentAuditEntry {
            run_id: run_id.to_string(),
            event_type: event_type.to_string(),
            status: status.to_string(),
            summary: summary.to_string(),
            payload_json: payload.map(|value| value.to_string()),
        });
    }
}


// ─── Shared Local Context Searcher ──────────────────────────────────────────

/// Shared search implementation used by both the heuristic path (AgentHandler)
/// and the ReAct dispatch path (ReactDispatchState).
struct LocalContextSearcher {
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    model_manager: Arc<ModelManager>,
}

impl LocalContextSearcher {
    fn push_workspace_source(&self, sources: &mut Vec<GroundingSource>) {
        let Ok(workspace) = self.workspace_manager.lock() else {
            return;
        };
        let current = workspace.current();
        let snippet = format!(
            "name={}, mode={}, panel={}, recent_files={}, notes={}",
            current.name,
            current.mode,
            current.panel.as_deref().unwrap_or("none"),
            current.recent_files.len(),
            current.note_ids.len()
        );
        sources.push(source(
            format!("workspace:{}", current.id),
            "workspace",
            current.name.clone(),
            snippet,
            0.92,
            ContextVisibility::PublicContext,
        ));
    }

    fn push_command_sources(
        &self,
        query: &str,
        sources: &mut Vec<GroundingSource>,
    ) -> Result<(), String> {
        let registry = self.builtin_registry.lock().map_err(|e| e.to_string())?;
        for meta in registry.list().into_iter().filter(|meta| {
            query.is_empty()
                || meta.name.contains(query)
                || meta.description.to_lowercase().contains(query)
        }) {
            sources.push(source(
                format!("command:{}", meta.name),
                "command",
                format!("/{}", meta.name),
                meta.description.to_string(),
                0.84,
                ContextVisibility::PublicContext,
            ));
        }
        Ok(())
    }

    fn push_note_sources(
        &self,
        query: &str,
        sources: &mut Vec<GroundingSource>,
    ) -> Result<(), String> {
        let notes = self.note_manager.lock().map_err(|e| e.to_string())?;
        for note in notes.list() {
            let content = notes.get(&note.name).unwrap_or_default();
            let searchable = format!("{} {}", note.name, content).to_lowercase();
            if !query.is_empty() && !searchable.contains(query) {
                continue;
            }
            let snippet = truncate(&content.replace('\n', " "), 160);
            sources.push(visibility_filtered_source(
                format!("note:{}", note.name),
                "note",
                note.name,
                if snippet.is_empty() {
                    format!("{} bytes", note.size_bytes)
                } else {
                    snippet
                },
                0.78,
            ));
        }
        Ok(())
    }

    fn push_history_sources(
        &self,
        query: &str,
        sources: &mut Vec<GroundingSource>,
    ) -> Result<(), String> {
        let history = self.history_manager.lock().map_err(|e| e.to_string())?;
        for entry in history.search(query) {
            sources.push(visibility_filtered_source(
                format!("history:{}", entry.id),
                "history",
                entry.content_type.clone(),
                truncate(&entry.content.replace('\n', " "), 160),
                if entry.pinned { 0.7 } else { 0.62 },
            ));
        }
        Ok(())
    }

    fn push_model_sources(&self, query: &str, sources: &mut Vec<GroundingSource>) {
        let hardware = crate::managers::model_manager::HardwareInfo {
            ram_mb: 0,
            vram_mb: 0,
        };
        for model in self
            .model_manager
            .catalog_fast(&hardware)
            .into_iter()
            .filter(|model| query.is_empty() || model.name.to_lowercase().contains(query))
        {
            sources.push(source(
                format!("model:{}", model.name),
                "model",
                model.name,
                model.rating,
                0.68,
                ContextVisibility::PublicContext,
            ));
        }
    }
}

// ─── ReAct Tool Dispatch ─────────────────────────────────────────────────────

/// Arc-captured state for the ReAct loop dispatch closure.
/// One instance per agent run; shared across loop steps.
struct ReactDispatchState {
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    config: Arc<Mutex<ConfigManager>>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    builtin_registry: Arc<Mutex<BuiltinCommandRegistry>>,
    model_manager: Arc<ModelManager>,
    #[allow(dead_code)]
    knowledge_store: KnowledgeStoreHandle,
}

impl ReactDispatchState {
    fn dispatch(&self, name: &str, args: &Value) -> Result<Value, String> {
        // Approval-gated tools require `"__approved": true` injected by the ReAct loop
        // after the user explicitly grants permission. Direct dispatch without approval fails.
        const APPROVAL_GATED: &[&str] = &[TOOL_GIT_STATUS];
        if APPROVAL_GATED.contains(&name)
            && args.get("__approved").and_then(Value::as_bool) != Some(true)
        {
            return Err(AgentError::ToolDenied {
                tool: name.to_string(),
                reason: "requires explicit user approval before dispatch".into(),
            }
            .to_string());
        }
        match name {
            TOOL_KEYNOVA_SEARCH => self.dispatch_keynova_search(args),
            TOOL_FILESYSTEM_SEARCH => self.dispatch_filesystem_search(args),
            TOOL_FILESYSTEM_READ => self.dispatch_filesystem_read(args),
            TOOL_WEB_SEARCH => self.dispatch_web_search(args),
            TOOL_GIT_STATUS => self.dispatch_git_status(args),
            other => Err(format!("unknown react tool '{other}'")),
        }
    }

    fn dispatch_filesystem_search(&self, args: &Value) -> Result<Value, String> {
        let query = args.get("query").and_then(Value::as_str).unwrap_or("").to_string();
        if query.trim().is_empty() {
            return Err("filesystem.search: 'query' must not be empty".into());
        }
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize;
        let roots: Vec<PathBuf> = args
            .get("roots")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_else(|| self.default_search_roots());

        let outcome = search_system_index(&query, &roots, limit.max(1));
        let sources: Vec<Value> = outcome
            .hits
            .into_iter()
            .map(|hit| {
                json!({
                    "title": hit.name,
                    "snippet": hit.path,
                    "uri": hit.path,
                    "source_type": if hit.is_dir { "folder" } else { "file" },
                })
            })
            .collect();
        Ok(json!({ "sources": sources }))
    }

    fn dispatch_filesystem_read(&self, args: &Value) -> Result<Value, String> {
        let path_str = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "filesystem.read: missing 'path' argument".to_string())?;
        let max_chars = args
            .get("max_chars")
            .and_then(Value::as_u64)
            .unwrap_or(4096) as usize;

        let roots = self.default_search_roots();
        let resolved = resolve_readable_path(path_str, &roots)?;

        let preview = read_text_preview(&resolved, max_chars.min(12_000))?;
        let observation = prepare_observation(
            &preview,
            &AgentObservationPolicy {
                max_chars,
                max_lines: 120,
                preserve_head_lines: 48,
                preserve_tail_lines: 48,
                redact_secrets: true,
            },
        );
        let name = resolved
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path_str)
            .to_string();
        Ok(json!({
            "sources": [{
                "title": name,
                "snippet": observation.content,
                "uri": resolved.display().to_string(),
                "source_type": "file_read",
            }]
        }))
    }

    fn local_searcher(&self) -> LocalContextSearcher {
        LocalContextSearcher {
            workspace_manager: Arc::clone(&self.workspace_manager),
            note_manager: Arc::clone(&self.note_manager),
            history_manager: Arc::clone(&self.history_manager),
            builtin_registry: Arc::clone(&self.builtin_registry),
            model_manager: Arc::clone(&self.model_manager),
        }
    }

    fn dispatch_keynova_search(&self, args: &Value) -> Result<Value, String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
        let searcher = self.local_searcher();
        let mut sources: Vec<GroundingSource> = Vec::new();

        searcher.push_workspace_source(&mut sources);
        searcher.push_command_sources(&query, &mut sources)?;
        searcher.push_note_sources(&query, &mut sources)?;
        searcher.push_history_sources(&query, &mut sources)?;
        searcher.push_model_sources(&query, &mut sources);

        sources.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.source_type.cmp(&b.source_type))
                .then_with(|| a.title.cmp(&b.title))
        });
        sources.truncate(limit.max(1));
        Ok(grounding_to_tool_sources_json(&sources))
    }

    fn dispatch_web_search(&self, args: &Value) -> Result<Value, String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize;
        let sanitized = sanitize_external_query(&query)?;
        let (provider_str, searxng_url, api_key, timeout_secs) = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            (
                config
                    .get("agent.web_search_provider")
                    .unwrap_or_else(|| "disabled".into()),
                config.get("agent.searxng_url").unwrap_or_default(),
                config.get("agent.web_search_api_key").unwrap_or_default(),
                config
                    .get("agent.web_search_timeout_secs")
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(8),
            )
        };
        let provider = resolve_web_search_provider(&provider_str, &searxng_url, &api_key)?;
        let sources = provider.search(&sanitized, limit, timeout_secs)?;
        Ok(grounding_to_tool_sources_json(&sources))
    }

    fn default_search_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Ok(ws) = self.workspace_manager.lock() {
            if let Some(root) = ws.current().project_root.as_deref() {
                if !root.trim().is_empty() {
                    roots.push(PathBuf::from(root));
                }
            }
        }
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd);
        }
        roots.dedup();
        roots
    }

    /// Execute a fixed read-only `git status --short` in the workspace CWD.
    /// Called only after the user has explicitly approved the gate approval.
    fn dispatch_git_status(&self, args: &Value) -> Result<Value, String> {
        let cwd = args
            .get("cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                self.default_search_roots()
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
            });

        let output = std::process::Command::new("git")
            .args(["status", "--short"])
            .current_dir(&cwd)
            .output()
            .map_err(|e| format!("git.status: failed to run git: {e}"))?;

        Ok(json!({
            "cwd": cwd.display().to_string(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code(),
        }))
    }
}



impl AgentHandler {
    /// Build a `ToolDispatch` closure wiring all ReAct-compatible tools.
    ///
    /// The closure captures Arc clones of every dep needed and is `Send + Sync`,
    /// so it can safely be passed to `spawn_react_loop`.
    pub fn build_react_dispatch(&self) -> Arc<ToolDispatch> {
        let state = Arc::new(ReactDispatchState {
            workspace_manager: Arc::clone(&self.workspace_manager),
            config: Arc::clone(&self.config),
            note_manager: Arc::clone(&self.note_manager),
            history_manager: Arc::clone(&self.history_manager),
            builtin_registry: Arc::clone(&self.builtin_registry),
            model_manager: Arc::clone(&self.model_manager),
            knowledge_store: self.knowledge_store.clone(),
        });
        Arc::new(move |name: &str, args: &Value| state.dispatch(name, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::models::settings_schema::SettingValueType;

    #[test]
    fn rejects_private_context_in_web_query() {
        let error = sanitize_external_query("search tasks.md architecture").unwrap_err();
        assert!(error.contains("private"));
    }

    #[test]
    fn redacts_private_architecture_sources() {
        let source = visibility_filtered_source(
            "note:test".into(),
            "note",
            "tasks.md".into(),
            "phase 4 architecture".into(),
            1.0,
        );
        assert_eq!(source.visibility, ContextVisibility::PrivateArchitecture);
        assert!(source.redacted_reason.is_some());
    }

    #[test]
    fn redacts_secret_sources() {
        let source = visibility_filtered_source(
            "history:test".into(),
            "history",
            "Copied secret".into(),
            "api key = secret-value".into(),
            1.0,
        );
        assert_eq!(source.visibility, ContextVisibility::Secret);
        assert_eq!(source.snippet, "[redacted secret]");
    }

    #[test]
    fn builds_prompt_audit_with_budget_and_filters() {
        let audit = build_prompt_audit(
            "test prompt",
            &[
                source(
                    "workspace:1".into(),
                    "workspace",
                    "Workspace".into(),
                    "Public context".into(),
                    1.0,
                    ContextVisibility::PublicContext,
                ),
                visibility_filtered_source(
                    "note:1".into(),
                    "note",
                    "tasks.md".into(),
                    "architecture".into(),
                    0.8,
                ),
            ],
            64,
        );
        assert_eq!(audit.included_sources.len(), 1);
        assert_eq!(audit.filtered_sources.len(), 1);
    }

    #[test]
    fn extracts_integer_setting_value() {
        let value = extract_setting_value("set max results to 25", &SettingValueType::Integer);
        assert_eq!(value.as_deref(), Some("25"));
    }

    #[test]
    fn extracts_terminal_command_from_backticks() {
        let command = extract_shell_command("please run `cargo test` for me");
        assert_eq!(command.as_deref(), Some("cargo test"));
    }

    #[test]
    fn answers_capability_questions_locally() {
        let answer = direct_local_answer("你可以做到甚麼").expect("capability answer");
        assert!(answer.contains("我可以"));
        assert!(answer.contains("approval"));
    }

    #[test]
    fn answers_time_questions_locally() {
        let answer = direct_local_answer("顯示目前的詳細時間").expect("time answer");
        assert!(answer.contains("目前時間"));
    }

    #[test]
    fn extracts_chinese_directory_listing_target() {
        let target = extract_directory_list_target("幫我搜尋 hw 資料夾中有哪些資料夾");
        assert_eq!(target.as_deref(), Some("hw"));
    }

    #[test]
    fn answers_directory_listing_with_child_folders() {
        let root = std::env::temp_dir().join(format!("keynova-agent-test-{}", Uuid::new_v4()));
        let hw = root.join("hw");
        std::fs::create_dir_all(hw.join("week1")).expect("create week1");
        std::fs::create_dir_all(hw.join("week2")).expect("create week2");

        let answer = answer_directory_listing(
            "幫我搜尋 hw 資料夾中有哪些資料夾",
            std::slice::from_ref(&root),
        )
        .expect("directory answer");

        assert!(answer.contains("2 個子資料夾"));
        assert!(answer.contains("week1"));
        assert!(answer.contains("week2"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn directory_listing_reports_checked_paths_when_missing() {
        let root = std::env::temp_dir().join(format!("keynova-agent-missing-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");

        let answer = answer_directory_listing(
            "幫我搜尋 hw 資料夾中有哪些資料夾",
            std::slice::from_ref(&root),
        )
        .expect("missing directory answer");

        assert!(answer.contains("找不到 `hw`"));
        assert!(answer.contains(&root.join("hw").display().to_string()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn extracts_whole_computer_filesystem_search_query() {
        let query = extract_filesystem_search_query("please search whole computer for keynova");
        assert_eq!(query.as_deref(), Some("keynova"));
        assert!(wants_whole_computer_search("search whole computer keynova"));
    }

    #[test]
    fn filesystem_search_finds_matching_files_read_only() {
        let root = std::env::temp_dir().join(format!("keynova-agent-fs-search-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::write(root.join("homework.md"), "hello").expect("write test file");

        let outcome = search_filesystem("homework", std::slice::from_ref(&root), 5);

        assert_eq!(outcome.hits.len(), 1);
        assert!(!outcome.hits[0].is_dir);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn reads_text_file_preview_without_modifying() {
        let root = std::env::temp_dir().join(format!("keynova-agent-read-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        let path = root.join("note.txt");
        std::fs::write(&path, "read-only preview").expect("write test file");

        let answer = read_file_answer("note.txt", std::slice::from_ref(&root));

        assert!(answer.contains("read-only preview"));
        assert_eq!(
            std::fs::read_to_string(&path).expect("read test file"),
            "read-only preview"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn summarizes_project_types_from_markers() {
        let root =
            std::env::temp_dir().join(format!("keynova-agent-project-types-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("app-a")).expect("create app-a");
        std::fs::create_dir_all(root.join("app-b")).expect("create app-b");
        std::fs::create_dir_all(root.join("rust-a")).expect("create rust-a");
        std::fs::write(root.join("app-a").join("package.json"), "{}").expect("write package");
        std::fs::write(root.join("app-b").join("package.json"), "{}").expect("write package");
        std::fs::write(root.join("rust-a").join("Cargo.toml"), "[package]").expect("write cargo");

        let counts = scan_project_types(std::slice::from_ref(&root));
        let answer = format_project_type_summary(&counts);

        assert!(is_project_type_summary_prompt(
            "confirm which project types are most"
        ));
        assert!(answer.contains("JavaScript/TypeScript"));
        assert!(answer.contains("2"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn extracts_web_search_query_from_news_prompt() {
        let query = extract_web_search_query("please search web for technology news today");
        assert_eq!(query.as_deref(), Some("web for technology news today"));
    }

    #[test]
    fn parses_duckduckgo_html_result() {
        let html = r#"
            <a rel="nofollow" class="result__a" href="/l/?uddg=https%3A%2F%2Fexample.com%2Fnews">Example &amp; News</a>
            <a class="result__snippet">A short &amp; useful snippet.</a>
        "#;

        let results = parse_duckduckgo_html_results(html, 3);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example & News");
        assert_eq!(results[0].uri.as_deref(), Some("https://example.com/news"));
        assert!(results[0].snippet.contains("short & useful"));
    }

    #[test]
    fn parses_tavily_json_result() {
        let response = json!({
            "results": [
                {
                    "title": "Example News",
                    "url": "https://example.com/news",
                    "content": "Structured search result content.",
                    "score": 0.92
                }
            ]
        });

        let results = parse_tavily_response(&response, 3).expect("parse tavily");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_id, "web:tavily:0");
        assert_eq!(results[0].title, "Example News");
        assert_eq!(results[0].uri.as_deref(), Some("https://example.com/news"));
        assert!(results[0].snippet.contains("Structured search"));
    }

    #[test]
    fn parses_github_trending_html() {
        let html = r#"
            <article class="Box-row">
              <h2><a href="/openai/example-repo">openai / example-repo</a></h2>
              <p>Example trending repo.</p>
            </article>
        "#;

        let repos = parse_github_trending_html(html, 10);

        assert!(is_github_trending_prompt("today popular github projects"));
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].owner, "openai");
        assert_eq!(repos[0].name, "example-repo");
        assert_eq!(repos[0].url, "https://github.com/openai/example-repo");
    }

    #[test]
    fn creates_workflow_plan_for_task_prompt() {
        let answer = answer_workflow_plan("plan a task to organize files").expect("workflow plan");
        assert!(answer.contains("1."));
        assert!(answer.contains("approval"));
    }

    #[test]
    fn extracts_direct_command_like_terminal_request() {
        let command = extract_shell_command("please run npm run build");
        assert_eq!(command.as_deref(), Some("npm run build"));
    }

    #[test]
    fn does_not_treat_plain_start_as_terminal_command() {
        let command = extract_shell_command("start with project search and propose next steps");
        assert!(command.is_none());
    }

    #[test]
    fn prompt_audit_marks_truncated_sources() {
        let audit = build_prompt_audit(
            "a very long prompt that consumes the tiny budget",
            &[source(
                "workspace:1".into(),
                "workspace",
                "Workspace".into(),
                "Public context".into(),
                1.0,
                ContextVisibility::PublicContext,
            )],
            12,
        );
        assert!(audit.truncated);
        assert!(audit.included_sources.is_empty());
    }

    #[test]
    fn safe_builtin_allowlist_rejects_args() {
        assert!(is_allowlisted_safe_builtin("help", ""));
        assert!(!is_allowlisted_safe_builtin("help", "--danger"));
        assert!(!is_allowlisted_safe_builtin("rebuild_search_index", ""));
    }

    #[test]
    fn openai_provider_selects_react_loop() {
        use crate::managers::ai_manager::{provider_supports_tool_calls, resolve_ai_runtime_config};
        let pairs = [
            ("ai.provider", "openai"),
            ("ai.openai_api_key", "test-key"),
            ("ai.openai_base_url", "https://api.openai.com/v1"),
            ("ai.model", "gpt-4o-mini"),
        ];
        let rt = resolve_ai_runtime_config(|k| {
            pairs.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string())
        })
        .unwrap();
        assert!(provider_supports_tool_calls(&rt.provider));
    }

    #[test]
    fn claude_provider_selects_heuristic_fallback() {
        use crate::managers::ai_manager::{provider_supports_tool_calls, resolve_ai_runtime_config};
        let pairs = [
            ("ai.provider", "claude"),
            ("ai.api_key", "test-key"),
            ("ai.model", "claude-sonnet-4-6"),
        ];
        let rt = resolve_ai_runtime_config(|k| {
            pairs.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string())
        })
        .unwrap();
        assert!(!provider_supports_tool_calls(&rt.provider));
    }

    #[test]
    fn extract_quoted_prefers_double_over_single() {
        assert_eq!(
            extract_quoted(r#"read "config.toml" please"#).as_deref(),
            Some("config.toml")
        );
    }

    #[test]
    fn extract_quoted_falls_back_to_single_when_no_double() {
        assert_eq!(
            extract_quoted("read 'config.toml' please").as_deref(),
            Some("config.toml")
        );
    }

    #[test]
    fn extract_quoted_returns_none_for_no_quotes() {
        assert!(extract_quoted("read config.toml please").is_none());
    }

    #[test]
    fn truncate_appends_ellipsis_only_when_truncated() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn looks_sensitive_path_blocks_ssh_keys() {
        assert!(looks_sensitive_path(Path::new("/home/user/.ssh/id_rsa")));
        assert!(looks_sensitive_path(Path::new("C:\\Users\\user\\.env")));
        assert!(looks_sensitive_path(Path::new("/home/user/.aws/credentials")));
        assert!(!looks_sensitive_path(Path::new("/home/user/projects/main.rs")));
    }

    #[test]
    fn resolve_readable_path_rejects_out_of_workspace() {
        let root = std::env::temp_dir()
            .join(format!("keynova-path-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        // Attempting to read something outside the root using `..` traversal
        let err = resolve_readable_path("../../etc/passwd", &[root.clone()]).unwrap_err();
        // Could fail at "not found" or "outside workspace" — both are correct rejections
        assert!(
            err.contains("not found") || err.contains("outside workspace") || err.contains("cannot resolve"),
            "unexpected error: {err}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_readable_path_allows_in_workspace() {
        let root = std::env::temp_dir()
            .join(format!("keynova-path-ok-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create root");
        let file = root.join("note.txt");
        std::fs::write(&file, "hello").expect("write file");
        let resolved = resolve_readable_path("note.txt", &[root.clone()]).expect("should resolve");
        assert!(resolved.starts_with(root.canonicalize().unwrap()));
        let _ = std::fs::remove_dir_all(root);
    }
}
