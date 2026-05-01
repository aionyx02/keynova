mod core;
mod handlers;
mod managers;
mod models;
mod platform;

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![cmd_ping, cmd_dispatch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
