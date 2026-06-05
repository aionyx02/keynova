use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::time::Instant;

use serde_json::json;

use crate::app::feature_registry;
use crate::core::agent_runtime::AgentArchiveSink;
use crate::core::config_manager::ConfigManager;
use crate::core::knowledge_store::AgentArchiveEntry;
use crate::core::local_context::LocalContextSearcher;
use crate::core::startup_preflight::StartupPreflight;
use crate::core::{
    ActionArena, AgentRuntime, AppEvent, BuiltinCommandRegistry, CommandRouter, EventBus,
    KnowledgeStoreHandle,
};
use crate::handlers::{
    agent::{AgentHandler, AgentHandlerDeps},
    ai::AiHandler,
    ai_capability::{AiCapabilityHandler, AiCapabilityHandlerDeps},
    automation::AutomationHandler,
    builtin_cmd::{
        BuiltinCmdHandler, CalCommand, DownCommand, HelpCommand, HistoryCommand, ModelCommand,
        NoteCommand, OnboardCommand, RebuildSearchIndexCommand, ReloadCommand, SettingCommand,
        SysCtlCommand, SysMonitorCommand, TrCommand,
    },
    dev_utils_cmd::{
        B64decCmd, B64encCmd, ColorCmd, CronCmd, HashCmd, JsonCmd, JsonmCmd, JwtCmd, KillPortCmd,
        NanoidCmd, PwCmd, RegexCmd, UrldecCmd, UrlencCmd, UuidCmd,
    },
    feature::FeatureHandler,
    file::FileHandler,
    hotkey::HotkeyHandler,
    launcher::LauncherHandler,
    model::ModelHandler,
    mouse::MouseHandler,
    plugin::PluginHandler,
    search::{SearchHandler, SearchHandlerDeps},
    setting::SettingHandler,
    terminal::TerminalHandler,
    workflow_memory::{WorkflowMemoryHandler, WorkflowMemoryHandlerDeps},
    workspace::WorkspaceHandler,
};
use crate::managers::{
    ai_manager::AiManager, app_manager::AppManager, history_manager::HistoryManager,
    hotkey_manager::HotkeyManager, model_manager::ModelManager, mouse_manager::MouseManager,
    note_manager::NoteManager, search_manager::SearchManager, search_service::SearchService,
    terminal_manager::TerminalManager, workspace_manager::WorkspaceManager,
};
use crate::models::agent::AgentRun;

pub(crate) struct AppState {
    pub(crate) command_router: CommandRouter,
    /// DECOUP.4: `(namespace, features.* flag)` pairs derived from feature specs;
    /// the dispatch namespace guard reads this instead of a hand-kept const.
    pub(crate) feature_namespace_guards: Vec<(&'static str, &'static str)>,
    pub(crate) action_arena: Arc<ActionArena>,
    pub(crate) event_bus: EventBus,
    pub(crate) knowledge_store: KnowledgeStoreHandle,
    pub(crate) mouse_active: Arc<AtomicBool>,
    pub(crate) launcher_focus_guard: Arc<Mutex<Option<Instant>>>,
    pub(crate) _config_manager: Arc<Mutex<ConfigManager>>,
    pub(crate) _app_manager: Arc<Mutex<AppManager>>,
    pub(crate) _config_watcher: Arc<Mutex<Option<notify::RecommendedWatcher>>>,
    pub(crate) _history_manager: Arc<Mutex<HistoryManager>>,
    pub(crate) _search_manager: Arc<Mutex<SearchManager>>,
    pub(crate) _startup_preflight: Arc<StartupPreflight>,
    pub(crate) _terminal_manager: Arc<Mutex<TerminalManager>>,
    pub(crate) _workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

/// All manager instances created at startup.
/// Fields needed only by handlers (not stored in AppState) are consumed during router build.
struct ManagerBundle {
    config_manager: Arc<Mutex<ConfigManager>>,
    app_manager: Arc<Mutex<AppManager>>,
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
    mouse_manager: Arc<Mutex<MouseManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    model_manager: Arc<ModelManager>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    terminal_manager: Arc<Mutex<TerminalManager>>,
    search_manager: Arc<Mutex<SearchManager>>,
    startup_preflight: Arc<StartupPreflight>,
    ai_manager: Arc<AiManager>,
    agent_runtime: Arc<AgentRuntime>,
}

/// Adapter that forwards FIFO-evicted agent runs to the `agent_archive` SQLite table.
/// Errors are swallowed by KnowledgeStore's fire-and-forget contract; we intentionally
/// trade durability for never blocking the runtime under DB backpressure.
struct KnowledgeStoreArchiveSink {
    handle: KnowledgeStoreHandle,
}

impl AgentArchiveSink for KnowledgeStoreArchiveSink {
    fn archive(&self, run: &AgentRun) {
        let status = serde_json::to_value(&run.status)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".into());
        let payload_json = serde_json::to_string(run).unwrap_or_else(|_| "{}".into());
        self.handle.try_archive_agent_run(AgentArchiveEntry {
            run_id: run.id.clone(),
            prompt: run.prompt.clone(),
            status,
            payload_json,
        });
    }
}

fn create_managers(event_bus: &EventBus, knowledge_store: &KnowledgeStoreHandle) -> ManagerBundle {
    let app_manager = Arc::new(Mutex::new(AppManager::new()));
    let hotkey_manager = Arc::new(Mutex::new(HotkeyManager::new()));
    let mouse_manager = Arc::new(Mutex::new(MouseManager::new()));
    let workspace_manager = Arc::new(Mutex::new(WorkspaceManager::new()));
    // PROJECT_ROOT.wire — detect the launch directory's project root once at
    // startup so workspace-aware search + project command discovery (1.A/1.C/1.D)
    // activate. `if_unset` preserves any persisted/user value; no project root
    // found ⇒ no-op (global search, as before).
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(root) = crate::core::project_commands::detect_project_root(&cwd) {
            if let Ok(mut ws) = workspace_manager.lock() {
                ws.set_project_root_if_unset(root.display().to_string());
            }
        }
    }
    let model_manager = Arc::new(ModelManager::new());

