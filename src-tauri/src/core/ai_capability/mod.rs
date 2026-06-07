//! REF.4 — stateless AI capability layer.
//!
//! Capability execution is strictly single-shot (no chaining to other
//! capabilities) and stateless (no session memory) per ADR-0029 §4. Each
//! capability returns an ADR-0030 `RiskTag` per call; audit emission is
//! governed by `CapabilityMeta.audit` set at registration time, not in the
//! per-call return.

mod capabilities;
pub(crate) mod command;
pub mod contract;
pub(crate) mod memory;
pub(crate) mod parse;
pub mod prompt;
pub mod registry;

pub use contract::{
    AiManagerChatProvider, CapabilityDeps, CapabilityError, CapabilityOutput, CapabilityRequest,
    CapabilityResponse, CapabilitySource, ChatProvider,
};
pub use registry::{all, meta, CapabilityId, CapabilityMeta};

/// Single public entry for the capability layer. Dispatch is a compile-time
/// `match` on `CapabilityId` — no `dyn Capability` indirection.
pub fn call_capability(
    request: CapabilityRequest,
    deps: &CapabilityDeps,
) -> Result<CapabilityResponse, CapabilityError> {
    match request.id {
        CapabilityId::Explain => capabilities::explain::call(request, deps),
        CapabilityId::Summarize => capabilities::summarize::call(request, deps),
        CapabilityId::FixError => capabilities::fix_error::call(request, deps),
        CapabilityId::GenCommand => capabilities::gen_command::call(request, deps),
        CapabilityId::SuggestNext => capabilities::suggest_next::call(request, deps),
        CapabilityId::Remember => capabilities::remember::call(request, deps),
        CapabilityId::Recall => capabilities::recall::call(request, deps),
    }
}

#[cfg(all(test, feature = "live-ai"))]
mod live_tests;

#[cfg(test)]
pub(crate) mod test_fixtures;
