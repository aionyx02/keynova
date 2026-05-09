use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::agent_runtime::ToolDispatch;
use crate::core::config_manager::ConfigManager;
use crate::core::{
    prepare_observation, AgentAuditEntry, AgentMemoryEntry, AgentObservationPolicy, AgentRuntime,
    BuiltinCommandRegistry, CommandHandler, CommandResult, KnowledgeStoreHandle,
};
use crate::managers::{
    history_manager::HistoryManager,
    model_manager::ModelManager,
    note_manager::NoteManager,
    system_indexer::{search_system_index, SystemSearchOutcome},
    workspace_manager::WorkspaceManager,
};
use crate::models::action::ActionRisk;
use crate::models::agent::{
    AgentActionKind, AgentApproval, AgentFilteredSource, AgentMemoryRef, AgentMemoryScope,
    AgentPlannedAction, AgentPromptAudit, AgentRun, AgentRunStatus, AgentStep, AgentToolCall,
    ContextVisibility, GroundingSource,
};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::models::settings_schema::{builtin_setting_schema, SettingValueType};
use crate::models::terminal::TerminalLaunchSpec;

const PROMPT_BUDGET_CHARS: usize = 1400;
const PROMPT_SOURCE_LIMIT: usize = 6;
const SESSION_MEMORY_LIMIT: usize = 3;
const LONG_TERM_MEMORY_LIMIT: usize = 3;

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

    fn keynova_search(&self, query: &str, limit: usize) -> Result<Vec<GroundingSource>, String> {
        let mut sources = Vec::new();
        let q = query.to_lowercase();

        self.push_workspace_source(&mut sources);
        self.push_command_sources(&q, &mut sources)?;
        self.push_setting_schema_sources(&q, &mut sources);
        self.push_model_sources(&q, &mut sources);
        self.push_note_sources(&q, &mut sources)?;
        self.push_history_sources(query, &mut sources)?;

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

    fn web_search(&self, query: &str, limit: usize) -> Result<Vec<GroundingSource>, String> {
        let sanitized = sanitize_external_query(query)?;
        let (provider, searxng_url, api_key, timeout_secs) = {
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

        match provider.as_str() {
            "searxng" => search_searxng(&searxng_url, &sanitized, limit, timeout_secs),
            "tavily" => search_tavily(&api_key, &sanitized, limit, timeout_secs),
            "duckduckgo" => search_duckduckgo_html(&sanitized, limit, timeout_secs),
            "disabled" | "" => Err(
                "web.search provider is disabled; configure agent.web_search_provider=searxng or tavily. DuckDuckGo is an explicit best-effort fallback.".into(),
            ),
            other => Err(format!("web.search provider '{other}' is not supported yet")),
        }
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

fn search_searxng(
    base_url: &str,
    query: &str,
    limit: usize,
    timeout_secs: u64,
) -> Result<Vec<GroundingSource>, String> {
    if base_url.trim().is_empty() {
        return Err("agent.searxng_url is required for web.search".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/search", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .query(&[("q", query), ("format", "json")])
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .map_err(|e| e.to_string())?;
    let results = response
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| "searxng response missing results".to_string())?;
    Ok(results
        .iter()
        .take(limit.max(1))
        .enumerate()
        .map(|(index, item)| {
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled")
                .to_string();
            let url = item
                .get("url")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let snippet = item
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            GroundingSource {
                source_id: format!("web:searxng:{index}"),
                source_type: "web".into(),
                title,
                snippet: truncate(&snippet, 240),
                uri: url,
                score: 1.0 - (index as f32 * 0.03),
                visibility: ContextVisibility::PublicContext,
                redacted_reason: None,
            }
        })
        .collect())
}

fn search_tavily(
    api_key: &str,
    query: &str,
    limit: usize,
    timeout_secs: u64,
) -> Result<Vec<GroundingSource>, String> {
    if api_key.trim().is_empty() {
        return Err("agent.web_search_api_key is required for Tavily web.search".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .user_agent("Keynova/0.1 structured agent search")
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .post("https://api.tavily.com/search")
        .header("content-type", "application/json")
        .json(&json!({
            "api_key": api_key,
            "query": query,
            "search_depth": "basic",
            "max_results": limit.max(1),
            "include_answer": false,
            "include_raw_content": false,
        }))
        .send()
        .map_err(|e| format!("Tavily request failed: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .map_err(|e| e.to_string())?;
    parse_tavily_response(&response, limit)
}

fn parse_tavily_response(response: &Value, limit: usize) -> Result<Vec<GroundingSource>, String> {
    let results = response
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| "Tavily response missing results".to_string())?;
    Ok(results
        .iter()
        .take(limit.max(1))
        .enumerate()
        .map(|(index, item)| {
            let title = item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled")
                .to_string();
            let url = item
                .get("url")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            let snippet = item
                .get("content")
                .or_else(|| item.get("snippet"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let score = item
                .get("score")
                .and_then(Value::as_f64)
                .map(|value| value as f32)
                .unwrap_or(1.0 - (index as f32 * 0.03));
            GroundingSource {
                source_id: format!("web:tavily:{index}"),
                source_type: "web".into(),
                title,
                snippet: truncate(&snippet, 240),
                uri: url,
                score,
                visibility: ContextVisibility::PublicContext,
                redacted_reason: None,
            }
        })
        .collect())
}

fn search_duckduckgo_html(
    query: &str,
    limit: usize,
    timeout_secs: u64,
) -> Result<Vec<GroundingSource>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .user_agent("Keynova/0.1 read-only agent search")
        .build()
        .map_err(|e| e.to_string())?;
    let html = client
        .get("https://duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .map_err(|e| format!("DuckDuckGo request failed: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())?;
    let results = parse_duckduckgo_html_results(&html, limit.max(1));
    if results.is_empty() {
        return Err("DuckDuckGo returned no parseable results".into());
    }
    Ok(results)
}

fn parse_duckduckgo_html_results(html: &str, limit: usize) -> Vec<GroundingSource> {
    let mut results = Vec::new();
    let mut rest = html;
    while results.len() < limit {
        let Some(link_pos) = rest.find("result__a") else {
            break;
        };
        rest = &rest[link_pos..];
        let Some(href_pos) = rest.find("href=\"") else {
            break;
        };
        let href_start = href_pos + "href=\"".len();
        let Some(href_end) = rest[href_start..].find('"') else {
            break;
        };
        let raw_href = &rest[href_start..href_start + href_end];
        let Some(text_start_rel) = rest[href_start + href_end..].find('>') else {
            break;
        };
        let text_start = href_start + href_end + text_start_rel + 1;
        let Some(text_end_rel) = rest[text_start..].find("</a>") else {
            break;
        };
        let title = strip_html(&rest[text_start..text_start + text_end_rel]);
        rest = &rest[text_start + text_end_rel..];

        let snippet = if let Some(snippet_pos) = rest.find("result__snippet") {
            let snippet_rest = &rest[snippet_pos..];
            if let Some(start_rel) = snippet_rest.find('>') {
                if let Some(end_rel) = snippet_rest[start_rel + 1..].find("</a>") {
                    strip_html(&snippet_rest[start_rel + 1..start_rel + 1 + end_rel])
                } else if let Some(end_rel) = snippet_rest[start_rel + 1..].find("</div>") {
                    strip_html(&snippet_rest[start_rel + 1..start_rel + 1 + end_rel])
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if title.trim().is_empty() {
            continue;
        }
        results.push(GroundingSource {
            source_id: format!("web:duckduckgo:{}", results.len()),
            source_type: "web".into(),
            title,
            snippet: truncate(&snippet, 240),
            uri: Some(normalize_duckduckgo_href(raw_href)),
            score: 1.0 - (results.len() as f32 * 0.03),
            visibility: ContextVisibility::PublicContext,
            redacted_reason: None,
        });
    }
    results
}

fn normalize_duckduckgo_href(raw_href: &str) -> String {
    let decoded = decode_html_entities(raw_href);
    if let Some(index) = decoded.find("uddg=") {
        let encoded = &decoded[index + "uddg=".len()..];
        let value = encoded.split('&').next().unwrap_or(encoded);
        return percent_decode(value);
    }
    decoded
}

fn strip_html(value: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    decode_html_entities(out.trim())
}

fn decode_html_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                    continue;
                }
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn build_prompt_audit(
    prompt: &str,
    sources: &[GroundingSource],
    budget_chars: usize,
) -> AgentPromptAudit {
    let mut remaining = budget_chars.saturating_sub(prompt.chars().count());
    let mut included_sources = Vec::new();
    let mut filtered_sources = Vec::new();
    let mut truncated = false;
    let mut redacted_secret_count = 0usize;

    for source in sources {
        match source.visibility {
            ContextVisibility::PublicContext | ContextVisibility::UserPrivate => {
                let weight = source.title.chars().count() + source.snippet.chars().count() + 24;
                if included_sources.len() >= PROMPT_SOURCE_LIMIT || weight > remaining {
                    truncated = true;
                    continue;
                }
                remaining = remaining.saturating_sub(weight);
                included_sources.push(source.clone());
            }
            ContextVisibility::PrivateArchitecture | ContextVisibility::Secret => {
                if source.visibility == ContextVisibility::Secret {
                    redacted_secret_count += 1;
                }
                filtered_sources.push(AgentFilteredSource {
                    source_id: source.source_id.clone(),
                    source_type: source.source_type.clone(),
                    title: source.title.clone(),
                    visibility: source.visibility,
                    reason: source
                        .redacted_reason
                        .clone()
                        .unwrap_or_else(|| "filtered".into()),
                });
            }
        }
    }

    let prompt_chars = prompt.chars().count()
        + included_sources
            .iter()
            .map(|source| source.title.chars().count() + source.snippet.chars().count())
            .sum::<usize>();

    AgentPromptAudit {
        budget_chars,
        prompt_chars,
        truncated,
        included_sources,
        filtered_sources,
        redacted_secret_count,
    }
}

fn direct_local_answer(prompt: &str) -> Option<String> {
    if is_capability_question(prompt) {
        return Some(capability_answer());
    }
    if is_time_question(prompt) {
        let now = chrono::Local::now();
        return Some(format!(
            "目前時間是 {}。\n\n如果你想要我每次都用更完整格式顯示，可以問「顯示目前的詳細時間」；如果要開啟/執行本機動作，我會先列出 approval。",
            now.format("%Y-%m-%d %H:%M:%S %:z")
        ));
    }
    None
}

fn answer_directory_listing(prompt: &str, roots: &[PathBuf]) -> Option<String> {
    let target = extract_directory_list_target(prompt)?;
    let (found, checked) = resolve_directory_target(&target, roots);
    let Some(path) = found else {
        let checked_text = if checked.is_empty() {
            "沒有可用的搜尋根目錄。".to_string()
        } else {
            checked
                .iter()
                .take(8)
                .map(|path| format!("- {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        return Some(format!(
            "我找不到 `{target}` 資料夾。\n\n我已檢查：\n{checked_text}\n\n如果它在其他位置，可以用完整路徑再問一次，例如「列出 `C:\\path\\to\\hw` 裡的資料夾」。"
        ));
    };

    let mut directories = std::fs::read_dir(&path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let file_type = entry.file_type().ok()?;
                    if !file_type.is_dir() {
                        return None;
                    }
                    Some(entry.file_name().to_string_lossy().to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    directories.sort_by_key(|name| name.to_lowercase());

    if directories.is_empty() {
        return Some(format!(
            "我找到了 `{target}` 資料夾：\n{}\n\n它目前沒有子資料夾。",
            path.display()
        ));
    }

    let list = directories
        .iter()
        .map(|name| format!("- {name}"))
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!(
        "我找到了 `{target}` 資料夾：\n{}\n\n裡面有 {} 個子資料夾：\n{}",
        path.display(),
        directories.len(),
        list
    ))
}

#[derive(Debug, Clone)]
#[cfg(test)]
#[allow(dead_code)]
struct FileSearchHit {
    path: PathBuf,
    is_dir: bool,
}

#[derive(Debug, Clone)]
#[cfg(test)]
#[allow(dead_code)]
struct FileSearchOutcome {
    hits: Vec<FileSearchHit>,
    checked_roots: Vec<PathBuf>,
    visited: usize,
    stopped_early: bool,
}

#[cfg(test)]
fn search_filesystem(query: &str, roots: &[PathBuf], limit: usize) -> FileSearchOutcome {
    let normalized = query.to_lowercase();
    let started = Instant::now();
    let mut hits = Vec::new();
    let mut visited = 0usize;
    let mut stopped_early = false;
    let max_results = limit.max(1);
    let max_visited = if wants_whole_computer_search(query) {
        40_000
    } else {
        12_000
    };

    for root in roots {
        let mut stack = vec![root.clone()];
        while let Some(path) = stack.pop() {
            if visited >= max_visited || started.elapsed() > Duration::from_millis(3500) {
                stopped_early = true;
                break;
            }
            visited += 1;

            let Ok(metadata) = std::fs::metadata(&path) else {
                continue;
            };
            let is_dir = metadata.is_dir();
            if path_matches_query(&path, &normalized) {
                hits.push(FileSearchHit {
                    path: path.clone(),
                    is_dir,
                });
                if hits.len() >= max_results {
                    stopped_early = true;
                    break;
                }
            }

            if !is_dir || should_skip_directory(&path) {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok) {
                stack.push(entry.path());
            }
        }
        if stopped_early || hits.len() >= max_results {
            break;
        }
    }

    FileSearchOutcome {
        hits,
        checked_roots: roots.to_vec(),
        visited,
        stopped_early,
    }
}

#[cfg(test)]
fn path_matches_query(path: &Path, query: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_lowercase().contains(query))
}

#[cfg(test)]
#[allow(dead_code)]
fn format_filesystem_search_answer(query: &str, outcome: &FileSearchOutcome) -> String {
    let roots = if outcome.checked_roots.is_empty() {
        "- 沒有可用搜尋根目錄".to_string()
    } else {
        outcome
            .checked_roots
            .iter()
            .take(8)
            .map(|root| format!("- {}", root.display()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    if outcome.hits.is_empty() {
        return format!(
            "我沒有找到符合 `{query}` 的檔案或資料夾。\n\n已檢查 {} 個項目，搜尋根目錄：\n{}",
            outcome.visited, roots
        );
    }

    let list = outcome
        .hits
        .iter()
        .map(|hit| {
            format!(
                "- [{}] {}",
                if hit.is_dir { "folder" } else { "file" },
                hit.path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let suffix = if outcome.stopped_early {
        "\n\n結果已達上限或時間上限，這是 bounded read-only 搜尋；你可以提供更精準關鍵字縮小範圍。"
    } else {
        ""
    };
    format!(
        "我找到 {} 個符合 `{query}` 的檔案/資料夾：\n{}\n\n已檢查 {} 個項目。{}",
        outcome.hits.len(),
        list,
        outcome.visited,
        suffix
    )
}

fn format_system_index_search_answer(query: &str, outcome: &SystemSearchOutcome) -> String {
    let diagnostics = &outcome.diagnostics;
    let mut diagnostic_parts = vec![format!("provider={}", diagnostics.provider)];
    if let Some(reason) = diagnostics.fallback_reason.as_deref() {
        diagnostic_parts.push(format!("fallback={reason}"));
    }
    if diagnostics.visited > 0 {
        diagnostic_parts.push(format!("visited={}", diagnostics.visited));
    }
    if diagnostics.permission_denied > 0 {
        diagnostic_parts.push(format!(
            "permission_denied={}",
            diagnostics.permission_denied
        ));
    }
    if let Some(age) = diagnostics.index_age_secs {
        diagnostic_parts.push(format!("index_age_secs={age}"));
    }
    if diagnostics.timed_out {
        diagnostic_parts.push("timed_out=true".into());
    }
    if let Some(message) = diagnostics.message.as_deref() {
        diagnostic_parts.push(format!("message={message}"));
    }
    let diagnostics = diagnostic_parts.join(", ");

    if outcome.hits.is_empty() {
        return format!("No filesystem matches found for `{query}`.\n\nDiagnostics: {diagnostics}");
    }

    let list = outcome
        .hits
        .iter()
        .map(|hit| {
            format!(
                "- [{}] {}",
                if hit.is_dir { "folder" } else { "file" },
                hit.path
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Found {} filesystem matches for `{query}`.\n{}\n\nDiagnostics: {}",
        outcome.hits.len(),
        list,
        diagnostics
    )
}

fn read_file_answer(target: &str, roots: &[PathBuf]) -> String {
    let (found, checked) = resolve_file_target(target, roots);
    let Some(path) = found else {
        let checked_text = checked
            .iter()
            .take(8)
            .map(|path| format!("- {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        return format!(
            "我找不到 `{target}` 這個檔案。\n\n我已檢查：\n{}",
            if checked_text.is_empty() {
                "- 沒有可用搜尋根目錄"
            } else {
                checked_text.as_str()
            }
        );
    };

    match read_text_preview(&path, 12_000) {
        Ok(preview) => format!(
            "我讀取了：\n{}\n\n內容預覽：\n```text\n{}\n```",
            path.display(),
            preview
        ),
        Err(error) => format!("我找到了 `{}`，但無法讀取文字內容：{error}", path.display()),
    }
}

fn read_text_preview(path: &Path, max_chars: usize) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("目標不是檔案".into());
    }
    const MAX_BYTES: u64 = 512 * 1024;
    if metadata.len() > MAX_BYTES {
        return Err(format!(
            "檔案大小 {} bytes 超過 read-only 預覽上限 {} bytes",
            metadata.len(),
            MAX_BYTES
        ));
    }
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(truncate(&content, max_chars))
}

#[derive(Debug, Clone)]
struct ProjectTypeCount {
    name: &'static str,
    count: usize,
    samples: Vec<PathBuf>,
}

fn is_project_type_summary_prompt(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("project type")
        || lower.contains("project types")
        || lower.contains("repo type")
        || lower.contains("repo types")
        || lower.contains("專案類型")
        || lower.contains("项目类型")
        || (lower.contains("專案") && lower.contains("最多"))
        || (lower.contains("项目") && lower.contains("最多"))
}

fn scan_project_types(roots: &[PathBuf]) -> Vec<ProjectTypeCount> {
    let started = Instant::now();
    let mut counts = project_type_markers()
        .iter()
        .map(|(name, _)| ProjectTypeCount {
            name,
            count: 0,
            samples: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    let max_visited = 60_000usize;

    for root in roots {
        let mut stack = vec![root.clone()];
        while let Some(path) = stack.pop() {
            if visited >= max_visited || started.elapsed() > Duration::from_millis(4500) {
                break;
            }
            visited += 1;
            if should_skip_directory(&path) {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&path) else {
                continue;
            };
            let mut child_dirs = Vec::new();
            let mut files = Vec::new();
            for entry in entries.filter_map(Result::ok) {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_dir() {
                    child_dirs.push(entry.path());
                } else if file_type.is_file() {
                    files.push(entry.file_name().to_string_lossy().to_lowercase());
                }
            }

            for (index, (_, markers)) in project_type_markers().iter().enumerate() {
                if markers.iter().any(|marker| {
                    if marker.starts_with('.') {
                        files.iter().any(|file| file.ends_with(marker))
                    } else {
                        files.iter().any(|file| file == marker)
                    }
                }) {
                    counts[index].count += 1;
                    if counts[index].samples.len() < 3 {
                        counts[index].samples.push(path.clone());
                    }
                }
            }
            stack.extend(child_dirs);
        }
    }

    counts.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(right.name))
    });
    counts
}

fn project_type_markers() -> &'static [(&'static str, &'static [&'static str])] {
    &[
        ("JavaScript/TypeScript", &["package.json"]),
        ("Rust", &["cargo.toml"]),
        (
            "Python",
            &["pyproject.toml", "requirements.txt", "setup.py"],
        ),
        ("Go", &["go.mod"]),
        (
            "Java/Kotlin",
            &["pom.xml", "build.gradle", "build.gradle.kts"],
        ),
        ("C#/.NET", &[".sln", ".csproj"]),
        ("PHP", &["composer.json"]),
        ("Ruby", &["gemfile"]),
        ("Dart/Flutter", &["pubspec.yaml"]),
        ("C/C++", &["cmakelists.txt", "makefile"]),
    ]
}

fn format_project_type_summary(counts: &[ProjectTypeCount]) -> String {
    let non_zero = counts
        .iter()
        .filter(|item| item.count > 0)
        .collect::<Vec<_>>();
    if non_zero.is_empty() {
        return "我做了 bounded read-only 專案類型掃描，但沒有找到常見專案 marker，例如 package.json、Cargo.toml、pyproject.toml、go.mod。".into();
    }
    let list = non_zero
        .iter()
        .take(8)
        .map(|item| {
            let samples = item
                .samples
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "- {}: {} 個{}",
                item.name,
                item.count,
                if samples.is_empty() {
                    String::new()
                } else {
                    format!("，例：{samples}")
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let top = non_zero[0];
    format!(
        "目前 bounded read-only 掃描結果顯示，最多的是 {} 專案，共 {} 個。\n\n統計：\n{}\n\n判斷依據是常見 marker 檔，例如 package.json、Cargo.toml、pyproject.toml、go.mod；不會修改任何檔案。",
        top.name,
        top.count,
        list
    )
}

fn extract_directory_list_target(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "folder",
            "folders",
            "directory",
            "directories",
            "資料夾",
            "文件夾",
        ],
    ) {
        return None;
    }
    if !contains_any(
        &lower,
        &[
            "list",
            "find",
            "search",
            "what",
            "which",
            "under",
            "inside",
            "有哪些",
            "哪些",
            "列出",
            "搜尋",
            "搜索",
            "找",
            "裡面",
            "里面",
            "中有",
        ],
    ) {
        return None;
    }

    if let Some(value) = extract_backticked(prompt).or_else(|| extract_quoted(prompt)) {
        return Some(value);
    }
    if let Some(path) = extract_path_like(prompt) {
        return Some(path);
    }

    for marker in ["資料夾", "文件夾", " folder", " directory"] {
        if let Some(index) = lower.find(marker) {
            if let Some(target) = last_target_token(&prompt[..index]) {
                return Some(target);
            }
        }
    }

    for marker in [" in ", " under ", " inside "] {
        if let Some(index) = lower.find(marker) {
            let rest = &prompt[index + marker.len()..];
            if let Some(target) = first_target_token(rest) {
                return Some(target);
            }
        }
    }

    None
}

fn extract_filesystem_search_query(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "filesystem",
            "file system",
            "whole computer",
            "entire computer",
            "all computer",
            "all drives",
            "全電腦",
            "全电脑",
            "整台",
            "所有磁碟",
            "所有硬碟",
            "檔案",
            "文件",
            "資料",
        ],
    ) || !contains_any(
        &lower,
        &[
            "search", "find", "look for", "搜尋", "搜索", "尋找", "查找", "找",
        ],
    ) {
        return None;
    }

    if let Some(value) = extract_backticked(prompt).or_else(|| extract_quoted(prompt)) {
        return Some(value);
    }

    let mut best = None;
    for token in prompt
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | ':' | '：'))
        .map(clean_target_token)
    {
        let lower = token.to_lowercase();
        if token.len() >= 2
            && !matches!(
                lower.as_str(),
                "幫我"
                    | "帮我"
                    | "搜尋"
                    | "搜索"
                    | "尋找"
                    | "查找"
                    | "找"
                    | "全電腦"
                    | "全电脑"
                    | "整台"
                    | "資料"
                    | "檔案"
                    | "文件"
                    | "file"
                    | "files"
                    | "data"
                    | "computer"
                    | "whole"
                    | "entire"
            )
        {
            best = Some(token);
        }
    }
    best
}

fn extract_file_read_target(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "read", "open", "show", "cat", "讀取", "读取", "打開", "打开", "顯示", "显示",
        ],
    ) {
        return None;
    }
    extract_backticked(prompt)
        .or_else(|| extract_quoted(prompt))
        .or_else(|| extract_path_like(prompt))
}

fn resolve_directory_target(target: &str, roots: &[PathBuf]) -> (Option<PathBuf>, Vec<PathBuf>) {
    let target_path = PathBuf::from(target);
    let mut checked = Vec::new();
    if target_path.is_absolute() {
        checked.push(target_path.clone());
        return (target_path.is_dir().then_some(target_path), checked);
    }

    for root in roots {
        let candidate = root.join(target);
        checked.push(candidate.clone());
        if candidate.is_dir() {
            return (Some(candidate), checked);
        }
    }

    if target_path.components().count() == 1 {
        for root in roots {
            if let Some(found) = find_directory_by_name(root, target, 4, &mut checked) {
                return (Some(found), checked);
            }
        }
    }

    (None, checked)
}

fn resolve_file_target(target: &str, roots: &[PathBuf]) -> (Option<PathBuf>, Vec<PathBuf>) {
    let target_path = PathBuf::from(target);
    let mut checked = Vec::new();
    if target_path.is_absolute() {
        checked.push(target_path.clone());
        return (target_path.is_file().then_some(target_path), checked);
    }

    for root in roots {
        let candidate = root.join(target);
        checked.push(candidate.clone());
        if candidate.is_file() {
            return (Some(candidate), checked);
        }
    }

    if target_path.components().count() == 1 {
        for root in roots {
            if let Some(found) = find_file_by_name(root, target, 4, &mut checked) {
                return (Some(found), checked);
            }
        }
    }

    (None, checked)
}

fn find_directory_by_name(
    root: &Path,
    target: &str,
    max_depth: usize,
    checked: &mut Vec<PathBuf>,
) -> Option<PathBuf> {
    if max_depth == 0 || should_skip_directory(root) {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        checked.push(path.clone());
        let name = entry.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case(target) {
            return Some(path);
        }
        if let Some(found) = find_directory_by_name(&path, target, max_depth - 1, checked) {
            return Some(found);
        }
    }
    None
}

fn find_file_by_name(
    root: &Path,
    target: &str,
    max_depth: usize,
    checked: &mut Vec<PathBuf>,
) -> Option<PathBuf> {
    if max_depth == 0 || should_skip_directory(root) {
        return None;
    }
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        checked.push(path.clone());
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_file() && name.eq_ignore_ascii_case(target) {
            return Some(path);
        }
        if file_type.is_dir() {
            if let Some(found) = find_file_by_name(&path, target, max_depth - 1, checked) {
                return Some(found);
            }
        }
    }
    None
}

fn should_skip_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                ".git" | ".idea" | "node_modules" | "target" | "dist"
            )
        })
}

fn last_target_token(value: &str) -> Option<String> {
    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | ':' | '：'))
        .rev()
        .map(clean_target_token)
        .find(|token| is_meaningful_target(token))
}

fn first_target_token(value: &str) -> Option<String> {
    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | ':' | '：'))
        .map(clean_target_token)
        .find(|token| is_meaningful_target(token))
}

