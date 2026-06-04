//! DECOUP / ADR-0044 — feature self-registration scaffolding.
//!
//! Each feature exposes a `register(reg, ctx)` fn that wires its own handler(s)
//! (and, in later batches, builtins / search hooks / settings / spec). The
//! central [`REGISTRARS`] list is the single place features are enumerated, so
//! removing a feature is: delete its module + delete one entry here.
//!
//! The registrar and ctx are intentionally thin right now; they grow as
//! features migrate (see `docs/tasks/feature-decoupling.md`). Cross-cutting
//! consumers (search / ai_capability / agent) stay centrally assembled until
//! `DECOUP.6`.

use std::sync::Arc;

use crate::core::{CommandHandler, CommandRouter};

/// Shared infrastructure handed to each feature's `register` fn. Empty for now;
/// genuinely-shared handles (config / event_bus / knowledge_store / …) are added
/// here as the first feature that needs them migrates (`DECOUP.3+`). Leaf
/// features (e.g. calculator) build their own manager and need nothing from it.
pub(crate) struct AssemblyCtx;

/// Collects a feature's contributions during assembly. Currently only handler
/// registration; builtin commands, search providers, settings fragments, and a
/// `FeatureSpec` join here in later batches.
pub(crate) struct FeatureRegistrar<'r> {
    router: &'r mut CommandRouter,
}

impl<'r> FeatureRegistrar<'r> {
    pub(crate) fn new(router: &'r mut CommandRouter) -> Self {
        Self { router }
    }

    /// Register an IPC handler for this feature's namespace.
    pub(crate) fn handler(&mut self, handler: Arc<dyn CommandHandler>) -> &mut Self {
        self.router.register(handler);
        self
    }
}

/// The self-registering features (ADR-0044). Add a feature by adding its
/// `register` fn here; remove one by deleting it (plus the feature module).
pub(crate) const REGISTRARS: &[fn(&mut FeatureRegistrar, &AssemblyCtx)] =
    &[crate::handlers::calculator::register];

/// Run every registrar against the router (called from `build_command_router`).
pub(crate) fn register_all(router: &mut CommandRouter) {
    let ctx = AssemblyCtx;
    let mut reg = FeatureRegistrar::new(router);
    for register in REGISTRARS {
        register(&mut reg, &ctx);
    }
}
