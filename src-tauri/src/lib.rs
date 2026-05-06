pub mod core;
mod handlers;
mod managers;
mod models;
mod platform;

use std::sync::{
    atomic::{AtomicBool, Ordering as AtomicOrd},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{Emitter, Manager};

use crate::core::config_manager::{ConfigChange, ConfigManager};
use crate::core::control_plane::{self, ControlCommand, ControlRequest, ControlResponse};
use crate::core::{AppEvent, BuiltinCommandRegistry, CommandRouter, EventBus};
use crate::handlers::{
    ai::AiHandler,
    builtin_cmd::{
        AiCommand, BuiltinCmdHandler, CalCommand, DownCommand, HelpCommand, HistoryCommand,
        ModelDownloadCommand, ModelListCommand, NoteCommand, ReloadCommand, SettingCommand,
        SysCtlCommand, TrCommand,
    },
    calculator::CalculatorHandler,
    history::HistoryHandler,
    hotkey::HotkeyHandler,
    launcher::LauncherHandler,
    model::ModelHandler,
    mouse::MouseHandler,
    note::NoteHandler,
    search::SearchHandler,
    setting::SettingHandler,
    system_control::SystemControlHandler,
    terminal::TerminalHandler,
    translation::TranslationHandler,
    workspace::WorkspaceHandler,
};
use crate::managers::{
    ai_manager::AiManager,
    app_manager::AppManager,
    calculator_manager::CalculatorManager,
    history_manager::HistoryManager,
    hotkey_manager::HotkeyManager,
    model_manager::ModelManager,
    mouse_manager::MouseManager,
    note_manager::NoteManager,
    search_manager::SearchManager,
    system_manager::SystemManager,
    terminal_manager::{start_prewarm, TerminalManager},
    translation_manager::TranslationManager,
    workspace_manager::WorkspaceManager,
};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};

// ─── App state ───────────────────────────────────────────────────────────────

struct AppState {
    command_router: CommandRouter,
    event_bus: EventBus,
    mouse_active: Arc<AtomicBool>,
    terminal_manager: Arc<Mutex<TerminalManager>>,
    launcher_focus_guard: Arc<Mutex<Option<Instant>>>,
    _config_manager: Arc<Mutex<ConfigManager>>,
    _config_watcher: Arc<Mutex<Option<notify::RecommendedWatcher>>>,
    _history_manager: Arc<Mutex<HistoryManager>>,
    _workspace_manager: Arc<Mutex<WorkspaceManager>>,
}

impl AppState {
    fn new() -> Self {
        let event_bus = EventBus::default();

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

        let search_manager = Arc::new(Mutex::new(SearchManager::new(Arc::clone(&app_manager))));

        // AI manager (with EventBus callback)
        let eb_for_ai = event_bus.clone();
        let ai_manager = Arc::new(AiManager::new(Arc::new(move |event| {
            let _ = eb_for_ai.publish(event);
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
            reg.register(Box::new(NoteCommand));
            reg.register(Box::new(CalCommand));
            reg.register(Box::new(HistoryCommand));
            reg.register(Box::new(SysCtlCommand));
        }

        let mut command_router = CommandRouter::new();
        command_router.register(Arc::new(SystemControlHandler::new(Arc::clone(
            &system_manager,
        ))));
        command_router.register(Arc::new(LauncherHandler::new(Arc::clone(&app_manager))));
        command_router.register(Arc::new(HotkeyHandler::new(Arc::clone(&hotkey_manager))));
        command_router.register(Arc::new(TerminalHandler::new(Arc::clone(
            &terminal_manager,
        ))));
        command_router.register(Arc::new(MouseHandler::new(Arc::clone(&mouse_manager))));
        command_router.register(Arc::new(SearchHandler::new(Arc::clone(&search_manager))));
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
        )));
        command_router.register(Arc::new(SettingHandler::new(Arc::clone(&config_manager))));
        command_router.register(Arc::new(CalculatorHandler::new(Arc::clone(
            &calculator_manager,
        ))));
        command_router.register(Arc::new(WorkspaceHandler::new(Arc::clone(
            &workspace_manager,
        ))));
        command_router.register(Arc::new(NoteHandler::new(Arc::clone(&note_manager))));
        command_router.register(Arc::new(HistoryHandler::new(Arc::clone(&history_manager))));
        command_router.register(Arc::new(AiHandler::new(
            Arc::clone(&ai_manager),
            Arc::clone(&config_manager),
        )));
        command_router.register(Arc::new(TranslationHandler::new(
            Arc::clone(&translation_manager),
            Arc::clone(&config_manager),
        )));

        Self {
            command_router,
            event_bus,
            mouse_active: Arc::new(AtomicBool::new(false)),
            terminal_manager: Arc::clone(&terminal_manager),
            launcher_focus_guard: Arc::new(Mutex::new(None)),
            _config_manager: config_manager,
            _config_watcher: Arc::new(Mutex::new(None)),
            _history_manager: history_manager,
            _workspace_manager: workspace_manager,
        }
    }
}