fn clean_target_token(value: &str) -> String {
    value
        .trim_matches(|ch| {
            matches!(
                ch,
                '"' | '\'' | '`' | ',' | '.' | '?' | '!' | '，' | '。' | '？' | '！' | '「' | '」'
            )
        })
        .trim_end_matches("裡")
        .trim_end_matches("里")
        .trim_end_matches("中")
        .to_string()
}

fn is_meaningful_target(value: &str) -> bool {
    let lower = value.to_lowercase();
    !value.trim().is_empty()
        && !matches!(
            lower.as_str(),
            "幫我"
                | "帮我"
                | "搜尋"
                | "搜索"
                | "查詢"
                | "查询"
                | "找"
                | "列出"
                | "有哪些"
                | "哪些"
                | "folder"
                | "folders"
                | "directory"
                | "directories"
                | "in"
                | "under"
                | "inside"
        )
}

fn capability_answer() -> String {
    [
        "我可以用兩種方式幫你：",
        "",
        "1. 直接回答本機可判斷的小問題，例如目前時間、Keynova 能力、下一步建議。",
        "2. 先規劃再等你批准本機動作，例如開 panel、建立筆記草稿、修改設定草稿、執行安全內建命令，或準備 terminal/file/system/model 這類高風險動作。",
        "3. 用 read-only tools 搜尋 Keynova context，並把 private architecture / secrets 留在本機 audit，不送進外部查詢。",
        "",
        "目前還沒完全像一般 agent 的原因是：Agent mode 尚未把 `ai.chat` provider 接回 `agent.run`，所以一般開放式聊天仍比較適合 Chat mode。下一步要做的是 Ask/Act 合一：一般問題走 AI provider，需要本機動作時才切 approval。",
    ]
    .join("\n")
}

