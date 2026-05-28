use tauri::Manager;

use crate::app::state::AppState;

const AUTOSTART_KEY: &str = "launcher.auto_start_on_login";

pub(crate) fn sync_autostart(app: &tauri::AppHandle) -> Result<(), String> {
    let should_enable = desired_autostart_enabled(app);

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        use tauri_plugin_autostart::ManagerExt;

        let manager = app.autolaunch();
        let is_enabled = manager.is_enabled().map_err(|e| e.to_string())?;
        match (should_enable, is_enabled) {
            (true, false) => manager.enable().map_err(|e| e.to_string())?,
            (false, true) => manager.disable().map_err(|e| e.to_string())?,
            _ => {}
        }
    }

    Ok(())
}

fn desired_autostart_enabled(app: &tauri::AppHandle) -> bool {
    app.state::<AppState>()
        ._config_manager
        .lock()
        .ok()
        .and_then(|cfg| cfg.get(AUTOSTART_KEY))
        .map(|value| !value.eq_ignore_ascii_case("false"))
        .unwrap_or(true)
}
