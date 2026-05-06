#![allow(dead_code)]

use serde::{Deserialize, Serialize};

use crate::models::action::{ActionRef, ActionRisk};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryRef {
    pub id: String,
    pub scope: String,
    pub visibility: ContextVisibility,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApproval {
    pub id: String,
    pub action_ref: Option<ActionRef>,
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
    pub output: Option<String>,
    pub error: Option<String>,
}