fn build_plan(
    prompt: &str,
    action: Option<&AgentPlannedAction>,
    has_direct_answer: bool,
) -> Vec<String> {
    let mut plan = vec![
        "Classify requested context by visibility before building any prompt".into(),
        "Use read-only tools first and keep private architecture and secrets out of model context"
            .into(),
    ];
    if let Some(action) = action {
        plan.push(format!(
            "Prepare '{}' as a {:?} action and wait for explicit approval",
            action.label, action.risk
        ));
    } else if has_direct_answer {
        plan.push("Answer directly from Keynova's local agent runtime".into());
    } else if should_run_local_search(prompt) {
        plan.push("Return a grounded plan using local Keynova context only".into());
    } else {
        plan.push("Explain how to ask for chat, search, or approved local actions".into());
    }
    plan
}

fn describe_run(prompt: &str, audit: &AgentPromptAudit) -> String {
    if should_run_local_search(prompt) && !audit.included_sources.is_empty() {
        return format!(
            "我找到 {} 個可用的 Keynova 參考來源。你可以展開 Context audit 看細節，或直接要求我「開啟設定」、「建立筆記草稿」、「搜尋 note/history/model」這類可批准動作。",
            audit.included_sources.len()
        );
    }
    if audit.filtered_sources.is_empty() {
        "我目前沒有偵測到需要執行的本機動作。你可以直接問「你可以做什麼」、「現在幾點」，或用更明確的動作語氣，例如「建立一篇筆記草稿」、「開啟設定」、「執行 `npm run build`」。一般聊天式回答會在下一步接上 AI provider 後變得更像標準 agent。".into()
    } else {
        format!(
            "我已完成安全檢查，但這次沒有可執行動作。{} 個來源因為 private/secret 規則被保留在本機 audit 中，沒有放進可用上下文。",
            audit.filtered_sources.len()
        )
    }
}

