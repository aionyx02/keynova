use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::time::Instant;

use serde_json::json;

use crate::core::config_manager::ConfigManager;
use crate::core::{
    ActionArena, AgentRuntime, AppEvent, BuiltinCommandRegistry, CommandRouter, EventBus,
    KnowledgeStoreHandle,
};
use crate::handlers::{
    agent::{AgentHandler, AgentHandlerDeps},
    ai::AiHandler,
    automation::AutomationHandler,
    builtin_cmd::{
        AiCommand, BuiltinCmdHandler, CalCommand, DownCommand, HelpCommand, HistoryCommand,
        ModelDownloadCommand, ModelListCommand, ModelRemoveCommand, NoteCommand,
        RebuildSearchIndexCommand, ReloadCommand, SettingCommand, SysCtlCommand,
        SysMonitorCommand, TrCommand,
    },
    calculator::CalculatorHandler,
    feature::FeatureHandler,
    history::HistoryHandler,
    hotkey::HotkeyHandler,
    launcher::LauncherHandler,
    learning_material::LearningMaterialHandler,
    model::ModelHandler,
    mouse::MouseHandler,
    note::NoteHandler,
    plugin::PluginHandler,
    search::{SearchHandler, SearchHandlerDeps},
    setting::SettingHandler,
    system_control::SystemControlHandler,
    nvim::NvimHandler,
    system_monitoring::SystemMonitoringHandler,
    terminal::TerminalHandler,
    translation::TranslationHandler,
    workspace::WorkspaceHandler,
};
use crate::managers::{
    ai_manager::AiManager, app_manager::AppManager, calculator_manager::CalculatorManager,
    history_manager::HistoryManager, hotkey_manager::HotkeyManager, model_manager::ModelManager,
    mouse_manager::MouseManager, note_manager::NoteManager, search_manager::SearchManager,
    search_service::SearchService, system_manager::SystemManager,
    terminal_manager::TerminalManager, translation_manager::TranslationManager,
    workspace_manager::WorkspaceManager,
};

