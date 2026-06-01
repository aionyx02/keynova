//! Heuristic action-planning helpers for `AgentHandler` (chat-first / offline
//! fallback path).
//!
//! Owns: `plan_approvals`, `detect_planned_action`, the eight `plan_*` draft
//! detectors (note/setting/terminal/file-write/system/model/panel/safe-builtin),
//! and `execute_planned_action`.
//!
//! Deprecation: per ADR-0029 and `docs/tasks/refactor-ai-capability.md` REF.3,
//! these helpers only support the legacy chat-first behaviour. They remain
//! behind `ai.legacy_agent` during the compatibility window and will be deleted
//! in REF.8 once the stateless capability layer (REF.4) ships.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::models::action::ActionRisk;
use crate::models::agent::{AgentActionKind, AgentApproval, AgentPlannedAction};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::models::terminal::TerminalLaunchSpec;

use super::formatting::{
    build_terminal_command_spec, contains_any, extract_path_like, extract_setting_value,
    extract_shell_command, inline_result, match_setting_schema, panel_result, require_str,
    suggested_note_name, title_case,
};
use super::safety::is_allowlisted_safe_builtin;
use super::AgentHandler;

impl AgentHandler {
    pub(super) fn plan_approvals(&self, prompt: &str) -> Result<Vec<AgentApproval>, String> {
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
            // Heuristic planned_action approvals don't correspond to a single
            // tool call and don't participate in the remember-for-run flow.
            tool_name: None,
            deadline_unix_ms: None,
            remember_for_run: false,
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
                    "panel": "model",
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
                    "panel": "model",
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
            ("model", "Open model panel")
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

    pub(super) fn execute_planned_action(
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
}
