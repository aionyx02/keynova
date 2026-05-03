mod core;
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

use crate::core::{AppEvent, BuiltinCommandRegistry, CommandHandler, CommandResult, CommandRouter, EventBus};
use crate::core::config_manager::ConfigManager;
use crate::handlers::{
    builtin_cmd::{BuiltinCmdHandler, HelpCommand, SettingCommand},
    hotkey::HotkeyHandler,
    launcher::LauncherHandler,
    mouse::MouseHandler,
    search::SearchHandler,
    setting::SettingHandler,
    terminal::TerminalHandler,
};
use crate::managers::{
    app_manager::AppManager,
    hotkey_manager::HotkeyManager,
    mouse_manager::MouseManager,
    search_manager::SearchManager,
    terminal_manager::{start_prewarm, TerminalManager},
};

// ─── System handler ──────────────────────────────────────────────────────────

struct SystemHandler;

impl CommandHandler for SystemHandler {
    fn namespace(&self) -> &'static str {
        "system"
    }

    fn execute(&self, command: &str, payload: Value) -> CommandResult {
        match command {
            "ping" => {
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or("Developer");
                Ok(json!({ "message": format!("Pong from Rust CommandRouter, {}!", name) }))
            }
            _ => Err(format!("unknown system command '{command}'")),
        }
    }
}

// ─── App state ───────────────────────────────────────────────────────────────

struct AppState {
    command_router: CommandRouter,
    event_bus: EventBus,
    /// 全域滑鼠控制模式是否啟用（Alt+M 切換）
    mouse_active: Arc<AtomicBool>,
    /// Shared reference for triggering terminal pre-warm from setup
    terminal_manager: Arc<Mutex<TerminalManager>>,
    /// Suppresses blur-to-hide while the launcher switches internal UI modes.
    launcher_focus_guard: Arc<Mutex<Option<Instant>>>,
    /// 使用者設定（TOML I/O）
    _config_manager: Arc<Mutex<ConfigManager>>,
}

impl AppState {
    fn new() -> Self {
        let event_bus = EventBus::default();

        let app_manager = Arc::new(Mutex::new(AppManager::new()));
        let hotkey_manager = Arc::new(Mutex::new(HotkeyManager::new()));
        let mouse_manager = Arc::new(Mutex::new(MouseManager::new()));

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
        let config_manager = Arc::new(Mutex::new(ConfigManager::new()));

        // BuiltinCommandRegistry：先建立 Arc，再注冊指令，最後傳給 handler
        let builtin_registry = Arc::new(Mutex::new(BuiltinCommandRegistry::new()));
        {
            let mut reg = builtin_registry.lock().expect("registry init");
            reg.register(Box::new(HelpCommand));
            reg.register(Box::new(SettingCommand));
        }

        let mut command_router = CommandRouter::new();
        command_router.register(Arc::new(SystemHandler));
        command_router.register(Arc::new(LauncherHandler::new(Arc::clone(&app_manager))));
        command_router.register(Arc::new(HotkeyHandler::new(Arc::clone(&hotkey_manager))));
        command_router.register(Arc::new(TerminalHandler::new(Arc::clone(&terminal_manager))));
        command_router.register(Arc::new(MouseHandler::new(Arc::clone(&mouse_manager))));
        command_router.register(Arc::new(SearchHandler::new(Arc::clone(&search_manager))));
        command_router.register(Arc::new(BuiltinCmdHandler::new(Arc::clone(&builtin_registry))));
        command_router.register(Arc::new(SettingHandler::new(Arc::clone(&config_manager))));

        Self {
            command_router,
            event_bus,
            mouse_active: Arc::new(AtomicBool::new(false)),
            terminal_manager: Arc::clone(&terminal_manager),
            launcher_focus_guard: Arc::new(Mutex::new(None)),
            _config_manager: config_manager,
        }
    }
}

// ─── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn cmd_dispatch(
    route: String,
    payload: Option<Value>,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    state
        .command_router
        .dispatch(&route, payload.unwrap_or(Value::Null))
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

// ─── Entry point ─────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            // Bridge: EventBus (Rust broadcast) → Tauri frontend events.
            // Tauri event names cannot contain '.', so expose dotted internal topics as kebab names.
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
            // Pre-warm one terminal session so the first open is nearly instant
            start_prewarm(Arc::clone(&app.state::<AppState>().terminal_manager));
            // Shortcut registration is best-effort; conflicts are logged, not fatal
            setup_global_shortcuts(app);
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

