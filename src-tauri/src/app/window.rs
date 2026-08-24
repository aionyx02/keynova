use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL,
};
#[cfg(target_os = "windows")]
use windows_core_compat::Interface;

use crate::app::state::AppState;
use crate::core::IpcError;
const LAUNCHER_NARROW_WIDTH: f64 = 700.0;
const LAUNCHER_LEFT_SHIFT: i32 = 36;

#[derive(Clone, Copy)]
enum LauncherMemoryLevel {
    Normal,
    Low,
}

pub(crate) fn show_launcher(app: &tauri::AppHandle) -> Result<(), IpcError> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| IpcError::new("window_not_found", "main window not found"))?;
    show_launcher_window(&window)
}

pub(crate) fn show_launcher_window(window: &tauri::WebviewWindow) -> Result<(), IpcError> {
    let _ = window.unminimize();
    set_launcher_memory_level(window, LauncherMemoryLevel::Normal);
    window
        .show()
        .map_err(|e| IpcError::tauri_api("window.show", e.to_string()))?;
    window
        .set_focus()
        .map_err(|e| IpcError::tauri_api("window.set_focus", e.to_string()))
}

pub(crate) fn hide_launcher_window(window: &tauri::WebviewWindow) -> Result<(), IpcError> {
    // An explicit hide (Ctrl+K toggle, Esc, `cmd_hide_launcher`) is a definitive
    // close, so cancel any pending focus-guard. Otherwise a guard renewed by a
    // keystroke ~moments before the hide (2000 ms TTL > the 1500 ms blur grace)
    // stays valid when the blur task wakes and re-shows the window — the
    // "Ctrl+K close then instantly reopen" race. The keep-open path uses `show`,
    // never `hide`, so clearing here never fights a legitimate keep-open.
    if let Some(state) = window.try_state::<AppState>() {
        if let Ok(mut guard) = state.launcher_focus_guard.lock() {
            *guard = None;
        }
    }
    window
        .hide()
        .map_err(|e| IpcError::tauri_api("window.hide", e.to_string()))?;
    set_launcher_memory_level(window, LauncherMemoryLevel::Low);
    trim_host_working_set();
    Ok(())
}

/// Push the host process's resident pages out of the working set while the
/// launcher is hidden. This trims the `tauri-app.exe` working set that the
/// WebView2 `SetMemoryUsageTargetLevel` call cannot reach (that one only
/// targets the webview child processes). Commit / private bytes are unchanged;
/// the pages fault back in on the next wake. Best-effort: failures are logged,
/// not surfaced.
#[cfg(target_os = "windows")]
fn trim_host_working_set() {
    use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
    use windows::Win32::System::Threading::GetCurrentProcess;

    unsafe {
        if let Err(error) = EmptyWorkingSet(GetCurrentProcess()) {
            eprintln!("[keynova] EmptyWorkingSet failed: {error}");
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn trim_host_working_set() {}

pub(crate) fn setup_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window = app
        .get_webview_window("main")
        .ok_or("main window not found")?;

    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen = monitor.size();
        let scale = monitor.scale_factor();
        let phys_w = (LAUNCHER_NARROW_WIDTH * scale) as i32;
        let x = ((screen.width as i32 - phys_w) / 2 - LAUNCHER_LEFT_SHIFT).max(0);
        let y = (screen.height as f64 * 0.25) as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }

    let window_focused = window.clone();
    let window_blur = window.clone();
    let blur_guard = app.state::<AppState>().launcher_focus_guard.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Focused(true) => {
            set_launcher_memory_level(&window_focused, LauncherMemoryLevel::Normal);
            let _ = window_focused.emit("window-focused", ());
        }
        tauri::WindowEvent::Focused(false) => {
            let window_blur = window_blur.clone();
            let blur_guard = Arc::clone(&blur_guard);
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(1500)).await;
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
                    let _ = show_launcher_window(&window_blur);
                    return;
                }
                if window_blur.is_focused().unwrap_or(false) {
                    return;
                }
                let _ = hide_launcher_window(&window_blur);
            });
        }
        _ => {}
    });

    if window.is_visible().unwrap_or(false) {
        set_launcher_memory_level(&window, LauncherMemoryLevel::Normal);
    } else {
        set_launcher_memory_level(&window, LauncherMemoryLevel::Low);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_launcher_memory_level(window: &tauri::WebviewWindow, level: LauncherMemoryLevel) {
    let target_label = match level {
        LauncherMemoryLevel::Normal => "normal",
        LauncherMemoryLevel::Low => "low",
    };
    let result = window.with_webview(move |webview| {
        if let Err(error) = apply_memory_usage_level(webview, level) {
            eprintln!("[keynova] webview memory level {target_label} failed: {error}");
        }
    });
    if let Err(error) = result {
        eprintln!("[keynova] webview memory level {target_label} dispatch failed: {error}");
    }
}

#[cfg(not(target_os = "windows"))]
fn set_launcher_memory_level(_window: &tauri::WebviewWindow, _level: LauncherMemoryLevel) {}

#[cfg(target_os = "windows")]
fn apply_memory_usage_level(
    webview: tauri::webview::PlatformWebview,
    level: LauncherMemoryLevel,
) -> Result<(), String> {
    let webview = unsafe { webview.controller().CoreWebView2() }.map_err(|e| e.to_string())?;
    let webview = webview
        .cast::<ICoreWebView2_19>()
        .map_err(|e| e.to_string())?;
    let level = match level {
        LauncherMemoryLevel::Normal => 0,
        LauncherMemoryLevel::Low => 1,
    };
    let level = COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL(level);
    unsafe { webview.SetMemoryUsageTargetLevel(level) }.map_err(|e| e.to_string())
}
