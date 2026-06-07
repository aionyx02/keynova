//! Capability: workspace command profile (PROFILE.1 / ADR-0054).
//!
//! "Your signature commands for *this project*" — the current project's top
//! commands ranked by frequency × success rate. Distinct from `suggest_next`
//! (which predicts the next step from transitions); this is the project toolkit.
//! Copy/replay-only, reusing the shared ranking helpers in [`super::suggest_next`].

use serde::Deserialize;
use serde_json::Value;

use crate::core::ai_capability::contract::{
    CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest, CapabilityResponse,
};
use crate::core::ai_capability::registry::CapabilityId;
use crate::core::workflow_memory;
use crate::models::unified_result::RiskTag;

use super::suggest_next::rank_profile;

/// History window scanned to build the profile; wider than the display limit so a
/// project's commands are well represented before scoping + ranking.
const PROFILE_WINDOW: usize = 400;

#[derive(Debug, Clone, Default, Deserialize)]
struct ProfileCtx {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct Payload {
    #[serde(default)]
    ctx: ProfileCtx,
}

pub fn call(
    req: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    let payload: Payload = serde_json::from_value(req.payload)
        .map_err(|e| CapabilityError::InvalidPayload(e.to_string()))?;
    let limit = workflow_memory::clamp_limit(payload.ctx.limit);

    // Pull a wide global window; `rank_profile` scopes it to the active project
    // (most-recent row's project_root, falling back to its slot, else global).
    let rows = match deps.knowledge_store.as_ref() {
        Some(store) => workflow_memory::suggest(store, None, PROFILE_WINDOW)
            .map_err(CapabilityError::ProviderError)?,
        None => Vec::new(),
    };

    let mut suggestions = rank_profile(rows, deps.local_context.as_ref(), now_seconds());
    suggestions.truncate(limit);

    Ok(CapabilityResponse {
        id: CapabilityId::WorkspaceProfile,
        output: CapabilityOutput::Structured {
            value: serde_json::to_value(suggestions).unwrap_or_else(|_| Value::Array(Vec::new())),
        },
        risk_tag: RiskTag::none(),
        sources: Vec::new(),
    })
}

fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}