fn describe_execution(action: &AgentPlannedAction, result: &BuiltinCommandResult) -> String {
    match &result.ui_type {
        CommandUiType::Inline => format!("Executed '{}'. {}", action.label, result.text),
        CommandUiType::Panel(panel) => {
            format!("Executed '{}'. Opened panel '{}'.", action.label, panel)
        }
        CommandUiType::Terminal(spec) => format!(
            "Executed '{}'. Ready to run terminal command '{}'.",
            action.label, spec.program
        ),
    }
}

fn visibility_filtered_source(
    source_id: String,
    source_type: &str,
    title: String,
    snippet: String,
    score: f32,
) -> GroundingSource {
    let combined = format!("{title} {snippet}").to_lowercase();
    if contains_any(
        &combined,
        &[
            "claude.md",
            "tasks.md",
            "memory.md",
            "decisions.md",
            "skill.md",
            "private_architecture",
            "architecture",
        ],
    ) {
        return GroundingSource {
            source_id,
            source_type: source_type.into(),
            title,
            snippet: "[redacted private architecture context]".into(),
            uri: None,
            score,
            visibility: ContextVisibility::PrivateArchitecture,
            redacted_reason: Some("private_architecture".into()),
        };
    }
    if contains_any(
        &combined,
        &[
            "api_key", "api key", "password", "token", "secret", "sk-", "bearer ",
        ],
    ) {
        return GroundingSource {
            source_id,
            source_type: source_type.into(),
            title,
            snippet: "[redacted secret]".into(),
            uri: None,
            score,
            visibility: ContextVisibility::Secret,
            redacted_reason: Some("secret".into()),
        };
    }
    source(
        source_id,
        source_type,
        title,
        snippet,
        score,
        ContextVisibility::UserPrivate,
    )
}

