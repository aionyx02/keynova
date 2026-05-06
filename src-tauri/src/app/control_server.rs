use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tauri::Manager;

use crate::app::dispatch::{reload_config_from_disk, schedule_shutdown};
use crate::app::state::AppState;
use crate::app::window::show_launcher;
use crate::core::control_plane::{self, ControlCommand, ControlRequest, ControlResponse};
pub(crate) fn start_control_server(app: &tauri::App) -> bool {
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
            Err(e) => ControlResponse::error(e.message),
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
            Err(e) => ControlResponse::error(e.message),
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
