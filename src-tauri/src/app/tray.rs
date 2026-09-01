use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

const REPO_URL: &str = "https://github.com/aionyx02/keynova";
use crate::app::settings_window::{
    focus_settings_window, launcher_hotkey_target, settings_window_open, LauncherHotkeyTarget,
};
use crate::app::window::show_launcher_window;
pub(crate) fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;

    let show_item = MenuItem::with_id(app, "show", "顯示 Keynova (Ctrl+K)", true, None::<&str>)?;
    let version_label = format!("Keynova v{}", env!("CARGO_PKG_VERSION"));
    let version_item = MenuItem::with_id(app, "version", &version_label, false, None::<&str>)?;
    let about_item = MenuItem::with_id(app, "about", "About Keynova", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &sep1,
            &version_item,
            &about_item,
            &sep2,
            &quit_item,
        ],
    )?;

    let handle = app.handle().clone();

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Keynova")
        .on_menu_event(move |_tray, event| match event.id.as_ref() {
            // This item advertises itself as Ctrl+K, so it obeys the same rule:
            // while settings is open, "show" means show settings.
            "show" => {
                if launcher_hotkey_target(settings_window_open(&handle))
                    == LauncherHotkeyTarget::FocusSettings
                {
                    focus_settings_window(&handle);
                } else if let Some(win) = handle.get_webview_window("main") {
                    let _ = show_launcher_window(&win);
                }
            }
            "about" => {
                if let Err(e) = handle.opener().open_url(REPO_URL, None::<&str>) {
                    eprintln!("[keynova] tray about failed to open url: {e}");
                }
            }
            "quit" => handle.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}