fn source(
    source_id: String,
    source_type: &str,
    title: String,
    snippet: String,
    score: f32,
    visibility: ContextVisibility,
) -> GroundingSource {
    GroundingSource {
        source_id,
        source_type: source_type.into(),
        title,
        snippet: truncate(&snippet, 240),
        uri: None,
        score,
        visibility,
        redacted_reason: None,
    }
}

fn parse_visibility(value: &str) -> ContextVisibility {
    match value {
        "public_context" => ContextVisibility::PublicContext,
        "private_architecture" => ContextVisibility::PrivateArchitecture,
        "secret" => ContextVisibility::Secret,
        _ => ContextVisibility::UserPrivate,
    }
}

fn suggested_note_name(prompt: &str) -> String {
    if let Some(quoted) = extract_quoted(prompt) {
        return quoted.replace(' ', "_");
    }
    let words = prompt
        .split_whitespace()
        .filter(|word| word.chars().any(|ch| ch.is_alphanumeric()))
        .take(4)
        .collect::<Vec<_>>();
    if words.is_empty() {
        "agent_draft".into()
    } else {
        words.join("_").to_lowercase()
    }
}

fn title_case(value: &str) -> String {
    value
        .split(['_', '-', ' '])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn match_setting_schema(prompt: &str) -> Option<crate::models::settings_schema::SettingSchema> {
    let q = prompt.to_lowercase();
    builtin_setting_schema()
        .into_iter()
        .filter(|schema| !schema.sensitive)
        .map(|schema| {
            let mut score = 0i32;
            if q.contains(schema.key) {
                score += 4;
            }
            for token in schema.key.split('.') {
                if token.len() >= 3 && q.contains(token) {
                    score += 1;
                }
            }
            for token in schema.label.to_lowercase().split_whitespace() {
                if token.len() >= 3 && q.contains(token) {
                    score += 1;
                }
            }
            (score, schema)
        })
        .filter(|(score, _)| *score >= 2)
        .max_by_key(|(score, _)| *score)
        .map(|(_, schema)| schema)
}

fn extract_setting_value(prompt: &str, value_type: &SettingValueType) -> Option<String> {
    let lower = prompt.to_lowercase();
    match value_type {
        SettingValueType::Boolean => {
            if contains_any(&lower, &["true", "enable", "enabled", "on", "開", "开启"]) {
                Some("true".into())
            } else if contains_any(
                &lower,
                &["false", "disable", "disabled", "off", "關", "关闭"],
            ) {
                Some("false".into())
            } else {
                None
            }
        }
        SettingValueType::Integer => prompt
            .split(|ch: char| !ch.is_ascii_digit())
            .rfind(|part| !part.is_empty())
            .map(ToOwned::to_owned),
        _ => extract_quoted(prompt).or_else(|| {
            prompt
                .split_whitespace()
                .last()
                .map(|value| value.trim_matches(|ch| ch == '.' || ch == ',').to_string())
        }),
    }
}

fn extract_shell_command(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "run ", "execute ", "start ", "terminal", "cargo ", "npm ", "git ", "pnpm ", "python ",
            "執行", "运行",
        ],
    ) {
        return None;
    }
    if let Some(quoted) = extract_backticked(prompt) {
        return Some(quoted);
    }
    for marker in ["run ", "execute ", "start ", "執行", "运行"] {
        if let Some(index) = lower.find(marker) {
            let value = prompt[index + marker.len()..].trim();
            if looks_like_shell_command(value) {
                return Some(value.to_string());
            }
        }
    }
    for marker in [
        "cargo ", "npm ", "git ", "pnpm ", "python ", "python3 ", "node ", "npx ", "yarn ", "bun ",
    ] {
        if let Some(index) = lower.find(marker) {
            let value = prompt[index..].trim();
            if looks_like_shell_command(value) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn looks_like_shell_command(value: &str) -> bool {
    let command = value
        .trim()
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | ':' | ',' | '.'));
    if command.is_empty() {
        return false;
    }
    let Some(program) = command.split_whitespace().next() else {
        return false;
    };
    let program = program
        .trim_matches(|ch| matches!(ch, '"' | '\'' | '`'))
        .to_lowercase();

    if program.starts_with("./")
        || program.starts_with(".\\")
        || program.contains('\\')
        || program.ends_with(".exe")
        || program.ends_with(".cmd")
        || program.ends_with(".bat")
        || program.ends_with(".ps1")
        || program.ends_with(".sh")
    {
        return true;
    }

    matches!(
        program.as_str(),
        "cargo"
            | "npm"
            | "pnpm"
            | "yarn"
            | "bun"
            | "npx"
            | "node"
            | "git"
            | "python"
            | "python3"
            | "py"
            | "rustup"
            | "tauri"
            | "powershell"
            | "powershell.exe"
            | "pwsh"
            | "pwsh.exe"
            | "cmd"
            | "cmd.exe"
            | "bash"
            | "sh"
            | "ls"
            | "dir"
            | "cat"
            | "type"
            | "mkdir"
            | "rm"
            | "del"
            | "copy"
            | "move"
            | "echo"
    )
}