pub(crate) struct AppState {
    pub(crate) command_router: CommandRouter,
    pub(crate) action_arena: Arc<ActionArena>,
    pub(crate) event_bus: EventBus,
    pub(crate) knowledge_store: KnowledgeStoreHandle,
    pub(crate) mouse_active: Arc<AtomicBool>,
    pub(crate) launcher_focus_guard: Arc<Mutex<Option<Instant>>>,
    pub(crate) _config_manager: Arc<Mutex<ConfigManager>>,
    pub(crate) _config_watcher: Arc<Mutex<Option<notify::RecommendedWatcher>>>,
    pub(crate) _history_manager: Arc<Mutex<HistoryManager>>,
    pub(crate) _search_manager: Arc<Mutex<SearchManager>>,
    pub(crate) _workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

/// All manager instances created at startup.
/// Fields needed only by handlers (not stored in AppState) are consumed during router build.
struct ManagerBundle {
    config_manager: Arc<Mutex<ConfigManager>>,
    app_manager: Arc<Mutex<AppManager>>,
    hotkey_manager: Arc<Mutex<HotkeyManager>>,
    mouse_manager: Arc<Mutex<MouseManager>>,
    system_manager: Arc<Mutex<SystemManager>>,
    calculator_manager: Arc<Mutex<CalculatorManager>>,
    workspace_manager: Arc<Mutex<WorkspaceManager>>,
    model_manager: Arc<ModelManager>,
    note_manager: Arc<Mutex<NoteManager>>,
    history_manager: Arc<Mutex<HistoryManager>>,
    terminal_manager: Arc<Mutex<TerminalManager>>,
    search_manager: Arc<Mutex<SearchManager>>,
    ai_manager: Arc<AiManager>,
    agent_runtime: Arc<AgentRuntime>,
    translation_manager: Arc<TranslationManager>,
}

fn create_managers(event_bus: &EventBus) -> ManagerBundle {
    let app_manager = Arc::new(Mutex::new(AppManager::new()));
    let hotkey_manager = Arc::new(Mutex::new(HotkeyManager::new()));
    let mouse_manager = Arc::new(Mutex::new(MouseManager::new()));
    let system_manager = Arc::new(Mutex::new(SystemManager::new()));
    let calculator_manager = Arc::new(Mutex::new(CalculatorManager::new()));
    let workspace_manager = Arc::new(Mutex::new(WorkspaceManager::new()));
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

    let eb_for_ai = event_bus.clone();
    let ai_manager = Arc::new(AiManager::new(Arc::new(move |event| {
        let _ = eb_for_ai.publish(event);
    })));

    let eb_for_agent = event_bus.clone();
    let agent_runtime = Arc::new(AgentRuntime::new(Arc::new(move |event| {
        let _ = eb_for_agent.publish(event);
    })));

    let eb_for_tr = event_bus.clone();
    let translation_manager = Arc::new(TranslationManager::new(Arc::new(move |event| {
        let _ = eb_for_tr.publish(event);
    })));

    ManagerBundle {
        config_manager,
        app_manager,
        hotkey_manager,
        mouse_manager,
        system_manager,
        calculator_manager,
        workspace_manager,
        model_manager,
        note_manager,
        history_manager,
        terminal_manager,
        search_manager,
        ai_manager,
        agent_runtime,
        translation_manager,
    }
}

fn build_builtin_registry(
    model_manager: &Arc<ModelManager>,
    config_manager: &Arc<Mutex<ConfigManager>>,
    note_manager: &Arc<Mutex<NoteManager>>,
) -> Arc<Mutex<BuiltinCommandRegistry>> {
    let registry = Arc::new(Mutex::new(BuiltinCommandRegistry::new()));
    let mut reg = registry.lock().expect("registry init");
    reg.register(Box::new(HelpCommand));
    reg.register(Box::new(SettingCommand));
    reg.register(Box::new(ReloadCommand));
    reg.register(Box::new(DownCommand));
    reg.register(Box::new(TrCommand));
    reg.register(Box::new(AiCommand));
    reg.register(Box::new(ModelDownloadCommand));
    reg.register(Box::new(ModelListCommand));
    reg.register(Box::new(ModelRemoveCommand::new(
        Arc::clone(model_manager),
        Arc::clone(config_manager),
    )));
    reg.register(Box::new(NoteCommand::new(
        Arc::clone(note_manager),
        Arc::clone(config_manager),
    )));
    reg.register(Box::new(CalCommand));
    reg.register(Box::new(HistoryCommand));
    reg.register(Box::new(SysCtlCommand));
    reg.register(Box::new(SysMonitorCommand));
    reg.register(Box::new(RebuildSearchIndexCommand));
    drop(reg);
    registry
}

fn build_command_router(
    bundle: &ManagerBundle,
    event_bus: &EventBus,
    action_arena: &Arc<ActionArena>,
    knowledge_store: &KnowledgeStoreHandle,
) -> CommandRouter {
    let builtin_registry = build_builtin_registry(
        &bundle.model_manager,
        &bundle.config_manager,
        &bundle.note_manager,
    );

    let agent_tantivy_dir = bundle
        .search_manager
        .lock()
        .ok()
        .map(|m| m.tantivy_index_dir().to_path_buf())
        .unwrap_or_else(|| crate::managers::tantivy_index::resolve_index_dir(None));

    let mut router = CommandRouter::new();
    router.register(Arc::new(SystemControlHandler::new(Arc::clone(
        &bundle.system_manager,
    ))));
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
    router.register(Arc::new(FeatureHandler::new(Arc::clone(
        &bundle.terminal_manager,
    ))));
    router.register(Arc::new(MouseHandler::new(Arc::clone(
        &bundle.mouse_manager,
    ))));
    router.register(Arc::new(SearchHandler::new(SearchHandlerDeps {
        manager: Arc::clone(&bundle.search_manager),
        action_arena: Arc::clone(action_arena),
        builtin_registry: Arc::clone(&builtin_registry),
        note_manager: Arc::clone(&bundle.note_manager),
        history_manager: Arc::clone(&bundle.history_manager),
        workspace_manager: Arc::clone(&bundle.workspace_manager),
        model_manager: Arc::clone(&bundle.model_manager),
        event_bus: event_bus.clone(),
        search_service: SearchService::new(),
    })));
    let eb_for_model = event_bus.clone();
    router.register(Arc::new(ModelHandler::new(
        Arc::clone(&bundle.model_manager),
        Arc::clone(&bundle.config_manager),
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
    router.register(Arc::new(CalculatorHandler::new(Arc::clone(
        &bundle.calculator_manager,
    ))));
    router.register(Arc::new(WorkspaceHandler::new(Arc::clone(
        &bundle.workspace_manager,
    ))));
    router.register(Arc::new(NoteHandler::new(
        Arc::clone(&bundle.note_manager),
        Arc::clone(&bundle.workspace_manager),
    )));
    router.register(Arc::new(HistoryHandler::new(Arc::clone(
        &bundle.history_manager,
    ))));
    router.register(Arc::new(AiHandler::new(
        Arc::clone(&bundle.ai_manager),
        Arc::clone(&bundle.config_manager),
        Arc::clone(&bundle.workspace_manager),
        Arc::clone(&bundle.model_manager),
    )));
    router.register(Arc::new(TranslationHandler::new(
        Arc::clone(&bundle.translation_manager),
        Arc::clone(&bundle.config_manager),
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
    router.register(Arc::new(SystemMonitoringHandler::new(Arc::new(
        event_bus.clone(),
    ))));
    router.register(Arc::new(NvimHandler::new(Arc::new(event_bus.clone()))));
    router.register(Arc::new(LearningMaterialHandler::new(
        Arc::clone(&bundle.config_manager),
        Arc::clone(&bundle.note_manager),
    )));
    router.register(Arc::new(AutomationHandler));
    router.register(Arc::new(PluginHandler));
    router
}

impl AppState {
    pub(crate) fn new() -> Self {
        let event_bus = EventBus::default();
        let action_arena = Arc::new(ActionArena::default());
        let knowledge_store = KnowledgeStoreHandle::new_default();

        let bundle = create_managers(&event_bus);
        let command_router =
            build_command_router(&bundle, &event_bus, &action_arena, &knowledge_store);

        Self {
            command_router,
            action_arena,
            event_bus,
            knowledge_store,
            mouse_active: Arc::new(AtomicBool::new(false)),
            launcher_focus_guard: Arc::new(Mutex::new(None)),
            _config_manager: bundle.config_manager,
            _config_watcher: Arc::new(Mutex::new(None)),
            _history_manager: bundle.history_manager,
            _search_manager: bundle.search_manager,
            _workspace_manager: bundle.workspace_manager,
        }
    }
}