    let config_manager = Arc::new(Mutex::new(ConfigManager::new()));

    let note_storage_dir = config_manager
        .lock()
        .ok()
        .and_then(|c| c.get("notes.storage_dir"));
    let note_manager = Arc::new(Mutex::new(NoteManager::new(note_storage_dir)));

    let max_items = config_manager
        .lock()
        .ok()
        .and_then(|c| c.get("history.max_items"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(200);
    let history_manager = Arc::new(Mutex::new(HistoryManager::new(max_items)));

    let eb_for_terminal = event_bus.clone();
    let terminal_manager = Arc::new(Mutex::new(TerminalManager::new(Arc::new(
        move |id, output| {
            let _ = eb_for_terminal.publish(AppEvent::new(
                "terminal.output",
                json!({ "id": id, "output": output }),
            ));
        },
    ))));

    let configured_search_backend = config_manager
        .lock()
        .ok()
        .and_then(|c| c.get("search.backend"));
    let configured_search_index_dir = config_manager
        .lock()
        .ok()
        .and_then(|c| c.get("search.index_dir"));
    let search_manager = Arc::new(Mutex::new(SearchManager::new_with_config(
        Arc::clone(&app_manager),
        configured_search_backend.as_deref(),
        configured_search_index_dir.as_deref(),
    )));
    let startup_preflight = Arc::new(StartupPreflight::new(
        Arc::clone(&config_manager),
        Arc::clone(&model_manager),
        event_bus.clone(),
    ));

    let eb_for_ai = event_bus.clone();
    let ai_manager = Arc::new(AiManager::new(Arc::new(move |event| {
        let _ = eb_for_ai.publish(event);
    })));

    let eb_for_agent = event_bus.clone();
    let run_cap = config_manager
        .lock()
        .ok()
        .and_then(|c| c.get("agent.run_history_cap"))
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(20);
    let archive_sink: Arc<dyn AgentArchiveSink> = Arc::new(KnowledgeStoreArchiveSink {
        handle: knowledge_store.clone(),
    });
    let agent_runtime = Arc::new(AgentRuntime::with_archive(
        Arc::new(move |event| {
            let _ = eb_for_agent.publish(event);
        }),
        run_cap,
        archive_sink,
    ));

    ManagerBundle {
        config_manager,
        app_manager,
        hotkey_manager,
        mouse_manager,
        workspace_manager,
        model_manager,
        note_manager,
        history_manager,
        terminal_manager,
        search_manager,
        startup_preflight,
        ai_manager,
        agent_runtime,
    }
}

fn build_builtin_registry(
    config_manager: &Arc<Mutex<ConfigManager>>,
    note_manager: &Arc<Mutex<NoteManager>>,
) -> Arc<Mutex<BuiltinCommandRegistry>> {
    let registry = Arc::new(Mutex::new(BuiltinCommandRegistry::new()));
    let mut reg = registry.lock().expect("registry init");
    reg.register(Box::new(HelpCommand));
    reg.register(Box::new(SettingCommand));
    reg.register(Box::new(ReloadCommand));
    reg.register(Box::new(OnboardCommand));
    reg.register(Box::new(DownCommand));
    reg.register(Box::new(TrCommand));
    // REF.6.B follow-up — `/ai` builtin removed: legacy chat panel does not
    // belong on the keyboard-first hot path (docx §4.6). Inline AI is now
    // invoked via prefix keyword (`explain <q>` / `summarize <text>`); the
    // `ai.model` config still feeds the capability layer via /model.
    // If chat ever returns, re-register here and reinstate
    // `PanelRegistry["ai"]`.
    // REF.8 — the `/ai_legacy_chat` builtin + AiPanel chat UI were removed. The
    // backend agent_runtime + handlers/agent are retained (dormant) behind the
    // reserved `ai.legacy_agent` config flag, but there is no command/panel entry
    // point in this build, so nothing is registered here.
    reg.register(Box::new(ModelCommand));
    reg.register(Box::new(NoteCommand::new(
        Arc::clone(note_manager),
        Arc::clone(config_manager),
    )));
    reg.register(Box::new(CalCommand));
    reg.register(Box::new(HistoryCommand));
    reg.register(Box::new(SysCtlCommand));
    reg.register(Box::new(SysMonitorCommand));
    reg.register(Box::new(RebuildSearchIndexCommand));
    // UTIL.2 dev utilities (Slice 1 — A through I)
    reg.register(Box::new(UuidCmd));
    reg.register(Box::new(NanoidCmd));
    reg.register(Box::new(PwCmd));
    reg.register(Box::new(HashCmd));
    reg.register(Box::new(B64encCmd));
    reg.register(Box::new(B64decCmd));
    reg.register(Box::new(UrlencCmd));
    reg.register(Box::new(UrldecCmd));
    reg.register(Box::new(JsonCmd));
    reg.register(Box::new(JsonmCmd));
    reg.register(Box::new(RegexCmd));
    reg.register(Box::new(JwtCmd));
    reg.register(Box::new(ColorCmd));
    reg.register(Box::new(CronCmd));
    reg.register(Box::new(KillPortCmd));
    drop(reg);
    registry
}

fn build_command_router(
    bundle: &ManagerBundle,
    event_bus: &EventBus,
    action_arena: &Arc<ActionArena>,
    knowledge_store: &KnowledgeStoreHandle,
    mouse_active: &Arc<AtomicBool>,
) -> (CommandRouter, Vec<(&'static str, &'static str)>) {
    let builtin_registry = build_builtin_registry(&bundle.config_manager, &bundle.note_manager);

    let agent_tantivy_dir = bundle
        .search_manager
        .lock()
        .ok()
        .map(|m| m.tantivy_index_dir().to_path_buf())
        .unwrap_or_else(|| crate::managers::tantivy_index::resolve_index_dir(None));

    let mut router = CommandRouter::new();
    router.register(Arc::new(LauncherHandler::new(Arc::clone(
        &bundle.app_manager,
    ))));
    router.register(Arc::new(HotkeyHandler::new(Arc::clone(
        &bundle.hotkey_manager,
    ))));
    router.register(Arc::new(TerminalHandler::new(
        Arc::clone(&bundle.terminal_manager),
        Arc::clone(&bundle.workspace_manager),
    )));
    router.register(Arc::new(FeatureHandler::new()));
    router.register(Arc::new(MouseHandler::new(
        Arc::clone(&bundle.mouse_manager),
        Arc::clone(mouse_active),
    )));
    router.register(Arc::new(SearchHandler::new(SearchHandlerDeps {
        manager: Arc::clone(&bundle.search_manager),
        action_arena: Arc::clone(action_arena),
        builtin_registry: Arc::clone(&builtin_registry),
        note_manager: Arc::clone(&bundle.note_manager),
        history_manager: Arc::clone(&bundle.history_manager),
        workspace_manager: Arc::clone(&bundle.workspace_manager),
        model_manager: Arc::clone(&bundle.model_manager),
        config: Arc::clone(&bundle.config_manager),
        knowledge_store: knowledge_store.clone(),
        event_bus: event_bus.clone(),
        search_service: SearchService::new(),
    })));
    let eb_for_model = event_bus.clone();
    router.register(Arc::new(ModelHandler::new(
        Arc::clone(&bundle.model_manager),
        Arc::clone(&bundle.config_manager),
        Arc::clone(&bundle.startup_preflight),
        Arc::new(move |event| {
            let _ = eb_for_model.publish(event);
        }),
    )));
    router.register(Arc::new(BuiltinCmdHandler::new(
        Arc::clone(&builtin_registry),
        Arc::clone(&bundle.config_manager),
        Arc::clone(&bundle.search_manager),
    )));
    router.register(Arc::new(SettingHandler::new(Arc::clone(
        &bundle.config_manager,
    ))));
    router.register(Arc::new(WorkspaceHandler::new(Arc::clone(
        &bundle.workspace_manager,
    ))));
    router.register(Arc::new(FileHandler::new()));
    router.register(Arc::new(AiHandler::new(
        Arc::clone(&bundle.ai_manager),
        Arc::clone(&bundle.config_manager),
        Arc::clone(&bundle.workspace_manager),
        Arc::clone(&bundle.model_manager),
    )));
    router.register(Arc::new(AgentHandler::new(AgentHandlerDeps {
        runtime: Arc::clone(&bundle.agent_runtime),
        config: Arc::clone(&bundle.config_manager),
        note_manager: Arc::clone(&bundle.note_manager),
        history_manager: Arc::clone(&bundle.history_manager),
        workspace_manager: Arc::clone(&bundle.workspace_manager),
        builtin_registry: Arc::clone(&builtin_registry),
        model_manager: Arc::clone(&bundle.model_manager),
        knowledge_store: knowledge_store.clone(),
        tantivy_index_dir: agent_tantivy_dir,
    })));
    router.register(Arc::new(AiCapabilityHandler::new(
        AiCapabilityHandlerDeps {
            ai: Arc::clone(&bundle.ai_manager),
            config: Arc::clone(&bundle.config_manager),
            local_context: LocalContextSearcher {
                workspace_manager: Arc::clone(&bundle.workspace_manager),
                note_manager: Arc::clone(&bundle.note_manager),
                history_manager: Arc::clone(&bundle.history_manager),
                builtin_registry: Arc::clone(&builtin_registry),
                model_manager: Arc::clone(&bundle.model_manager),
            },
            knowledge_store: knowledge_store.clone(),
            event_bus: Arc::new(event_bus.clone()),
        },
    )));
    router.register(Arc::new(AutomationHandler));
    router.register(Arc::new(PluginHandler));
    router.register(Arc::new(WorkflowMemoryHandler::new(
        WorkflowMemoryHandlerDeps {
            store: knowledge_store.clone(),
            workspace_manager: Arc::clone(&bundle.workspace_manager),
        },
    )));

    // DECOUP (ADR-0044): self-registering feature modules. Migrated features
    // wire themselves here instead of being hand-listed above.
    let assembly_ctx = feature_registry::AssemblyCtx {
        config: Arc::clone(&bundle.config_manager),
        event_bus: event_bus.clone(),
        workspace_manager: Arc::clone(&bundle.workspace_manager),
        note_manager: Arc::clone(&bundle.note_manager),
        history_manager: Arc::clone(&bundle.history_manager),
    };
    let feature_namespace_guards = feature_registry::register_all(&mut router, &assembly_ctx);

    (router, feature_namespace_guards)
}

impl AppState {
    pub(crate) fn new() -> Self {
        let event_bus = EventBus::default();
        let action_arena = Arc::new(ActionArena::default());
        let knowledge_store = KnowledgeStoreHandle::new_default();
        let mouse_active = Arc::new(AtomicBool::new(false));

        let bundle = create_managers(&event_bus, &knowledge_store);
        let (command_router, feature_namespace_guards) = build_command_router(
            &bundle,
            &event_bus,
            &action_arena,
            &knowledge_store,
            &mouse_active,
        );

        Self {
            command_router,
            feature_namespace_guards,
            action_arena,
            event_bus,
            knowledge_store,
            mouse_active,
            launcher_focus_guard: Arc::new(Mutex::new(None)),
            _config_manager: bundle.config_manager,
            _app_manager: bundle.app_manager,
            _config_watcher: Arc::new(Mutex::new(None)),
            _history_manager: bundle.history_manager,
            _search_manager: bundle.search_manager,
            _startup_preflight: bundle.startup_preflight,
            _terminal_manager: bundle.terminal_manager,
            _workspace_manager: bundle.workspace_manager,
        }
    }
}