fn extract_backticked(prompt: &str) -> Option<String> {
    let start = prompt.find('`')?;
    let rest = &prompt[start + 1..];
    let end = rest.find('`')?;
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn extract_quoted(prompt: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let start = prompt.find(quote)?;
        let rest = &prompt[start + 1..];
        let end = rest.find(quote)?;
        let value = rest[..end].trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn extract_path_like(prompt: &str) -> Option<String> {
    prompt
        .split_whitespace()
        .map(|token| token.trim_matches(|ch| matches!(ch, '"' | '\'' | ',' | '.')))
        .find(|token| {
            token.contains('\\')
                || token.contains('/')
                || token.ends_with(".md")
                || token.ends_with(".txt")
                || token.ends_with(".json")
                || token.ends_with(".rs")
                || token.ends_with(".ts")
        })
        .map(ToOwned::to_owned)
}

fn build_terminal_command_spec(
    config: &Arc<Mutex<ConfigManager>>,
    command: &str,
) -> TerminalLaunchSpec {
    let configured_shell = config
        .lock()
        .ok()
        .and_then(|cfg| cfg.get("terminal.default_shell"))
        .filter(|value| !value.trim().is_empty());

    #[cfg(target_os = "windows")]
    let (program, args) = {
        let program = configured_shell.unwrap_or_else(|| "powershell.exe".into());
        let lower = program.to_lowercase();
        let args = if lower.ends_with("cmd.exe") || lower == "cmd.exe" {
            vec!["/C".into(), command.to_string()]
        } else if lower.ends_with("pwsh.exe")
            || lower.ends_with("powershell.exe")
            || lower == "pwsh.exe"
            || lower == "powershell.exe"
        {
            vec!["-NoLogo".into(), "-Command".into(), command.to_string()]
        } else {
            vec!["-c".into(), command.to_string()]
        };
        (program, args)
    };

    #[cfg(not(target_os = "windows"))]
    let (program, args) = {
        let program = configured_shell
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/sh".into());
        (program, vec!["-lc".into(), command.to_string()])
    };

    TerminalLaunchSpec {
        launch_id: Uuid::new_v4().to_string(),
        program,
        args,
        cwd: std::env::current_dir()
            .ok()
            .map(|path| path.display().to_string()),
        title: Some(format!("Agent: {command}")),
        env: Vec::new(),
        editor: false,
    }
}

fn should_run_local_search(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    if is_capability_question(prompt) {
        return true;
    }
    contains_any(
        &q,
        &[
            "keynova",
            "search",
            "find",
            "model",
            "note",
            "history",
            "setting",
            "workspace",
            "搜尋",
            "搜索",
            "模型",
            "筆記",
            "历史",
        ],
    )
}

fn wants_whole_computer_search(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    contains_any(
        &q,
        &[
            "whole computer",
            "entire computer",
            "all computer",
            "all drives",
            "全電腦",
            "全电脑",
            "整台",
            "所有磁碟",
            "所有硬碟",
            "全機",
        ],
    )
}

fn system_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(target_os = "windows")]
    {
        for letter in b'A'..=b'Z' {
            let root = format!("{}:\\", letter as char);
            let path = PathBuf::from(root);
            if path.is_dir() {
                roots.push(path);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        roots.push(PathBuf::from("/"));
        if let Ok(home) = std::env::var("HOME") {
            roots.push(PathBuf::from(home));
        }
    }
    roots
}

fn is_capability_question(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    contains_any(
        &q,
        &[
            "what can you do",
            "capability",
            "capabilities",
            "help me",
            "你可以",
            "可以做",
            "可以做到",
            "能做",
            "能幫",
            "能帮",
            "功能",
            "能力",
            "做什麼",
            "做什么",
            "做到甚麼",
            "做到什麼",
        ],
    )
}

fn is_time_question(prompt: &str) -> bool {
    let q = prompt.to_lowercase();
    contains_any(
        &q,
        &[
            "current time",
            "what time",
            "date",
            "time now",
            "現在時間",
            "目前時間",
            "詳細時間",
            "幾點",
            "几点",
            "日期",
            "時間",
        ],
    )
}

fn extract_web_search_query(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "web",
            "internet",
            "online",
            "news",
            "latest",
            "網路",
            "网络",
            "上網",
            "联网",
            "查網路",
            "查网络",
            "新聞",
            "新闻",
            "最新",
            "今天",
        ],
    ) {
        return None;
    }

    if let Some(query) = extract_backticked(prompt).or_else(|| extract_quoted(prompt)) {
        return Some(query);
    }

    let query = prompt
        .trim()
        .trim_start_matches("please search")
        .trim_start_matches("search")
        .trim_start_matches("look up")
        .trim_start_matches("查詢")
        .trim_start_matches("查询")
        .trim_start_matches("幫我")
        .trim_start_matches("帮我")
        .trim_start_matches("查詢")
        .trim_start_matches("查询")
        .trim_start_matches("搜尋")
        .trim_start_matches("搜索")
        .trim_start_matches("查")
        .trim()
        .to_string();
    (!query.is_empty()).then_some(query)
}