// ─── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn cmd_dispatch(
    route: String,
    payload: Option<Value>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    let payload = payload.unwrap_or(Value::Null);

    if let Some(name) = builtin_control_command(&route, &payload) {
        return run_builtin_control_command(&app, name);
    }

    let before = should_apply_config_after_dispatch(&route, &payload)
        .then(|| {
            state
                ._config_manager
                .lock()
                .map(|cfg| cfg.snapshot())
                .map_err(|e| e.to_string())
        })
        .transpose()?;

    let result = state.command_router.dispatch(&route, payload)?;

    if let Some(before) = before {
        let after = state
            ._config_manager
            .lock()
            .map(|cfg| cfg.snapshot())
            .map_err(|e| e.to_string())?;
        let changes = ConfigManager::diff(&before, &after);
        apply_config_changes(&app, changes, "setting.set", false)?;
    }

    Ok(result)
}

#[tauri::command]
fn cmd_ping(name: Option<String>, state: tauri::State<'_, AppState>) -> Result<String, String> {
    let name = name.unwrap_or_else(|| "Developer".to_string());
    let response = state
        .command_router
        .dispatch("system.ping", json!({ "name": name }))?;
    let message = response
        .get("message")
        .and_then(Value::as_str)
        .ok_or("missing message")?
        .to_string();
    let _ = state.event_bus.publish(AppEvent::new(
        "system.ping.completed",
        json!({ "message": message, "registered_handlers": state.command_router.handler_count() }),
    ));
    Ok(message)
}