/// 在背景執行緒預先掃描應用程式清單，確保首次搜尋無冷啟動延遲。
fn prescan_apps(app: &tauri::App) {
    let app_handle = app.handle().clone();
    std::thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        let _ = state
            .command_router
            .dispatch("launcher.list_all", Value::Null);
    });
}

/// 在背景執行緒建立檔案索引快取（Windows only），之後查詢只讀記憶體，不掃碟。
fn start_file_index() {
    #[cfg(target_os = "windows")]
    std::thread::spawn(|| {
        crate::platform::windows::build_file_index();
    });
}

/// 快捷鍵註冊為 best-effort：衝突時僅 eprintln，不中斷 setup。
fn setup_global_shortcuts(app: &tauri::App) {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    const STEP: i32 = 15;
    let gs = app.global_shortcut();

    // ── 讀取設定中的快捷鍵（修改 config.toml 並重啟後生效）──
    let launcher_key = app
        .state::<AppState>()
        ._config_manager
        .lock()
        .map(|c| c.get("hotkeys.app_launcher").unwrap_or_else(|| "Ctrl+K".to_string()))
        .unwrap_or_else(|_| "Ctrl+K".to_string());
    let mouse_key = app
        .state::<AppState>()
        ._config_manager
        .lock()
        .map(|c| c.get("hotkeys.mouse_control").unwrap_or_else(|| "Ctrl+Alt+M".to_string()))
        .unwrap_or_else(|_| "Ctrl+Alt+M".to_string());

    // ── 開啟 / 關閉啟動器視窗 ──
    let handle_k = app.handle().clone();
    if let Err(e) = gs.on_shortcut(launcher_key.as_str(), move |_app, _, event| {
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
        eprintln!("[keynova] {launcher_key} registration failed (conflict?): {e}");
    }

    // ── 切換全域滑鼠控制模式 ──
    // 使用 Ctrl+Alt 而非單獨 Alt，避免 Alt 觸發 Windows 選單列造成修飾鍵殘留（卡鍵）
    let handle_m = app.handle().clone();
    if let Err(e) = gs.on_shortcut(mouse_key.as_str(), move |app_h, _, event| {
        if event.state() == ShortcutState::Pressed {
            let state = app_h.state::<AppState>();
            let new_val = !state.mouse_active.load(AtomicOrd::Relaxed);
            state.mouse_active.store(new_val, AtomicOrd::Relaxed);
            if let Some(win) = handle_m.get_webview_window("main") {
                let _ = win.emit("mouse-control-toggled", new_val);
            }
        }
    }) {
        eprintln!("[keynova] {mouse_key} registration failed: {e}");
    }

    // ── Ctrl+Alt+W/A/S/D: cursor movement ──
    for (shortcut, dx, dy) in [
        ("Ctrl+Alt+W", 0i32, -STEP),
        ("Ctrl+Alt+A", -STEP, 0i32),
        ("Ctrl+Alt+S", 0i32, STEP),
        ("Ctrl+Alt+D", STEP, 0i32),
    ] {
        if let Err(e) = gs.on_shortcut(shortcut, move |app_h, _, event| {
            if event.state() == ShortcutState::Pressed {
                let state = app_h.state::<AppState>();
                if state.mouse_active.load(AtomicOrd::Relaxed) {
                    let _ = state
                        .command_router
                        .dispatch("mouse.move_relative", json!({ "dx": dx, "dy": dy }));
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
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let show_item = MenuItem::with_id(app, "show", "顯示 Keynova (Ctrl+K)", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let handle = app.handle().clone();

    // `default_window_icon()` 在開發環境可能回傳 None（icon 檔不存在時）
    // 使用 if let 避免 unwrap panic；沒有 icon 仍可建立 tray
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

    // 視窗定位：水平置中，垂直約 25% 處（Flow Launcher 風格）
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
    window.on_window_event(move |event| {
        match event {
            tauri::WindowEvent::Focused(true) => {
                let _ = window_focused.emit("window-focused", ());
            }
            // 失焦時自動隱藏（點擊視窗外部即關閉，不需要透明覆蓋層）
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
        }
    });

    Ok(())
}
