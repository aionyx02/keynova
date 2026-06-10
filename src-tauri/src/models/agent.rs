// Shared grounding/visibility types for the AI capability layer.
//
// Historically this module also held the legacy ReAct agent run/approval/audit
// types; those were removed in REF.8 (ADR-0029 superseded the agent). Only the
// context-visibility + grounding-source types survive, because they are shared
// by `ai_capability/*`, `grounding`, `local_context`, and `context_bundle`.

use serde::{Deserialize, Serialize};

/// Visibility class for context considered when grounding a capability answer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextVisibility {
    PublicContext,
    UserPrivate,
    PrivateArchitecture,
    Secret,
}

/// UI-safe source shown with a capability answer after visibility filtering.
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