fn format_web_search_answer(query: &str, sources: &[GroundingSource]) -> String {
    let list = sources
        .iter()
        .take(5)
        .enumerate()
        .map(|(index, source)| {
            let uri = source.uri.as_deref().unwrap_or("no url");
            format!(
                "{}. {}\n   {}\n   {}",
                index + 1,
                source.title,
                uri,
                source.snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "我查詢了網路：`{query}`\n\n找到 {} 個結果：\n{}\n\n這是 read-only 網路查詢；如果你要我根據結果整理摘要或後續工作流，可以直接接著下任務。",
        sources.len(),
        list
    )
}

#[derive(Debug, Clone)]
struct GithubTrendingRepo {
    owner: String,
    name: String,
    description: String,
    url: String,
}

fn is_github_trending_prompt(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("github")
        && (lower.contains("trending")
            || lower.contains("popular")
            || lower.contains("hot")
            || lower.contains("熱門")
            || lower.contains("热门")
            || lower.contains("最熱門")
            || lower.contains("最热门"))
}

fn fetch_github_trending(limit: usize) -> Result<Vec<GithubTrendingRepo>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Keynova/0.1 read-only github trending")
        .build()
        .map_err(|e| e.to_string())?;
    let html = client
        .get("https://github.com/trending")
        .query(&[("since", "daily")])
        .send()
        .map_err(|e| format!("GitHub request failed: {e}"))?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())?;
    Ok(parse_github_trending_html(&html, limit.max(1)))
}

