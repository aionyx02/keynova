use serde_json::Value;
use tauri::{Emitter, Manager};

use crate::app::autostart::sync_autostart;
use crate::app::control_server::start_control_server;
use crate::app::dispatch::{
    cmd_dispatch_impl, cmd_hide_launcher_impl, cmd_keep_launcher_open_impl, cmd_ping_impl,
    cmd_show_launcher_impl,
};
use crate::app::migration::run_legacy_migration;
use crate::app::shortcuts::setup_global_shortcuts;
use crate::app::state::AppState;
use crate::app::tray::setup_tray;
use crate::app::watchers::{
    prescan_apps, setup_config_watcher, start_clipboard_watcher, start_file_index,
};
use crate::app::window::{hide_launcher_window, setup_main_window, show_launcher};
use crate::core::observability;
use crate::core::{AppEvent, IpcError};

#[tauri::command]
fn cmd_dispatch(
    route: String,
    payload: Option<Value>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Value, IpcError> {
    cmd_dispatch_impl(route, payload, app, state)
}

#[tauri::command]
fn cmd_ping(name: Option<String>, state: tauri::State<'_, AppState>) -> Result<String, IpcError> {
    cmd_ping_impl(name, state)
}

#[tauri::command]
fn cmd_hide_launcher(window: tauri::WebviewWindow) -> Result<(), IpcError> {
    cmd_hide_launcher_impl(window)
}

#[tauri::command]
fn cmd_show_launcher(window: tauri::WebviewWindow) -> Result<(), IpcError> {
    cmd_show_launcher_impl(window)
}

#[tauri::command]
fn cmd_keep_launcher_open(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    cmd_keep_launcher_open_impl(window, state)
}

pub fn run() {
    let context = tauri::generate_context!();
    // In-app updater (PRODUCT.3 / ADR-0050). The plugin panics at init when
    // `plugins.updater` is absent, so register it only once the developer adds
    // that config (endpoints + pubkey). Until then the `/update` command reports
    // "not configured" and `tauri dev`/release stay unaffected.
    let updater_configured = context.config().plugins.0.contains_key("updater");

    let mut builder = tauri::Builder::default();
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        builder = builder
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                eprintln!("[keynova] single_instance: second launch -> show");
                let _ = show_launcher(app);
            }))
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None::<Vec<&'static str>>,
            ));
        if updater_configured {
            builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
        }
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::new())
        .setup(|app| {
            run_legacy_migration();

            if !start_control_server(app) {
                return Ok(());
            }

            if let Err(error) = sync_autostart(app.handle()) {
                eprintln!("[keynova] autostart sync failed: {error}");
            }

            // Bridge EventBus (Rust broadcast) to Tauri frontend events.
            let mut rx = app.state::<AppState>().event_bus.subscribe();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                use tokio::sync::broadcast::error::RecvError;
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            emit_app_event_with_legacy_alias(&handle, &event);
                        }
                        Err(RecvError::Lagged(_)) => continue,
                        Err(RecvError::Closed) => break,
                    }
                }
            });

            app.state::<AppState>()._startup_preflight.ensure_started();
            prescan_apps(app);

            let low_memory = app
                .state::<AppState>()
                ._config_manager
                .lock()
                .ok()
                .and_then(|c| c.get_bool("performance.low_memory_mode"))
                .unwrap_or(false);

            start_file_index(app, low_memory);
            observability::spawn_idle_baseline_probe();
            start_clipboard_watcher(app);
            setup_global_shortcuts(app.handle(), false);
            setup_config_watcher(app);
            setup_tray(app)?;
            setup_main_window(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Some(main) = window.app_handle().get_webview_window("main") {
                    let _ = hide_launcher_window(&main);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            cmd_ping,
            cmd_dispatch,
            cmd_hide_launcher,
            cmd_show_launcher,
            cmd_keep_launcher_open,
        ])
        .run(context)
        .expect("error while running tauri application");
}

fn emit_app_event_with_legacy_alias(handle: &tauri::AppHandle, event: &AppEvent) {
    let _ = handle.emit(&event.topic, &event.payload);
    let legacy_topic = event.legacy_tauri_topic();
    if legacy_topic != event.topic {
        let _ = handle.emit(&legacy_topic, &event.payload);
    }
}
