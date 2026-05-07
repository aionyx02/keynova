use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::config_manager::ConfigManager;
use crate::core::{
    AgentAuditEntry, AgentMemoryEntry, AgentRuntime, BuiltinCommandRegistry, CommandHandler,
    CommandResult, KnowledgeStoreHandle,
};
use crate::managers::{
    history_manager::HistoryManager, model_manager::ModelManager, note_manager::NoteManager,
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
        let status = if approvals.is_empty() {
            AgentRunStatus::Completed
        } else {
            AgentRunStatus::WaitingApproval
        };
        let plan = build_plan(&prompt, approvals.first().and_then(|approval| approval.planned_action.as_ref()));
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
            output: Some(describe_run(&prompt_audit)),
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
        let action = run.approvals[approval_index]
            .planned_action
            .clone()
            .ok_or_else(|| "approval is missing planned action".to_string())?;

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
            let workspace_id = self.workspace_manager.lock().ok().map(|ws| ws.current().id);
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
        run.status = AgentRunStatus::Cancelled;
        run.command_result = None;
        run.output = Some(format!(
            "Approval rejected. {}",
            run.approvals[approval_index].summary
        ));
        if let Some(step) = run.steps.get_mut(1) {
            step.status = "cancelled".into();
            step.title = "Approval rejected".into();
        }
        self.log_audit(
            run_id,
            "approval_rejected",
            "cancelled",
            &run.approvals[approval_index].summary,
            None,
        );
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
                    &format!("Recent run: {} -> {}", run.prompt, run.output.unwrap_or_default()),
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
            &["note", "notes", "memo", "draft", "筆記", "笔记", "便條", "草稿"],
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
            &["setting", "settings", "config", "preference", "設定", "设置", "偏好"],
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
            &["volume", "brightness", "wifi", "mute", "音量", "亮度", "網路", "静音"],
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
        let (panel, label) = if contains_any(&lower, &["history", "clipboard", "歷史", "剪貼"]) {
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
            AgentActionKind::CreateNoteDraft => Ok(panel_result("note", action.payload.to_string())),
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
                    return Err(format!("built-in command '/{name}' is not safe for agent use"));
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
            other => return Err(format!("unknown agent tool '{other}'")),
        };
        Ok(AgentToolRunResult {
            tool_name: tool_name.to_string(),
            sources,
        })
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
        let (provider, searxng_url, timeout_secs) = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            (
                config
                    .get("agent.web_search_provider")
                    .unwrap_or_else(|| "disabled".into()),
                config.get("agent.searxng_url").unwrap_or_default(),
                config
                    .get("agent.web_search_timeout_secs")
                    .and_then(|value| value.parse::<u64>().ok())
                    .unwrap_or(8),
            )
        };

        match provider.as_str() {
            "searxng" => search_searxng(&searxng_url, &sanitized, limit, timeout_secs),
            "disabled" | "" => Err(
                "web.search provider is disabled; set agent.web_search_provider=searxng and agent.searxng_url".into(),
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

fn build_plan(prompt: &str, action: Option<&AgentPlannedAction>) -> Vec<String> {
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
    } else if should_run_local_search(prompt) {
        plan.push("Return a grounded plan using local Keynova context only".into());
    } else {
        plan.push("Return a safe next-step plan without executing local actions".into());
    }
    plan
}

fn describe_run(audit: &AgentPromptAudit) -> String {
    format!(
        "Prompt prepared with {} included source(s), {} filtered source(s), budget {} chars, truncated={}.\nFiltered sources stay local and are recorded only in audit metadata.",
        audit.included_sources.len(),
        audit.filtered_sources.len(),
        audit.budget_chars,
        audit.truncated
    )
}

fn describe_execution(action: &AgentPlannedAction, result: &BuiltinCommandResult) -> String {
    match &result.ui_type {
        CommandUiType::Inline => format!("Executed '{}'. {}", action.label, result.text),
        CommandUiType::Panel(panel) => format!("Executed '{}'. Opened panel '{}'.", action.label, panel),
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
            "api_key",
            "api key",
            "password",
            "token",
            "secret",
            "sk-",
            "bearer ",
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
            "run ",
            "execute ",
            "start ",
            "terminal",
            "cargo ",
            "npm ",
            "git ",
            "pnpm ",
            "python ",
            "執行",
            "运行",
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
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
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
        && matches!(name, "help" | "note" | "history" | "cal" | "tr" | "ai" | "model_list")
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
}
