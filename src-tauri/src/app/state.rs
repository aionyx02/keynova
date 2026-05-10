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
    history::HistoryHandler,
    hotkey::HotkeyHandler,
    launcher::LauncherHandler,
    model::ModelHandler,
    mouse::MouseHandler,
    note::NoteHandler,
    plugin::PluginHandler,
    search::{SearchHandler, SearchHandlerDeps},
    setting::SettingHandler,
    system_control::SystemControlHandler,
    system_monitoring::SystemMonitoringHandler,
    terminal::TerminalHandler,
    translation::TranslationHandler,
    workspace::WorkspaceHandler,
};
use crate::managers::{
    ai_manager::AiManager, app_manager::AppManager, calculator_manager::CalculatorManager,
    history_manager::HistoryManager, hotkey_manager::HotkeyManager, model_manager::ModelManager,
    mouse_manager::MouseManager, note_manager::NoteManager, search_manager::SearchManager,
    system_manager::SystemManager, terminal_manager::TerminalManager,
    translation_manager::TranslationManager, workspace_manager::WorkspaceManager,
};
pub(crate) struct AppState {
    pub(crate) command_router: CommandRouter,
    pub(crate) action_arena: Arc<ActionArena>,
    pub(crate) event_bus: EventBus,
    pub(crate) knowledge_store: KnowledgeStoreHandle,
    pub(crate) mouse_active: Arc<AtomicBool>,
    pub(crate) terminal_manager: Arc<Mutex<TerminalManager>>,
    pub(crate) launcher_focus_guard: Arc<Mutex<Option<Instant>>>,
    pub(crate) _config_manager: Arc<Mutex<ConfigManager>>,
    pub(crate) _config_watcher: Arc<Mutex<Option<notify::RecommendedWatcher>>>,
    pub(crate) _history_manager: Arc<Mutex<HistoryManager>>,
    pub(crate) _search_manager: Arc<Mutex<SearchManager>>,
    pub(crate) _workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

impl AppState {
    pub(crate) fn new() -> Self {
        let event_bus = EventBus::default();
        let action_arena = Arc::new(ActionArena::default());
        let knowledge_store = KnowledgeStoreHandle::new_default();

        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let hotkey_manager = Arc::new(Mutex::new(HotkeyManager::new()));
        let mouse_manager = Arc::new(Mutex::new(MouseManager::new()));
        let system_manager = Arc::new(Mutex::new(SystemManager::new()));
        let calculator_manager = Arc::new(Mutex::new(CalculatorManager::new()));
        let workspace_manager = Arc::new(Mutex::new(WorkspaceManager::new()));
        let model_manager = Arc::new(ModelManager::new());

        // Config manager (shared)
        let config_manager = Arc::new(Mutex::new(ConfigManager::new()));

        // NoteManager reads storage_dir from config
        let note_storage_dir = config_manager
            .lock()
            .ok()
            .and_then(|c| c.get("notes.storage_dir"));
        let note_manager = Arc::new(Mutex::new(NoteManager::new(note_storage_dir)));

        // HistoryManager reads max_items from config
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

        // AI manager (with EventBus callback)
        let eb_for_ai = event_bus.clone();
        let ai_manager = Arc::new(AiManager::new(Arc::new(move |event| {
            let _ = eb_for_ai.publish(event);
        })));

        let eb_for_agent = event_bus.clone();
        let agent_runtime = Arc::new(AgentRuntime::new(Arc::new(move |event| {
            let _ = eb_for_agent.publish(event);
        })));

        // Translation manager (with EventBus callback)
        let eb_for_tr = event_bus.clone();
        let translation_manager = Arc::new(TranslationManager::new(Arc::new(move |event| {
            let _ = eb_for_tr.publish(event);
        })));

        // BuiltinCommandRegistry
        let builtin_registry = Arc::new(Mutex::new(BuiltinCommandRegistry::new()));
        {
            let mut reg = builtin_registry.lock().expect("registry init");
            reg.register(Box::new(HelpCommand));
            reg.register(Box::new(SettingCommand));
            reg.register(Box::new(ReloadCommand));
            reg.register(Box::new(DownCommand));
            // Phase 3 commands
            reg.register(Box::new(TrCommand));
            reg.register(Box::new(AiCommand));
            reg.register(Box::new(ModelDownloadCommand));
            reg.register(Box::new(ModelListCommand));
            reg.register(Box::new(ModelRemoveCommand::new(
                Arc::clone(&model_manager),
                Arc::clone(&config_manager),
            )));
            reg.register(Box::new(NoteCommand::new(
                Arc::clone(&note_manager),
                Arc::clone(&config_manager),
            )));
            reg.register(Box::new(CalCommand));
            reg.register(Box::new(HistoryCommand));
            reg.register(Box::new(SysCtlCommand));
            reg.register(Box::new(SysMonitorCommand));
            reg.register(Box::new(RebuildSearchIndexCommand));
        }

        let mut command_router = CommandRouter::new();
        command_router.register(Arc::new(SystemControlHandler::new(Arc::clone(
            &system_manager,
        ))));
        command_router.register(Arc::new(LauncherHandler::new(Arc::clone(&app_manager))));
        command_router.register(Arc::new(HotkeyHandler::new(Arc::clone(&hotkey_manager))));
        command_router.register(Arc::new(TerminalHandler::new(
            Arc::clone(&terminal_manager),
            Arc::clone(&workspace_manager),
        )));
        command_router.register(Arc::new(MouseHandler::new(Arc::clone(&mouse_manager))));
        command_router.register(Arc::new(SearchHandler::new(SearchHandlerDeps {
            manager: Arc::clone(&search_manager),
            action_arena: Arc::clone(&action_arena),
            builtin_registry: Arc::clone(&builtin_registry),
            note_manager: Arc::clone(&note_manager),
            history_manager: Arc::clone(&history_manager),
            workspace_manager: Arc::clone(&workspace_manager),
            model_manager: Arc::clone(&model_manager),
            event_bus: event_bus.clone(),
        })));
        let eb_for_model = event_bus.clone();
        command_router.register(Arc::new(ModelHandler::new(
            Arc::clone(&model_manager),
            Arc::clone(&config_manager),
            Arc::new(move |event| {
                let _ = eb_for_model.publish(event);
            }),
        )));
        command_router.register(Arc::new(BuiltinCmdHandler::new(
            Arc::clone(&builtin_registry),
            Arc::clone(&config_manager),
            Arc::clone(&search_manager),
        )));
        command_router.register(Arc::new(SettingHandler::new(Arc::clone(&config_manager))));
        command_router.register(Arc::new(CalculatorHandler::new(Arc::clone(
            &calculator_manager,
        ))));
        command_router.register(Arc::new(WorkspaceHandler::new(Arc::clone(
            &workspace_manager,
        ))));
        command_router.register(Arc::new(NoteHandler::new(
            Arc::clone(&note_manager),
            Arc::clone(&workspace_manager),
        )));
        command_router.register(Arc::new(HistoryHandler::new(Arc::clone(&history_manager))));
        command_router.register(Arc::new(AiHandler::new(
            Arc::clone(&ai_manager),
            Arc::clone(&config_manager),
            Arc::clone(&workspace_manager),
            Arc::clone(&model_manager),
        )));
        command_router.register(Arc::new(TranslationHandler::new(
            Arc::clone(&translation_manager),
            Arc::clone(&config_manager),
        )));
        command_router.register(Arc::new(AgentHandler::new(AgentHandlerDeps {
            runtime: Arc::clone(&agent_runtime),
            config: Arc::clone(&config_manager),
            note_manager: Arc::clone(&note_manager),
            history_manager: Arc::clone(&history_manager),
            workspace_manager: Arc::clone(&workspace_manager),
            builtin_registry: Arc::clone(&builtin_registry),
            model_manager: Arc::clone(&model_manager),
            knowledge_store: knowledge_store.clone(),
        })));
        command_router.register(Arc::new(SystemMonitoringHandler::new(Arc::new(
            event_bus.clone(),
        ))));
        command_router.register(Arc::new(AutomationHandler));
        command_router.register(Arc::new(PluginHandler));

        Self {
            command_router,
            action_arena,
            event_bus,
            knowledge_store,
            mouse_active: Arc::new(AtomicBool::new(false)),
            terminal_manager: Arc::clone(&terminal_manager),
            launcher_focus_guard: Arc::new(Mutex::new(None)),
            _config_manager: config_manager,
            _config_watcher: Arc::new(Mutex::new(None)),
            _history_manager: history_manager,
            _search_manager: search_manager,
            _workspace_manager: workspace_manager,
        }
    }
}
