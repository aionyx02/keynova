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

use std::sync::{Arc, Mutex};

use crate::core::config_manager::ConfigManager;
use crate::core::{CommandHandler, CommandRouter, EventBus};
use crate::managers::history_manager::HistoryManager;
use crate::managers::note_manager::NoteManager;
use crate::managers::workspace_manager::WorkspaceManager;

/// Shared infrastructure handed to each feature's `register` fn. Holds the
/// genuinely-shared handles (per ADR-0044 §2 boundary) that more than one
/// feature/consumer needs; leaf features (calculator, translation, system) build
/// their own manager and only borrow what they need (e.g. `event_bus`/`config`).
pub(crate) struct AssemblyCtx {
    pub config: Arc<Mutex<ConfigManager>>,
    pub event_bus: EventBus,
    pub workspace_manager: Arc<Mutex<WorkspaceManager>>,
    pub note_manager: Arc<Mutex<NoteManager>>,
    pub history_manager: Arc<Mutex<HistoryManager>>,
}

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
pub(crate) const REGISTRARS: &[fn(&mut FeatureRegistrar, &AssemblyCtx)] = &[
    crate::handlers::calculator::register,
    crate::handlers::translation::register,
    crate::handlers::system_control::register,
    crate::handlers::note::register,
    crate::handlers::history::register,
    crate::handlers::nvim::register,
    crate::handlers::learning_material::register,
    crate::handlers::system_monitoring::register,
];

/// Run every registrar against the router (called from `build_command_router`).
pub(crate) fn register_all(router: &mut CommandRouter, ctx: &AssemblyCtx) {
    let mut reg = FeatureRegistrar::new(router);
    for register in REGISTRARS {
        register(&mut reg, ctx);
    }
}