#[tauri::command]
fn cmd_hide_launcher(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_show_launcher(window: tauri::WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
fn cmd_keep_launcher_open(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    if let Ok(mut guard) = state.launcher_focus_guard.lock() {
        *guard = Some(Instant::now() + Duration::from_millis(600));
    }
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

fn builtin_control_command<'a>(route: &str, payload: &'a Value) -> Option<&'a str> {
    if route != "cmd.run" {
        return None;
    }
    let name = payload.get("name").and_then(Value::as_str)?;
    matches!(name, "reload" | "down").then_some(name)
}

fn should_apply_config_after_dispatch(route: &str, payload: &Value) -> bool {
    if route == "setting.set" {
        return true;
    }
    if route != "cmd.run" {
        return false;
    }
    let Some("setting") = payload.get("name").and_then(Value::as_str) else {
        return false;
    };
    payload
        .get("args")
        .and_then(Value::as_str)
        .map(|args| {
            args.split_once(' ')
                .is_some_and(|(_, v)| !v.trim().is_empty())
        })
        .unwrap_or(false)
}

fn run_builtin_control_command(app: &tauri::AppHandle, name: &str) -> Result<Value, String> {
    match name {
        "reload" => {
            let changes = reload_config_from_disk(app, "command", true)?;
            Ok(json!(BuiltinCommandResult {
                text: format!("Reloaded config ({} change(s))", changes.len()),
                ui_type: CommandUiType::Inline,
            }))
        }
        "down" => {
            schedule_shutdown(app);
            Ok(json!(BuiltinCommandResult {
                text: "Keynova is shutting down".into(),
                ui_type: CommandUiType::Inline,
            }))
        }
        _ => Err(format!("unknown control command '/{name}'")),
    }
}

fn reload_config_from_disk(
    app: &tauri::AppHandle,
    source: &str,
    emit_on_empty: bool,
) -> Result<Vec<ConfigChange>, String> {
    let state = app.state::<AppState>();
    let changes = match state._config_manager.lock() {
        Ok(mut cfg) => cfg.reload_from_disk(),
        Err(e) => Err(e.to_string()),
    };

    match changes {
        Ok(changes) => {
            apply_config_changes(app, changes.clone(), source, emit_on_empty)?;
            Ok(changes)
        }
        Err(error) => {
            publish_config_reload_failed(app, source, &error);
            Err(error)
        }
    }
}

fn apply_config_changes(
    app: &tauri::AppHandle,
    changes: Vec<ConfigChange>,
    source: &str,
    emit_on_empty: bool,
) -> Result<(), String> {
    if changes
        .iter()
        .any(|change| change.key.starts_with("hotkeys."))
    {
        setup_global_shortcuts(app, true);
    }

    if emit_on_empty || !changes.is_empty() {
        publish_config_reloaded(app, source, &changes);
    }

    Ok(())
}

fn publish_config_reloaded(app: &tauri::AppHandle, source: &str, changes: &[ConfigChange]) {
    let state = app.state::<AppState>();
    let changed_keys: Vec<_> = changes.iter().map(|change| change.key.clone()).collect();
    let _ = state.event_bus.publish(AppEvent::new(
        "config.reloaded",
        json!({
            "source": source,
            "changed_keys": changed_keys,
            "changes": changes,
        }),
    ));
}

fn publish_config_reload_failed(app: &tauri::AppHandle, source: &str, error: &str) {
    let state = app.state::<AppState>();
    let _ = state.event_bus.publish(AppEvent::new(
        "config.reload_failed",
        json!({
            "source": source,
            "error": error,
        }),
    ));
}

fn schedule_shutdown(app: &tauri::AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(75)).await;
        handle.exit(0);
    });
}

