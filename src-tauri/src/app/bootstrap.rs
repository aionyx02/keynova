use serde_json::Value;
use tauri::{Emitter, Manager};

use crate::app::autostart::sync_autostart;
use crate::app::control_server::start_control_server;
use crate::app::dispatch::{
    cmd_dispatch_impl, cmd_hide_launcher_impl, cmd_keep_launcher_open_impl, cmd_ping_impl,
    cmd_show_launcher_impl,
};
use crate::app::migration::run_legacy_migration;
use crate::app::settings_window::{close_settings_window, open_settings_window};
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

/// Opens (or focuses) the settings window.
///
/// This is an application command rather than a `plugin:core:window|create`
/// call from the webview on purpose: application commands are not ACL-gated,
/// so opening and closing settings needs no capability change. `title` is the
/// localized window title, which only the frontend knows.
#[tauri::command]
fn cmd_open_settings_window(app: tauri::AppHandle, title: Option<String>) -> Result<(), IpcError> {
    open_settings_window(&app, title)
}

/// Closes the settings window. Same reasoning as `cmd_open_settings_window`.
#[tauri::command]
fn cmd_close_settings_window(app: tauri::AppHandle) -> Result<(), IpcError> {
    close_settings_window(&app)
}

#[tauri::command]
fn cmd_keep_launcher_open(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), IpcError> {
    cmd_keep_launcher_open_impl(window, state)
}

pub fn run() {
    // STAB.1 / ADR-0051: install the redacted crash-log panic hook before any
    // other startup work so even a setup-time panic is captured to disk.
    crate::core::crash_log::install_panic_hook(
        crate::platform_dirs::keynova_data_dir().join("crash.log"),
    );

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
                // Drop a second-launch "show" that echoes a Ctrl+K toggle which
                // just hid the window (the close-then-reopen race).
                if app
                    .state::<AppState>()
                    .launcher_toggled_within(std::time::Duration::from_millis(400))
                {
                    return;
                }
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
        // TRAP: this handler is App-level, so it fires for *every* window.
        // Only the launcher may survive its own close — it is a background
        // window that hides instead of exiting. Without the label check, a
        // second window's close button would be swallowed here and would hide
        // the launcher instead of closing anything, leaving a window the user
        // cannot get rid of. Anyone adding a window inherits the correct
        // behaviour by doing nothing.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
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
            cmd_open_settings_window,
            cmd_close_settings_window,
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
