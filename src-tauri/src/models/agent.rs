#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::action::{ActionRef, ActionRisk};
use crate::models::builtin_command::BuiltinCommandResult;
use crate::models::context_bundle::ContextBundle;

/// Visibility class for context considered by the local agent runtime.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextVisibility {
    PublicContext,
    UserPrivate,
    PrivateArchitecture,
    Secret,
}

/// UI-safe source shown with agent answers after visibility filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundingSource {
    pub source_id: String,
    pub source_type: String,
    pub title: String,
    pub snippet: String,
    pub uri: Option<String>,
    pub score: f32,
    pub visibility: ContextVisibility,
    pub redacted_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Planning,
    WaitingApproval,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMemoryScope {
    Session,
    Workspace,
    LongTerm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryRef {
    pub id: String,
    pub scope: AgentMemoryScope,
    pub visibility: ContextVisibility,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFilteredSource {
    pub source_id: String,
    pub source_type: String,
    pub title: String,
    pub visibility: ContextVisibility,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPromptAudit {
    pub budget_chars: usize,
    pub prompt_chars: usize,
    pub truncated: bool,
    pub included_sources: Vec<GroundingSource>,
    pub filtered_sources: Vec<AgentFilteredSource>,
    pub redacted_secret_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCall {
    pub id: String,
    pub tool_name: String,
    pub risk: ActionRisk,
    pub status: String,
    pub duration_ms: Option<u128>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionKind {
    OpenPanel,
    CreateNoteDraft,
    UpdateSettingDraft,
    RunBuiltinCommand,
    TerminalCommand,
    FileWrite,
    SystemControl,
    ModelLifecycle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlannedAction {
    pub id: String,
    pub kind: AgentActionKind,
    pub risk: ActionRisk,
    pub label: String,
    pub summary: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApproval {
    pub id: String,
    pub action_ref: Option<ActionRef>,
    pub planned_action: Option<AgentPlannedAction>,
    pub risk: ActionRisk,
    pub summary: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub id: String,
    pub title: String,
    pub status: String,
    pub tool_calls: Vec<AgentToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub id: String,
    pub prompt: String,
    pub status: AgentRunStatus,
    pub plan: Vec<String>,
    pub steps: Vec<AgentStep>,
    pub approvals: Vec<AgentApproval>,
    pub memory_refs: Vec<AgentMemoryRef>,
    pub sources: Vec<GroundingSource>,
    pub prompt_audit: Option<AgentPromptAudit>,
    #[serde(default)]
    pub context_bundle: Option<ContextBundle>,
    pub command_result: Option<BuiltinCommandResult>,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Structured error type for agent handler internals.
/// Converted to `String` at the `CommandResult` boundary so the external API is unchanged.
#[derive(Debug)]
pub enum AgentError {
    Config(String),
    LockPoisoned,
    ProviderUnavailable(String),
    ToolDenied { tool: String, reason: String },
    SafetyViolation(String),
    IoError(String),
    Unknown(String),
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::Config(msg) => write!(f, "config error: {msg}"),
            AgentError::LockPoisoned => write!(f, "internal lock was poisoned"),
            AgentError::ProviderUnavailable(msg) => write!(f, "AI provider unavailable: {msg}"),
            AgentError::ToolDenied { tool, reason } => write!(f, "tool '{tool}' denied: {reason}"),
            AgentError::SafetyViolation(msg) => write!(f, "safety violation: {msg}"),
            AgentError::IoError(msg) => write!(f, "I/O error: {msg}"),
            AgentError::Unknown(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<AgentError> for String {
    fn from(e: AgentError) -> Self {
        e.to_string()
    }
}
