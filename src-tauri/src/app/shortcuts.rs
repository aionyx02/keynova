use std::sync::atomic::Ordering as AtomicOrd;

use serde_json::json;
use tauri::{Emitter, Manager};

use crate::app::state::AppState;
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
