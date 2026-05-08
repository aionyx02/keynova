use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::Manager;

use crate::app::dispatch::reload_config_from_disk;
use crate::app::state::AppState;
use crate::core::AppEvent;
pub(crate) fn setup_config_watcher(app: &tauri::App) {
    let handle = app.handle().clone();
    let state = app.state::<AppState>();
    let config_path = match state._config_manager.lock() {
        Ok(cfg) => cfg.config_path(),
        Err(e) => {
            eprintln!("[keynova] config watcher skipped: {e}");
            return;
        }
    };

    match start_config_file_watcher(handle, config_path) {
        Ok(watcher) => {
            if let Ok(mut slot) = state._config_watcher.lock() {
                *slot = Some(watcher);
            }
        }
        Err(e) => eprintln!("[keynova] config watcher disabled: {e}"),
    }
}

fn start_config_file_watcher(
    app: tauri::AppHandle,
    config_path: std::path::PathBuf,
) -> Result<notify::RecommendedWatcher, String> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

    let watch_dir = config_path
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| "invalid config path".to_string())?;
    std::fs::create_dir_all(&watch_dir).map_err(|e| e.to_string())?;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |event| {
            let _ = tx.send(event);
        },
        Config::default(),
    )
    .map_err(|e| e.to_string())?;
    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        let mut last_reload = Instant::now() - Duration::from_secs(1);
        for event in rx {
            let Ok(event) = event else {
                continue;
            };
            if !is_config_file_event(&event, &config_path) {
                continue;
            }
            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                continue;
            }
            if last_reload.elapsed() < Duration::from_millis(250) {
                continue;
            }
            last_reload = Instant::now();
            std::thread::sleep(Duration::from_millis(150));
            if let Err(e) = reload_config_from_disk(&app, "watcher", false) {
                eprintln!("[keynova] config watcher reload failed: {e}");
            }
        }
    });

    Ok(watcher)
}

fn is_config_file_event(event: &notify::Event, config_path: &std::path::Path) -> bool {
    let Some(wanted_name) = config_path.file_name() else {
        return false;
    };
    event
        .paths
        .iter()
        .any(|path| path == config_path || path.file_name().is_some_and(|name| name == wanted_name))
}

/// 背景執行緒監聽剪貼簿變化（Windows: 輪詢每 800ms）。
pub(crate) fn start_clipboard_watcher(app: &tauri::App) {
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        clipboard_poll_loop(&handle);
    });
}

fn clipboard_poll_loop(app: &tauri::AppHandle) {
    let mut last_sequence = platform_clipboard_sequence_number();
    loop {
        std::thread::sleep(Duration::from_millis(800));
        let sequence = platform_clipboard_sequence_number();
        if sequence == last_sequence {
            continue;
        }
        last_sequence = sequence;
        if let Some(text) = read_clipboard_text() {
            let state = app.state::<AppState>();
            let workspace_id = state
                ._workspace_manager
                .lock()
                .ok()
                .map(|workspace| workspace.current().id);
            let added = state
                ._history_manager
                .lock()
                .map(|mut mgr| mgr.push_text_for_workspace(text, workspace_id))
                .unwrap_or(false);
            if added {
                let _ = state
                    .event_bus
                    .publish(AppEvent::new("history.updated", json!({})));
            }
        }
    }
}

fn read_clipboard_text() -> Option<String> {
    platform_read_clipboard()
}

#[cfg(target_os = "windows")]
fn platform_clipboard_sequence_number() -> u32 {
    crate::platform::windows::clipboard_sequence_number()
}

#[cfg(not(target_os = "windows"))]
fn platform_clipboard_sequence_number() -> u32 {
    0
}

#[cfg(target_os = "windows")]
fn platform_read_clipboard() -> Option<String> {
    crate::platform::windows::read_clipboard_text()
}

#[cfg(not(target_os = "windows"))]
fn platform_read_clipboard() -> Option<String> {
    None
}

pub(crate) fn prescan_apps(app: &tauri::App) {
    let app_handle = app.handle().clone();
    std::thread::spawn(move || {
        let state = app_handle.state::<AppState>();
        let _ = state
            .command_router
            .dispatch("launcher.list_all", Value::Null);
    });
}

pub(crate) fn start_file_index() {
    #[cfg(target_os = "windows")]
    std::thread::spawn(|| {
        crate::platform::windows::build_file_index();
    });
}