fn show_launcher(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    let _ = window.unminimize();
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = show_launcher(app);
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            if !start_control_server(app) {
                return Ok(());
            }

            // Bridge EventBus (Rust broadcast) → Tauri frontend events
            let mut rx = app.state::<AppState>().event_bus.subscribe();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                use tokio::sync::broadcast::error::RecvError;
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            let topic = event.topic.replace('.', "-");
                            let _ = handle.emit(&topic, &event.payload);
                        }
                        Err(RecvError::Lagged(_)) => continue,
                        Err(RecvError::Closed) => break,
                    }
                }
            });

            prescan_apps(app);
            start_file_index();
            start_prewarm(Arc::clone(&app.state::<AppState>().terminal_manager));
            start_clipboard_watcher(app);
            setup_global_shortcuts(app.handle(), false);
            setup_config_watcher(app);
            setup_tray(app)?;
            setup_main_window(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            cmd_ping,
            cmd_dispatch,
            cmd_hide_launcher,
            cmd_show_launcher,
            cmd_keep_launcher_open,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ─── Setup helpers ───────────────────────────────────────────────────────────

fn start_control_server(app: &tauri::App) -> bool {
    let listener = match control_plane::bind_listener() {
        Ok(listener) => listener,
        Err(bind_error) => {
            match control_plane::send_request(ControlCommand::Start, Duration::from_millis(400)) {
                Ok(response) if response.ok => {
                    eprintln!("[keynova] existing instance detected, handing off and exiting");
                    let handle = app.handle().clone();
                    schedule_shutdown(&handle);
                    return false;
                }
                _ => {
                    eprintln!("[keynova] control plane disabled: {bind_error}");
                    return true;
                }
            }
        }
    };

    let handle = app.handle().clone();
    std::thread::spawn(move || {
        let handler: Arc<dyn Fn(ControlRequest) -> ControlResponse + Send + Sync> = {
            let handle = handle.clone();
            Arc::new(move |request| handle_control_request(&handle, request))
        };
        if let Err(e) = control_plane::serve_listener(listener, handler) {
            eprintln!("[keynova] control plane disabled: {e}");
        }
    });

    true
}

fn handle_control_request(app: &tauri::AppHandle, request: ControlRequest) -> ControlResponse {
    match request.command {
        ControlCommand::Start => match show_launcher(app) {
            Ok(()) => ControlResponse::ok("Keynova is focused", json!({ "visible": true })),
            Err(e) => ControlResponse::error(e),
        },
        ControlCommand::Down => {
            schedule_shutdown(app);
            ControlResponse::ok("Keynova is shutting down", json!(null))
        }
        ControlCommand::Reload => match reload_config_from_disk(app, "cli", true) {
            Ok(changes) => ControlResponse::ok(
                format!("Reloaded config ({} change(s))", changes.len()),
                json!({ "changed_keys": changes.iter().map(|c| c.key.clone()).collect::<Vec<_>>() }),
            ),
            Err(e) => ControlResponse::error(e),
        },
        ControlCommand::Status => {
            let state = app.state::<AppState>();
            let config_path = state
                ._config_manager
                .lock()
                .map(|cfg| cfg.config_path().display().to_string())
                .unwrap_or_else(|_| "<locked>".into());
            ControlResponse::ok(
                "Keynova is running",
                json!({
                    "config_path": config_path,
                    "handlers": state.command_router.handler_count(),
                }),
            )
        }
    }
}

fn setup_config_watcher(app: &tauri::App) {
    let handle = app.handle().clone();
    let state = app.state::<AppState>();
    let config_path = match state._config_manager.lock() {
        Ok(cfg) => cfg.config_path(),
        Err(e) => {
            eprintln!("[keynova] config watcher skipped: {e}");
            return;
        }
    };

    match start_config_file_watcher(handle, config_path) {
        Ok(watcher) => {
            if let Ok(mut slot) = state._config_watcher.lock() {
                *slot = Some(watcher);
            }
        }
        Err(e) => eprintln!("[keynova] config watcher disabled: {e}"),
    }
}

fn start_config_file_watcher(
    app: tauri::AppHandle,
    config_path: std::path::PathBuf,
) -> Result<notify::RecommendedWatcher, String> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

    let watch_dir = config_path
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| "invalid config path".to_string())?;
    std::fs::create_dir_all(&watch_dir).map_err(|e| e.to_string())?;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            let _ = tx.send(event);
        },
        Config::default(),
    )
    .map_err(|e| e.to_string())?;
    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        let mut last_reload = Instant::now() - Duration::from_secs(1);
        for event in rx {
            let Ok(event) = event else {
                continue;
            };
            if !is_config_file_event(&event, &config_path) {
                continue;
            }
            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                continue;
            }
            if last_reload.elapsed() < Duration::from_millis(250) {
                continue;
            }
            last_reload = Instant::now();
            std::thread::sleep(Duration::from_millis(150));
            if let Err(e) = reload_config_from_disk(&app, "watcher", false) {
                eprintln!("[keynova] config watcher reload failed: {e}");
            }
        }
    });

    Ok(watcher)
}

fn is_config_file_event(event: &notify::Event, config_path: &std::path::Path) -> bool {
    let Some(wanted_name) = config_path.file_name() else {
        return false;
    };
    event
        .paths
        .iter()
        .any(|path| path == config_path || path.file_name().is_some_and(|name| name == wanted_name))
}

/// 背景執行緒監聽剪貼簿變化（Windows: 輪詢每 800ms）。
fn start_clipboard_watcher(app: &tauri::App) {
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        clipboard_poll_loop(&handle);
    });
}

