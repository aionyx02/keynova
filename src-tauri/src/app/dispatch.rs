use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::Manager;

use crate::app::autostart::sync_autostart;
use crate::app::shortcuts::setup_global_shortcuts;
use crate::app::state::AppState;
use crate::app::window::{hide_launcher_window, show_launcher_window};
use crate::core::automation_engine::AutomationEngine;
use crate::core::config_manager::{ConfigChange, ConfigManager};
use crate::core::knowledge_store::WorkflowHistoryEntry;
use crate::core::observability;
use crate::core::workflow_memory;
use crate::core::{ActionLogEntry, AppEvent, IpcError};
use crate::models::action::{Action, ActionKind, ActionRef, ActionResult};
use crate::models::builtin_command::{BuiltinCommandResult, CommandUiType};
use crate::models::settings_schema::is_sensitive_key;
pub(crate) fn cmd_dispatch_impl(
    route: String,
    payload: Option<Value>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Value, IpcError> {
    let payload = payload.unwrap_or(Value::Null);
    let request_metric = observability::measure_json(&payload);
    let action_name = action_name_for_observability(&route, &payload);
    // REF.5 — keep a cheap reference snapshot for the post-success workflow
    // record hook. action.run already has access to the resolved label
    // inside `run_action_command`; the central hook only needs payload for
    // cmd.run / capability.call label extraction.
    let request_payload_for_workflow = if matches!(route.as_str(), "cmd.run" | "capability.call") {
        Some(payload.clone())
    } else {
        None
    };
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
            // REF.5 — record cmd.run + capability.call into workflow_history.
            // action.run is recorded inside `run_action_command` next to the
            // existing `try_log_action` site so the resolved label is at hand.
            if let Some(p) = request_payload_for_workflow.as_ref() {
                maybe_record_central_workflow(&route, p, state.inner(), true);
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
            // ADR-0053: record failures too so ranking can compute a success rate.
            if let Some(p) = request_payload_for_workflow.as_ref() {
                maybe_record_central_workflow(&route, p, state.inner(), false);
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

    if route == "automation.execute_pipeline" {
        return run_pipeline_execute(payload, app, state);
    }

    if let Some(name) = builtin_control_command(route, &payload) {
        return run_builtin_control_command(app, name);
    }

    // Feature-visibility gate: refuse a disabled feature's whole IPC namespace.
    // The AI namespaces (`capability`/`ai`/`agent`) are intentionally absent —
    // they gate per-route in their handlers so model-setup routes
    // (`ai.check_setup`, `capability.list`) stay reachable while AI is off,
    // avoiding a model-configuration bootstrap deadlock.
    if let Some(reason) = namespace_feature_block(route, state)? {
        return Err(IpcError::handler(route, reason));
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

    register_terminal_launch_from_response(&result, state.inner())
        .map_err(|e| IpcError::handler("terminal.register_launch", e))?;

    Ok(result)
}

/// Maps a route to the `features.*` key gating it, or `None` if ungated. The
/// `guards` are `(namespace, flag)` pairs derived from feature specs (DECOUP.4),
/// so the dispatch guard no longer hand-lists features. Matching is on the route
/// namespace segment, so e.g. `system.` does not catch `system_monitoring.`.
/// A missing/empty flag means enabled (repo-wide `unwrap_or(true)` idiom); only
/// an explicit `false` refuses the namespace.
fn route_feature_key<'a>(route: &str, guards: &'a [(&'static str, &'static str)]) -> Option<&'a str> {
    let ns = route.split('.').next()?;
    guards
        .iter()
        .find(|(namespace, _)| *namespace == ns)
        .map(|(_, key)| *key)
}

/// Returns `Some(reason)` when `route` belongs to a disabled feature namespace.
fn namespace_feature_block(
    route: &str,
    state: &tauri::State<'_, AppState>,
) -> Result<Option<String>, IpcError> {
    let Some(key) = route_feature_key(route, &state.feature_namespace_guards) else {
        return Ok(None);
    };
    let enabled = state
        ._config_manager
        .lock()
        .map_err(|e| IpcError::state_lock("config_manager", e.to_string()))?
        .get(key)
        .as_deref()
        .map(|v| !v.eq_ignore_ascii_case("false"))
        .unwrap_or(true);
    Ok((!enabled).then(|| format!("{key} 功能已停用。請前往 /setting → Features 開啟。")))
}

fn register_terminal_launch_from_response(value: &Value, state: &AppState) -> Result<(), String> {
    let Ok(result) = serde_json::from_value::<BuiltinCommandResult>(value.clone()) else {
        return Ok(());
    };
    if let CommandUiType::Terminal(spec) = result.ui_type {
        state
            ._terminal_manager
            .lock()
            .map_err(|e| e.to_string())?
            .register_launch_spec(spec)?;
    }
    Ok(())
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

fn run_pipeline_execute(
    payload: Value,
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<Value, IpcError> {
    use crate::core::workflow_pipeline::parse_pipeline_text;

    let text = payload
        .get("text")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| IpcError::new("invalid_pipeline_request", "missing or empty 'text'"))?;

    let actions = parse_pipeline_text(text)
        .map_err(|e| IpcError::new("pipeline_parse_error", e.to_string()))?;

    let report =
        AutomationEngine::execute_pipeline("pipeline", actions, |route, action_payload| {
            dispatch_command(route, action_payload, app, state).map_err(|e| e.to_string())
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
                        ._app_manager
                        .lock()
                        .map_err(|e| IpcError::handler("launcher.launch_trusted", e.to_string()))?
                        .launch_trusted_path(&path)
                        .map_err(|e| IpcError::handler("launcher.launch_trusted", e))?;
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
            // REF.5 / ADR-0053 — record every action.run attempt with its
            // outcome (was success-only). Captures the resolved human-readable
            // label the central hook in `cmd_dispatch_impl` cannot see.
            let workflow_label = workflow_label_for_action(&action);
            record_workflow_event(
                state.inner(),
                "action.run",
                &workflow_label,
                Some(&payload),
                result.is_ok(),
            );
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
    hide_launcher_window(&window)
}

pub(crate) fn cmd_show_launcher_impl(window: tauri::WebviewWindow) -> Result<(), IpcError> {
    show_launcher_window(&window)
}

pub(crate) fn cmd_keep_launcher_open_impl(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    if let Ok(mut guard) = state.launcher_focus_guard.lock() {
        // 2000ms TTL > backend 1500ms grace — 配合 frontend 在每次 keydown
        // (200ms throttle) renew guard，blur 後 sleep 完檢查時 guard 還有效。
        *guard = Some(Instant::now() + Duration::from_millis(2000));
    }
    show_launcher_window(&window)
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

    if changes
        .iter()
        .any(|change| change.key == "search.backend" || change.key == "search.index_dir")
    {
        let state = app.state::<AppState>();
        let search_manager = Arc::clone(&state._search_manager);
        let (configured_backend, configured_index_dir) = state
            ._config_manager
            .lock()
            .ok()
            .map(|cfg| (cfg.get("search.backend"), cfg.get("search.index_dir")))
            .unwrap_or((None, None));
        if let Ok(mut manager) = search_manager.lock() {
            manager.set_configured_backend(
                configured_backend.as_deref(),
                configured_index_dir.as_deref(),
            );
        };
    }

    if changes
        .iter()
        .any(|change| change.key == "launcher.auto_start_on_login")
    {
        sync_autostart(app).map_err(|e| IpcError::handler("autostart.sync", e))?;
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

// REF.5 — workflow_history record helpers.
//
// `record_workflow_event` is the single fire-and-forget entry point. The
// workspace lock is best-effort: if it can't be acquired, the entry is
// recorded without a `context_hash` so the critical IPC path is never
// blocked by workflow bookkeeping.
fn record_workflow_event(
    state: &AppState,
    route: &str,
    action_label: &str,
    payload: Option<&Value>,
    succeeded: bool,
) {
    let (context_hash, workspace_id, project_root) = match state._workspace_manager.lock() {
        Ok(workspace) => {
            let current = workspace.current();
            let hash = workflow_memory::compute_context_hash(
                current.id as i64,
                &current.mode,
                current.panel.as_deref(),
            );
            (
                Some(hash),
                Some(current.id as i64),
                current.project_root.clone(),
            )
        }
        Err(_) => (None, None, None),
    };
    let payload_digest = payload.map(workflow_memory::digest_payload);
    workflow_memory::record(
        &state.knowledge_store,
        WorkflowHistoryEntry {
            context_hash,
            route: route.to_string(),
            action_label: action_label.to_string(),
            payload_digest,
            workspace_id,
            // ADR-0053: record the outcome so ranking can use success rate.
            succeeded: Some(succeeded),
            // ADR-0054: tag with the project root for project-keyed profiles.
            project_root,
        },
    );
}

// REF.5 — central record hook for cmd.run + capability.call. action.run is
// handled by its own call site in `run_action_command` so the resolved
// human-readable label is available.
fn maybe_record_central_workflow(route: &str, payload: &Value, state: &AppState, succeeded: bool) {
    if route == "capability.call"
        && payload
            .get("id")
            .and_then(Value::as_str)
            .is_some_and(|id| id == "suggest_next")
    {
        return;
    }
    let label = match route {
        "cmd.run" => workflow_label_for_cmd_payload(payload),
        "capability.call" => workflow_label_for_capability_payload(payload),
        _ => return,
    };
    record_workflow_event(state, route, &label, Some(payload), succeeded);
}

fn workflow_label_for_cmd_payload(payload: &Value) -> String {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("cmd");
    let args = payload
        .get("args")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if name == "setting" {
        return workflow_label_for_setting_cmd_args(args);
    }
    match args {
        Some(args) => format!("/{name} {args}"),
        None => format!("/{name}"),
    }
}

fn workflow_label_for_setting_cmd_args(args: Option<&str>) -> String {
    let Some(args) = args else {
        return "/setting".into();
    };
    let Some(split_idx) = args
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace())
        .map(|(idx, _)| idx)
    else {
        return format!("/setting {args}");
    };
    let key = args[..split_idx].trim();
    let value = args[split_idx..].trim();
    if is_sensitive_key(key) && !value.is_empty() {
        format!("/setting {key} [redacted]")
    } else {
        format!("/setting {args}")
    }
}

fn workflow_label_for_capability_payload(payload: &Value) -> String {
    let id = payload
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("capability");
    let body = payload
        .get("payload")
        .and_then(capability_payload_excerpt)
        .unwrap_or_default();
    if body.is_empty() {
        id.to_string()
    } else {
        format!("{id} {body}")
    }
}

fn capability_payload_excerpt(payload: &Value) -> Option<String> {
    let raw = ["text", "intent", "raw_output", "question"]
        .into_iter()
        .find_map(|key| payload.get(key).and_then(Value::as_str))?;
    let single_line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    Some(truncate_workflow_label(single_line, 48))
}

fn workflow_label_for_action(action: &Action) -> String {
    match &action.kind {
        ActionKind::LaunchPath { path } => {
            let path = path.replace('\\', "/");
            let tail = path.rsplit('/').next().unwrap_or(path.as_str());
            format!("Open {tail}")
        }
        ActionKind::OpenPanel { panel, .. } => format!("Open {panel}"),
        ActionKind::CommandRoute { route, payload } if route == "cmd.run" => {
            workflow_label_for_cmd_payload(payload)
        }
        ActionKind::CommandRoute { .. } => truncate_workflow_label(action.label.clone(), 48),
        ActionKind::Inline { text } => truncate_workflow_label(text.trim().to_string(), 48),
        ActionKind::Noop { reason } => truncate_workflow_label(reason.trim().to_string(), 48),
    }
}

fn truncate_workflow_label(value: String, max_chars: usize) -> String {
    let mut iter = value.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Mirrors the `(namespace, flag)` pairs the feature specs derive at assembly
    // (DECOUP.4); the live guards come from `AppState::feature_namespace_guards`.
    const TEST_GUARDS: &[(&str, &str)] = &[
        ("calculator", "features.calculator"),
        ("translation", "features.translation"),
        ("system", "features.system"),
        ("note", "features.notes"),
        ("history", "features.history"),
    ];

    #[test]
    fn route_feature_key_gates_feature_namespaces() {
        assert_eq!(route_feature_key("note.save", TEST_GUARDS), Some("features.notes"));
        assert_eq!(
            route_feature_key("history.list", TEST_GUARDS),
            Some("features.history")
        );
        assert_eq!(
            route_feature_key("translation.translate", TEST_GUARDS),
            Some("features.translation")
        );
        assert_eq!(
            route_feature_key("calculator.eval", TEST_GUARDS),
            Some("features.calculator")
        );
        assert_eq!(
            route_feature_key("system.shutdown", TEST_GUARDS),
            Some("features.system")
        );
    }

    #[test]
    fn route_feature_key_leaves_ai_and_core_namespaces_ungated() {
        // AI namespaces gate per-route in their handlers (so `ai.check_setup`
        // stays reachable while AI is off); the dispatch guard must not touch them.
        assert_eq!(route_feature_key("capability.call", TEST_GUARDS), None);
        assert_eq!(route_feature_key("ai.check_setup", TEST_GUARDS), None);
        assert_eq!(route_feature_key("agent.start", TEST_GUARDS), None);
        // Core namespaces are never feature-gated.
        assert_eq!(route_feature_key("setting.set", TEST_GUARDS), None);
        assert_eq!(route_feature_key("search.query", TEST_GUARDS), None);
        assert_eq!(route_feature_key("model.list", TEST_GUARDS), None);
        // `system_monitoring.*` must not be caught by the `system` namespace.
        assert_eq!(route_feature_key("system_monitoring.snapshot", TEST_GUARDS), None);
    }

    #[test]
    fn workflow_label_for_cmd_payload_keeps_args() {
        let payload = json!({ "name": "setting", "args": "launcher.opacity 0.9" });
        assert_eq!(
            workflow_label_for_cmd_payload(&payload),
            "/setting launcher.opacity 0.9"
        );
    }

    #[test]
    fn workflow_label_for_cmd_payload_redacts_sensitive_setting_values() {
        let payload = json!({ "name": "setting", "args": "translation.api_key secret-value" });
        assert_eq!(
            workflow_label_for_cmd_payload(&payload),
            "/setting translation.api_key [redacted]"
        );
    }

    #[test]
    fn workflow_label_for_capability_payload_includes_excerpt() {
        let payload = json!({
            "id": "explain",
            "payload": { "text": "rust hashmap remove with ownership rules" }
        });
        assert_eq!(
            workflow_label_for_capability_payload(&payload),
            "explain rust hashmap remove with ownership rules"
        );
    }

    #[test]
    fn workflow_label_for_action_uses_launch_target_name() {
        let action = Action::launch_path("C:/work/keynova/src-tauri/src/main.rs");
        assert_eq!(workflow_label_for_action(&action), "Open main.rs");
    }

    #[test]
    fn workflow_label_for_non_cmd_command_route_uses_action_label() {
        let action = Action::command_route(
            "projectcmd:npm run test",
            "Copy npm run test",
            "search.record_selection",
            json!({ "source": "command", "path": "projectcmd://npm run test" }),
        );
        assert_eq!(workflow_label_for_action(&action), "Copy npm run test");
    }
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
