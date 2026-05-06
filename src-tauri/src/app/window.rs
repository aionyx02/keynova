use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};

use crate::app::state::AppState;
use crate::core::IpcError;
pub(crate) fn show_launcher(app: &tauri::AppHandle) -> Result<(), IpcError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| IpcError::new("window_not_found", "main window not found"))?;
    let _ = window.unminimize();
    window
        .show()
        .map_err(|e| IpcError::tauri_api("window.show", e.to_string()))?;
    window
        .set_focus()
        .map_err(|e| IpcError::tauri_api("window.set_focus", e.to_string()))
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub(crate) fn setup_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
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
