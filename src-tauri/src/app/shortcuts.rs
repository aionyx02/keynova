use std::time::{Duration, Instant};

use serde_json::json;
use tauri::{Emitter, Manager};

use crate::app::state::AppState;
use crate::app::window::{hide_launcher_window, show_launcher_window};

/// Minimum gap between two honored Ctrl+K toggles. Collapses a double-fired
/// global shortcut (the "close then instantly reopen" race) while staying short
/// enough not to swallow a deliberate open-then-close.
const TOGGLE_DEBOUNCE: Duration = Duration::from_millis(180);
pub(crate) fn setup_global_shortcuts(app: &tauri::AppHandle, reset_existing: bool) {
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

    let candidates: &[&str] = if launcher_key == "Ctrl+K" {
        &["Ctrl+K"]
    } else {
        &[launcher_key.as_str(), "Ctrl+K"]
    };
    'launcher: for &key in candidates {
        let handle_k = app.clone();
        if let Err(e) = gs.on_shortcut(key, move |_app, _, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            // Debounce: a double-fired global shortcut would otherwise hide then
            // immediately show (the "close then instantly reopen" race).
            if let Ok(mut last) = handle_k.state::<AppState>().last_launcher_toggle.lock() {
                let now = Instant::now();
                if last.is_some_and(|prev| now.duration_since(prev) < TOGGLE_DEBOUNCE) {
                    return;
                }
                *last = Some(now);
            }
            if let Some(win) = handle_k.get_webview_window("main") {
                // `is_visible` is the reliable signal on Windows/WebView2 (where
                // `is_focused` blips); a plain visible→hide / hidden→show toggle
                // plus the debounce above is the robust in-app behavior.
                let visible = win.is_visible().unwrap_or(false);
                if visible {
                    let _ = hide_launcher_window(&win);
                } else {
                    let _ = show_launcher_window(&win);
                }
            }
        }) {
            eprintln!("[keynova] {key} registration failed: {e}");
        } else {
            break 'launcher;
        }
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

    // Workspace cycle (Ctrl+Alt+0 default), sitting naturally next to
    // workspace_1/2/3. Cycle
    // emits a distinct `workspace-cycled` event so the frontend can clear the
    // search query (direct 1/2/3 switches preserve per-workspace state).
    let cycle_key = app
        .state::<AppState>()
        ._config_manager
        .lock()
        .map(|c| {
            c.get("hotkeys.workspace_cycle")
                .unwrap_or_else(|| "Ctrl+Alt+0".into())
        })
        .unwrap_or_else(|_| "Ctrl+Alt+0".into());
    let handle_cycle = app.clone();
    if let Err(e) = gs.on_shortcut(cycle_key.as_str(), move |app_h, _, event| {
        if event.state() != ShortcutState::Pressed {
            return;
        }
        let payload = {
            let state = app_h.state::<AppState>();
            state._workspace_manager.lock().ok().and_then(|mut mgr| {
                let next = (mgr.current().id + 1) % mgr.all().len();
                mgr.switch_to(next).ok().map(|ws| json!(ws))
            })
        };
        if let Some(payload) = payload {
            if let Some(win) = handle_cycle.get_webview_window("main") {
                let _ = win.emit("workspace-cycled", &payload);
            }
        }
    }) {
        eprintln!("[keynova] {cycle_key} (workspace_cycle) registration failed: {e}");
    }
}
