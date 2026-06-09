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

/// A feature's gate metadata (DECOUP.4 / ADR-0044): the IPC namespace it owns and
/// the `features.*` flag that gates it. `flag_key = None` means always-on (e.g.
/// nvim, which has no config flag). This is the single source for the dispatch
/// namespace guard — removing a feature removes its guard automatically.
pub(crate) struct FeatureSpec {
    pub namespace: &'static str,
    pub flag_key: Option<&'static str>,
}

/// Collects a feature's contributions during assembly: handler registration +
/// its `FeatureSpec`. (Builtin commands / search providers join in DECOUP.6.)
pub(crate) struct FeatureRegistrar<'r> {
    router: &'r mut CommandRouter,
    specs: Vec<FeatureSpec>,
}

impl<'r> FeatureRegistrar<'r> {
    pub(crate) fn new(router: &'r mut CommandRouter) -> Self {
        Self {
            router,
            specs: Vec::new(),
        }
    }

    /// Register an IPC handler for this feature's namespace.
    pub(crate) fn handler(&mut self, handler: Arc<dyn CommandHandler>) -> &mut Self {
        self.router.register(handler);
        self
    }

    /// Declare this feature's namespace + gating flag.
    pub(crate) fn spec(&mut self, spec: FeatureSpec) -> &mut Self {
        self.specs.push(spec);
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
    crate::handlers::learning_material::register,
    crate::handlers::system_monitoring::register,
];

/// Run every registrar against the router and return the `(namespace, flag)`
/// pairs for features that declare a gating flag — the source for the dispatch
/// namespace guard (DECOUP.4). Features with `flag_key = None` are omitted.
pub(crate) fn register_all(
    router: &mut CommandRouter,
    ctx: &AssemblyCtx,
) -> Vec<(&'static str, &'static str)> {
    let mut reg = FeatureRegistrar::new(router);
    for register in REGISTRARS {
        register(&mut reg, ctx);
    }
    reg.specs
        .iter()
        .filter_map(|spec| spec.flag_key.map(|flag| (spec.namespace, flag)))
        .collect()
}