fn parse_github_trending_html(html: &str, limit: usize) -> Vec<GithubTrendingRepo> {
    let mut repos = Vec::new();
    let mut rest = html;
    while repos.len() < limit {
        let Some(article_pos) = rest.find("<article") else {
            break;
        };
        rest = &rest[article_pos..];
        let Some(article_end) = rest.find("</article>") else {
            break;
        };
        let article = &rest[..article_end];
        rest = &rest[article_end + "</article>".len()..];

        let Some(href_pos) = article.find("href=\"/") else {
            continue;
        };
        let href_start = href_pos + "href=\"/".len();
        let Some(href_end) = article[href_start..].find('"') else {
            continue;
        };
        let repo_path = article[href_start..href_start + href_end]
            .split_whitespace()
            .collect::<String>();
        let mut parts = repo_path.split('/');
        let (Some(owner), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        let description = extract_first_paragraph(article);
        repos.push(GithubTrendingRepo {
            owner: decode_html_entities(owner.trim()).to_string(),
            name: decode_html_entities(name.trim()).to_string(),
            description,
            url: format!("https://github.com/{repo_path}"),
        });
    }
    repos
}

fn extract_first_paragraph(html: &str) -> String {
    let Some(p_pos) = html.find("<p") else {
        return String::new();
    };
    let rest = &html[p_pos..];
    let Some(start_rel) = rest.find('>') else {
        return String::new();
    };
    let Some(end_rel) = rest[start_rel + 1..].find("</p>") else {
        return String::new();
    };
    strip_html(&rest[start_rel + 1..start_rel + 1 + end_rel])
}

fn format_github_trending_answer(repos: &[GithubTrendingRepo]) -> String {
    let list = repos
        .iter()
        .take(10)
        .enumerate()
        .map(|(index, repo)| {
            let desc = if repo.description.trim().is_empty() {
                "No description".to_string()
            } else {
                repo.description.clone()
            };
            format!(
                "{}. {}/{}\n   {}\n   {}",
                index + 1,
                repo.owner,
                repo.name,
                repo.url,
                desc
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "今天 GitHub Trending daily 前 {} 個專案：\n{}\n\n來源：GitHub Trending daily（read-only 查詢）。",
        repos.len().min(10),
        list
    )
}

fn answer_workflow_plan(prompt: &str) -> Option<String> {
    let lower = prompt.to_lowercase();
    if !contains_any(
        &lower,
        &[
            "任務",
            "工作流",
            "計畫",
            "计划",
            "完成",
            "workflow",
            "plan",
            "task",
        ],
    ) {
        return None;
    }
    Some(format!(
        "我會把這個任務拆成可執行工作流：\n\n1. 釐清目標與輸出：確認你要的最終結果、限制、是否需要網路或本機資料。\n2. 蒐集資料：優先用 read-only 本機搜尋/讀取；需要外部資訊時使用 web.search。\n3. 制定步驟：把任務拆成可驗證的小步驟，標記哪些是 read-only、哪些需要 approval。\n4. 執行安全步驟：read-only 步驟可直接完成；任何修改檔案、terminal、system/model 動作都會先請你批准。\n5. 回報結果：列出完成項、證據來源、失敗原因與下一步。\n\n目前任務：\n{}",
        prompt.trim()
    ))
}

fn sanitize_external_query(query: &str) -> Result<String, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err("web.search query is empty".into());
    }
    let lower = trimmed.to_lowercase();
    for denied in [
        "claude.md",
        "tasks.md",
        "memory.md",
        "decisions.md",
        "skill.md",
        "private_architecture",
        "secret",
        "api_key",
        "api key",
        "password",
        "token",
    ] {
        if lower.contains(denied) {
            return Err(format!(
                "web.search query contains private or secret context term '{denied}'"
            ));
        }
    }
    Ok(trimmed.to_string())
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

fn long_term_memory_opt_in(config: &Arc<Mutex<ConfigManager>>) -> bool {
    config
        .lock()
        .ok()
        .and_then(|cfg| cfg.get("agent.long_term_memory_opt_in"))
        .is_some_and(|value| value == "true")
}

fn is_allowlisted_safe_builtin(name: &str, args: &str) -> bool {
    args.trim().is_empty()
        && matches!(
            name,
            "help" | "note" | "history" | "cal" | "tr" | "ai" | "model_list"
        )
}

fn inline_result(text: String) -> BuiltinCommandResult {
    BuiltinCommandResult {
        text,
        ui_type: CommandUiType::Inline,
    }
}

fn panel_result(name: &str, initial_args: String) -> BuiltinCommandResult {
    BuiltinCommandResult {
        text: initial_args,
        ui_type: CommandUiType::Panel(name.to_string()),
    }
}

fn require_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str, String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing or empty '{key}'"))
}

// ─── ReAct Tool Dispatch ─────────────────────────────────────────────────────

/// Arc-captured state for the ReAct loop dispatch closure.
/// One instance per agent run; shared across loop steps.
/// Methods are wired into production in 5.5.C.
#[allow(dead_code)]
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
        match name {
            "keynova_search" => self.dispatch_keynova_search(args),
            "filesystem_search" => self.dispatch_filesystem_search(args),
            "filesystem_read" => self.dispatch_filesystem_read(args),
            "web_search" => self.dispatch_web_search(args),
            "git_status" => self.dispatch_git_status(args),
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

        let path = PathBuf::from(path_str);
        let resolved = if path.is_absolute() {
            path
        } else {
            let roots = self.default_search_roots();
            roots
                .iter()
                .map(|r| r.join(path_str))
                .find(|p| p.exists())
                .ok_or_else(|| {
                    format!("filesystem.read: '{path_str}' not found in workspace roots")
                })?
        };

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

    fn dispatch_keynova_search(&self, args: &Value) -> Result<Value, String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_lowercase();
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
        let mut sources: Vec<GroundingSource> = Vec::new();

        self.push_react_workspace_source(&mut sources);
        self.push_react_command_sources(&query, &mut sources)?;
        self.push_react_note_sources(&query, &mut sources)?;
        self.push_react_history_sources(&query, &mut sources)?;
        self.push_react_model_sources(&query, &mut sources);

        sources.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
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
        let (provider, searxng_url, api_key, timeout_secs) = {
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
        let sources = match provider.as_str() {
            "searxng" => search_searxng(&searxng_url, &sanitized, limit, timeout_secs)?,
            "tavily" => search_tavily(&api_key, &sanitized, limit, timeout_secs)?,
            "duckduckgo" => search_duckduckgo_html(&sanitized, limit, timeout_secs)?,
            "disabled" | "" => {
                return Err("web.search provider disabled; configure agent.web_search_provider".into())
            }
            other => return Err(format!("web.search provider '{other}' not supported")),
        };
        Ok(grounding_to_tool_sources_json(&sources))
    }

    fn push_react_workspace_source(&self, sources: &mut Vec<GroundingSource>) {
        let Ok(ws) = self.workspace_manager.lock() else {
            return;
        };
        let cur = ws.current();
        sources.push(source(
            format!("workspace:{}", cur.id),
            "workspace",
            cur.name.clone(),
            format!(
                "mode={}, panel={}, files={}, notes={}",
                cur.mode,
                cur.panel.as_deref().unwrap_or("none"),
                cur.recent_files.len(),
                cur.note_ids.len()
            ),
            0.92,
            ContextVisibility::PublicContext,
        ));
    }

    fn push_react_command_sources(
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

    fn push_react_note_sources(
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

    fn push_react_history_sources(
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

    fn push_react_model_sources(&self, query: &str, sources: &mut Vec<GroundingSource>) {
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

/// Converts `GroundingSource` slice to the `ToolSourcesResult` JSON shape.
#[allow(dead_code)]
fn grounding_to_tool_sources_json(sources: &[GroundingSource]) -> Value {
    json!({
        "sources": sources.iter().map(|s| json!({
            "title": s.title,
            "snippet": s.snippet,
            "uri": s.uri,
            "source_type": s.source_type,
        })).collect::<Vec<_>>()
    })
}

impl AgentHandler {
    /// Build a `ToolDispatch` closure wiring all ReAct-compatible tools.
    ///
    /// The closure captures Arc clones of every dep needed and is `Send + Sync`,
    /// so it can safely be passed to `spawn_react_loop`.
    /// Called from `start_run` in 5.5.C once the heuristic path is retired.
    #[allow(dead_code)]
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
}
