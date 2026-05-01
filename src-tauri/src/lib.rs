mod core;
mod handlers;
mod managers;
mod models;
mod platform;

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tauri::{Emitter, Manager};

use crate::core::{AppEvent, CommandHandler, CommandResult, CommandRouter, EventBus};
use crate::handlers::{
    hotkey::HotkeyHandler, launcher::LauncherHandler, mouse::MouseHandler,
    terminal::TerminalHandler,
};
use crate::managers::{
    app_manager::AppManager, hotkey_manager::HotkeyManager, mouse_manager::MouseManager,
    terminal_manager::TerminalManager,
};

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
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or("Developer");

                Ok(json!({
                    "message": format!("Pong from Rust CommandRouter, {}!", name),
                }))
            }
            _ => Err(format!("unknown system command '{command}'")),
        }
    }
}

struct AppState {
    command_router: CommandRouter,
    event_bus: EventBus,
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

        let mut command_router = CommandRouter::new();
        command_router.register(Arc::new(SystemHandler));
        command_router.register(Arc::new(LauncherHandler::new(Arc::clone(&app_manager))));
        command_router.register(Arc::new(HotkeyHandler::new(Arc::clone(&hotkey_manager))));
        command_router.register(Arc::new(TerminalHandler::new(Arc::clone(
            &terminal_manager,
        ))));
        command_router.register(Arc::new(MouseHandler::new(Arc::clone(&mouse_manager))));

        Self {
            command_router,
            event_bus,
        }
    }
}

/// 通用命令分發入口，路由格式為 "namespace.command"。
#[tauri::command]
fn cmd_dispatch(
    route: String,
    payload: Option<Value>,
    state: tauri::State<'_, AppState>,
) -> Result<Value, String> {
    let payload = payload.unwrap_or(Value::Null);
    state.command_router.dispatch(&route, payload)
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
        .ok_or_else(|| "missing message in command response".to_string())?
        .to_string();

    let _ = state.event_bus.publish(AppEvent::new(
        "system.ping.completed",
        json!({
            "message": message,
            "registered_handlers": state.command_router.handler_count(),
        }),
    ));

    Ok(message)
}

/// 隱藏啟動器視窗（App 繼續背景常駐）。
#[tauri::command]
fn cmd_hide_launcher(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

/// 顯示啟動器視窗並移至最前景。
#[tauri::command]
fn cmd_show_launcher(window: tauri::WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            setup_global_shortcut(app)?;
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_global_shortcut(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let handle = app.handle().clone();
    app.global_shortcut().on_shortcut("Ctrl+K", move |_app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            if let Some(window) = handle.get_webview_window("main") {
                let visible = window.is_visible().unwrap_or(false);
                if visible {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
    })?;

    Ok(())
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let show_item = MenuItem::with_id(app, "show", "顯示 Keynova (Ctrl+K)", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let handle = app.handle().clone();
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Keynova")
        .on_menu_event(move |_tray, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => handle.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn setup_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;

    let window_clone = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(true) = event {
            // Emit a Tauri event so the frontend knows to focus the input
            let _ = window_clone.emit("window-focused", ());
        }
    });

    Ok(())
}