fn clipboard_poll_loop(app: &tauri::AppHandle) {
    loop {
        std::thread::sleep(Duration::from_secs(2));
        if let Some(text) = read_clipboard_text() {
            let state = app.state::<AppState>();
            let added = state
                ._history_manager
                .lock()
                .map(|mut mgr| mgr.push_text(text))
                .unwrap_or(false);
            if added {
                let _ = state
                    .event_bus
                    .publish(AppEvent::new("history.updated", json!({})));
            }
        }
    }
}

fn read_clipboard_text() -> Option<String> {
    platform_read_clipboard()
}

#[cfg(target_os = "windows")]
fn platform_read_clipboard() -> Option<String> {
    // Use PowerShell Get-Clipboard for simplicity; runs in a background thread (2s interval)
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "Get-Clipboard"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(not(target_os = "windows"))]
fn platform_read_clipboard() -> Option<String> {
    None
}

fn prescan_apps(app: &tauri::App) {
    let app_handle = app.handle().clone();
    std::thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        let _ = state
            .command_router
            .dispatch("launcher.list_all", Value::Null);
    });
}

fn start_file_index() {
    #[cfg(target_os = "windows")]
    std::thread::spawn(|| {
        crate::platform::windows::build_file_index();
    });
}

fn setup_global_shortcuts(app: &tauri::AppHandle, reset_existing: bool) {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let gs = app.global_shortcut();
    if reset_existing {
        if let Err(e) = gs.unregister_all() {
            eprintln!("[keynova] shortcut unregister_all failed: {e}");
        }
    }

    let launcher_key = app
        .state::<AppState>()
        ._config_manager
        .lock()
        .map(|c| {
            c.get("hotkeys.app_launcher")
                .unwrap_or_else(|| "Ctrl+K".to_string())
        })
        .unwrap_or_else(|_| "Ctrl+K".to_string());
    let mouse_key = app
        .state::<AppState>()
        ._config_manager
        .lock()
        .map(|c| {
            c.get("hotkeys.mouse_control")
                .unwrap_or_else(|| "Ctrl+Alt+M".to_string())
        })
        .unwrap_or_else(|_| "Ctrl+Alt+M".to_string());

    let candidates: &[&str] = if launcher_key == "Ctrl+K" {
        &["Ctrl+K"]
    } else {
        &[launcher_key.as_str(), "Ctrl+K"]
    };
    'launcher: for &key in candidates {
        let handle_k = app.clone();
        if let Err(e) = gs.on_shortcut(key, move |_app, _, event| {
            if event.state() == ShortcutState::Pressed {
                if let Some(win) = handle_k.get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        let _ = win.hide();
                    } else {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
            }
        }) {
            eprintln!("[keynova] {key} registration failed: {e}");
        } else {
            break 'launcher;
        }
    }

    let mouse_candidates: &[&str] = if mouse_key == "Ctrl+Alt+M" {
        &["Ctrl+Alt+M"]
    } else {
        &[mouse_key.as_str(), "Ctrl+Alt+M"]
    };
    'mouse: for &key in mouse_candidates {
        let handle_m = app.clone();
        if let Err(e) = gs.on_shortcut(key, move |app_h, _, event| {
            if event.state() == ShortcutState::Pressed {
                let state = app_h.state::<AppState>();
                let new_val = !state.mouse_active.load(AtomicOrd::Relaxed);
                state.mouse_active.store(new_val, AtomicOrd::Relaxed);
                if let Some(win) = handle_m.get_webview_window("main") {
                    let _ = win.emit("mouse-control-toggled", new_val);
                }
            }
        }) {
            eprintln!("[keynova] {key} (mouse_control) registration failed: {e}");
        } else {
            break 'mouse;
        }
    }

    // ── Ctrl+Alt+W/A/S/D: cursor movement ──
    for (shortcut, dx_dir, dy_dir) in [
        ("Ctrl+Alt+W", 0i32, -1i32),
        ("Ctrl+Alt+A", -1i32, 0i32),
        ("Ctrl+Alt+S", 0i32, 1i32),
        ("Ctrl+Alt+D", 1i32, 0i32),
    ] {
        if let Err(e) = gs.on_shortcut(shortcut, move |app_h, _, event| {
            if event.state() == ShortcutState::Pressed {
                let state = app_h.state::<AppState>();
                if state.mouse_active.load(AtomicOrd::Relaxed) {
                    let step = state
                        ._config_manager
                        .lock()
                        .ok()
                        .and_then(|c| c.get("mouse_control.step_size"))
                        .and_then(|v| v.parse::<i32>().ok())
                        .unwrap_or(15);
                    let _ = state.command_router.dispatch(
                        "mouse.move_relative",
                        json!({ "dx": dx_dir * step, "dy": dy_dir * step }),
                    );
                }
            }
        }) {
            eprintln!("[keynova] {shortcut} registration failed: {e}");
        }
    }

    // ── Ctrl+Alt+Enter: left click ──
    if let Err(e) = gs.on_shortcut("Ctrl+Alt+Enter", move |app_h, _, event| {
        if event.state() == ShortcutState::Pressed {
            let state = app_h.state::<AppState>();
            if state.mouse_active.load(AtomicOrd::Relaxed) {
                let _ = state
                    .command_router
                    .dispatch("mouse.click", json!({ "button": "left", "count": 1 }));
            }
        }
    }) {
        eprintln!("[keynova] Ctrl+Alt+Enter registration failed: {e}");
    }

    // ── Ctrl+Alt+1/2/3: workspace switch ──
    for (shortcut, slot) in [("Ctrl+Alt+1", 0usize), ("Ctrl+Alt+2", 1), ("Ctrl+Alt+3", 2)] {
        let handle_ws = app.clone();
        if let Err(e) = gs.on_shortcut(shortcut, move |app_h, _, event| {
            if event.state() == ShortcutState::Pressed {
                let payload = {
                    let state = app_h.state::<AppState>();
                    state
                        ._workspace_manager
                        .lock()
                        .ok()
                        .and_then(|mut mgr| mgr.switch_to(slot).ok().map(|ws| json!(ws)))
                };
                if let Some(payload) = payload {
                    if let Some(win) = handle_ws.get_webview_window("main") {
                        let _ = win.emit("workspace-switched", &payload);
                    }
                }
            }
        }) {
            eprintln!("[keynova] {shortcut} (workspace) registration failed: {e}");
        }
    }
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let show_item = MenuItem::with_id(app, "show", "顯示 Keynova (Ctrl+K)", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let handle = app.handle().clone();

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Keynova")
        .on_menu_event(move |_tray, event| match event.id.as_ref() {
            "show" => {
                if let Some(win) = handle.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => handle.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

fn setup_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen = monitor.size();
        let scale = monitor.scale_factor();
        let phys_w = (640.0 * scale) as i32;
        let x = ((screen.width as i32 - phys_w) / 2).max(0);
        let y = (screen.height as f64 * 0.25) as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }

    let window_focused = window.clone();
    let window_blur = window.clone();
    let blur_guard = app.state::<AppState>().launcher_focus_guard.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Focused(true) => {
            let _ = window_focused.emit("window-focused", ());
        }
        tauri::WindowEvent::Focused(false) => {
            let window_blur = window_blur.clone();
            let blur_guard = Arc::clone(&blur_guard);
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(120)).await;
                let should_keep_open = blur_guard
                    .lock()
                    .map(|mut guard| {
                        let now = Instant::now();
                        match guard.as_ref() {
                            Some(until) if now <= *until => true,
                            Some(_) => {
                                *guard = None;
                                false
                            }
                            None => false,
                        }
                    })
                    .unwrap_or(false);
                if should_keep_open {
                    let _ = window_blur.show();
                    let _ = window_blur.set_focus();
                    return;
                }
                if window_blur.is_focused().unwrap_or(false) {
                    return;
                }
                let _ = window_blur.hide();
            });
        }
        _ => {}
    });

    Ok(())
}
