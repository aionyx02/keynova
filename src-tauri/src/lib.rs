mod core;

use std::sync::Arc;

use serde_json::{json, Value};

use crate::core::{AppEvent, CommandHandler, CommandResult, CommandRouter, EventBus};

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
        let mut command_router = CommandRouter::new();
        command_router.register(Arc::new(SystemHandler));

        Self {
            command_router,
            event_bus: EventBus::default(),
        }
    }
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

    state
        .event_bus
        .publish(AppEvent::new(
            "system.ping.completed",
            json!({
                "message": message,
                "registered_handlers": state.command_router.handler_count(),
            }),
        ))
        .map_err(|error| error.to_string())?;

    Ok(message)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![cmd_ping])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
