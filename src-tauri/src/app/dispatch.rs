use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::Manager;

use crate::app::shortcuts::setup_global_shortcuts;
use crate::app::state::AppState;
use crate::core::automation_engine::AutomationEngine;
use crate::core::config_manager::{ConfigChange, ConfigManager};
use crate::core::observability;
use crate::core::{ActionLogEntry, AppEvent, IpcError};
use crate::models::action::{ActionKind, ActionRef, ActionResult};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
pub(crate) fn cmd_dispatch_impl(
    route: String,
    payload: Option<Value>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Value, IpcError> {
    let payload = payload.unwrap_or(Value::Null);
    let request_metric = observability::measure_json(&payload);
    let action_name = action_name_for_observability(&route, &payload);
    let started = Instant::now();

    let result = dispatch_command(&route, payload, &app, &state);
    let elapsed = started.elapsed();

    match &result {
        Ok(value) => {
            let response_metric = observability::measure_json(value);
            observability::log_ipc_command(
                &route,
                true,
                elapsed,
                request_metric,
                Some(response_metric),
                None,
            );
            if let Some(name) = action_name.as_deref() {
                observability::log_action_execution(name, true, elapsed);
            }
        }
        Err(error) => {
            observability::log_ipc_command(
                &route,
                false,
                elapsed,
                request_metric,
                None,
                Some(&error.code),
            );
            if let Some(name) = action_name.as_deref() {
                observability::log_action_execution(name, false, elapsed);
            }
        }
    }

    result
}

fn dispatch_command(
    route: &str,
    payload: Value,
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<Value, IpcError> {
    if let Some(command) = route.strip_prefix("action.") {
        return run_action_command(command, payload, app, state);
    }

    if route == "automation.execute" {
        return run_automation_execute(payload, app, state);
    }

    if let Some(name) = builtin_control_command(route, &payload) {
        return run_builtin_control_command(app, name);
    }

    let before = should_apply_config_after_dispatch(route, &payload)
        .then(|| {
            state
                ._config_manager
                .lock()
                .map(|cfg| cfg.snapshot())
                .map_err(|e| IpcError::state_lock("config_manager", e.to_string()))
        })
        .transpose()?;

    let result = state
        .command_router
        .dispatch(route, payload)
        .map_err(|e| IpcError::handler(route, e))?;

    if let Some(before) = before {
        let after = state
            ._config_manager
            .lock()
            .map(|cfg| cfg.snapshot())
            .map_err(|e| IpcError::state_lock("config_manager", e.to_string()))?;
        let changes = ConfigManager::diff(&before, &after);
        apply_config_changes(app, changes, "setting.set", false)?;
    }

    Ok(result)
}

fn run_automation_execute(
    payload: Value,
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<Value, IpcError> {
    let source = payload
        .get("source")
        .and_then(Value::as_str)
        .filter(|source| !source.trim().is_empty())
        .ok_or_else(|| IpcError::new("invalid_automation_request", "missing source"))?;
    let workflow = AutomationEngine::parse_toml(source)
        .map_err(|e| IpcError::handler("automation.parse", e))?;
    let report = AutomationEngine::execute(&workflow, |route, payload| {
        dispatch_command(route, payload, app, state).map_err(|error| error.to_string())
    });
    Ok(json!(report))
}

fn run_action_command(
    command: &str,
    payload: Value,
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<Value, IpcError> {
    match command {
        "run" => {
            let action_ref = read_action_ref(&payload)?;
            let action = state
                .action_arena
                .resolve(&action_ref)
                .map_err(|e| IpcError::with_details("stale_action_ref", e, json!(action_ref)))?;
            let started = Instant::now();
            let result: Result<ActionResult, IpcError> = match action.kind.clone() {
                ActionKind::LaunchPath { path } => {
                    state
                        .command_router
                        .dispatch("launcher.launch", json!({ "path": path.clone() }))
                        .map_err(|e| IpcError::handler("launcher.launch", e))?;
                    if let Ok(mut workspace) = state._workspace_manager.lock() {
                        workspace.record_file(path.clone());
                    }
                    Ok(ActionResult::Launched { path })
                }
                ActionKind::CommandRoute { route, payload } => {
                    let value = dispatch_command(&route, payload, app, state)?;
                    Ok(ActionResult::Inline {
                        text: value.to_string(),
                    })
                }
                ActionKind::OpenPanel {
                    panel,
                    initial_args,
                } => Ok(ActionResult::Panel {
                    name: panel,
                    initial_args,
                }),
                ActionKind::Inline { text } => Ok(ActionResult::Inline { text }),
                ActionKind::Noop { reason } => Ok(ActionResult::Noop { reason }),
            };
            let elapsed = started.elapsed();
            let status = if result.is_ok() { "ok" } else { "error" }.to_string();
            state.knowledge_store.try_log_action(ActionLogEntry {
                action_id: action.id.clone(),
                action_label: action.label.clone(),
                status,
                duration_ms: elapsed.as_millis(),
                error: result.as_ref().err().map(ToString::to_string),
            });
            if let Ok(mut workspace) = state._workspace_manager.lock() {
                workspace.record_action(action.id);
            }
            Ok(json!(result?))
        }
        "list_secondary" => {
            let action_ref = read_action_ref(&payload)?;
            let actions = state
                .action_arena
                .list_secondary(&action_ref)
                .map_err(|e| IpcError::with_details("stale_action_ref", e, json!(action_ref)))?;
            let refs = actions
                .into_iter()
                .map(|action| {
                    let action_ref = match action_ref.session_id.clone() {
                        Some(session_id) => state.action_arena.insert(
                            &crate::core::action_registry::ActionSession {
                                session_id,
                                generation: action_ref.generation,
                            },
                            action.clone(),
                        ),
                        None => state.action_arena.insert_stable(action.clone()),
                    }?;
                    Ok(json!({
                        "action_ref": action_ref,
                        "label": action.label,
                        "risk": action.risk,
                    }))
                })
                .collect::<Result<Vec<_>, String>>()
                .map_err(|e| IpcError::handler("action.list_secondary", e))?;
            Ok(json!(refs))
        }
        _ => Err(IpcError::with_details(
            "unknown_action_command",
            format!("unknown action command '{command}'"),
            json!({ "command": command }),
        )),
    }
}

fn read_action_ref(payload: &Value) -> Result<ActionRef, IpcError> {
    let value = payload
        .get("action_ref")
        .cloned()
        .ok_or_else(|| IpcError::new("invalid_action_request", "missing action_ref"))?;
    serde_json::from_value(value).map_err(|e| {
        IpcError::with_details(
            "invalid_action_ref",
            e.to_string(),
            json!({ "expected": "ActionRef" }),
        )
    })
}

pub(crate) fn cmd_ping_impl(
    name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<String, IpcError> {
    let name = name.unwrap_or_else(|| "Developer".to_string());
    let response = state
        .command_router
        .dispatch("system.ping", json!({ "name": name }))
        .map_err(|e| IpcError::handler("system.ping", e))?;
    let message = response
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| IpcError::new("invalid_response", "system.ping missing message"))?
        .to_string();
    let _ = state.event_bus.publish(AppEvent::new(
        "system.ping.completed",
        json!({ "message": message, "registered_handlers": state.command_router.handler_count() }),
    ));
    Ok(message)
}

pub(crate) fn cmd_hide_launcher_impl(window: tauri::WebviewWindow) -> Result<(), IpcError> {
    window
        .hide()
        .map_err(|e| IpcError::tauri_api("window.hide", e.to_string()))
}

pub(crate) fn cmd_show_launcher_impl(window: tauri::WebviewWindow) -> Result<(), IpcError> {
    window
        .show()
        .map_err(|e| IpcError::tauri_api("window.show", e.to_string()))?;
    window
        .set_focus()
        .map_err(|e| IpcError::tauri_api("window.set_focus", e.to_string()))
}

pub(crate) fn cmd_keep_launcher_open_impl(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    if let Ok(mut guard) = state.launcher_focus_guard.lock() {
        *guard = Some(Instant::now() + Duration::from_millis(600));
    }
    window
        .show()
        .map_err(|e| IpcError::tauri_api("window.show", e.to_string()))?;
    window
        .set_focus()
        .map_err(|e| IpcError::tauri_api("window.set_focus", e.to_string()))
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

fn action_name_for_observability(route: &str, payload: &Value) -> Option<String> {
    if route != "cmd.run" {
        return None;
    }
    payload
        .get("name")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn run_builtin_control_command(app: &tauri::AppHandle, name: &str) -> Result<Value, IpcError> {
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
        _ => Err(IpcError::with_details(
            "unknown_command",
            format!("unknown control command '/{name}'"),
            json!({ "name": name }),
        )),
    }
}

pub(crate) fn reload_config_from_disk(
    app: &tauri::AppHandle,
    source: &str,
    emit_on_empty: bool,
) -> Result<Vec<ConfigChange>, IpcError> {
    let state = app.state::<AppState>();
    let changes = match state._config_manager.lock() {
        Ok(mut cfg) => cfg.reload_from_disk().map_err(|e| {
            IpcError::with_details(
                "config_reload_failed",
                e,
                json!({
                    "source": source,
                }),
            )
        }),
        Err(e) => Err(IpcError::state_lock("config_manager", e.to_string())),
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

pub(crate) fn apply_config_changes(
    app: &tauri::AppHandle,
    changes: Vec<ConfigChange>,
    source: &str,
    emit_on_empty: bool,
) -> Result<(), IpcError> {
    if changes
        .iter()
        .any(|change| change.key.starts_with("hotkeys."))
    {
        setup_global_shortcuts(app, true);
    }

    if changes.iter().any(|change| change.key == "search.backend") {
        let state = app.state::<AppState>();
        let search_manager = Arc::clone(&state._search_manager);
        let configured_backend = state
            ._config_manager
            .lock()
            .ok()
            .and_then(|cfg| cfg.get("search.backend"));
        if let Ok(mut manager) = search_manager.lock() {
            manager.set_configured_backend(configured_backend.as_deref());
        };
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

fn publish_config_reload_failed(app: &tauri::AppHandle, source: &str, error: &IpcError) {
    let state = app.state::<AppState>();
    let _ = state.event_bus.publish(AppEvent::new(
        "config.reload_failed",
        json!({
            "source": source,
            "code": error.code.clone(),
            "error": error.message.clone(),
            "details": error.details.clone(),
        }),
    ));
}

pub(crate) fn schedule_shutdown(app: &tauri::AppHandle) {
    let handle = app.clone();
    let knowledge_store = app.state::<AppState>().knowledge_store.clone();
    tauri::async_runtime::spawn(async move {
        let _ = knowledge_store.flush().await;
        tokio::time::sleep(Duration::from_millis(75)).await;
        handle.exit(0);
    });
}
