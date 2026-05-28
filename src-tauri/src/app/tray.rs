use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

const REPO_URL: &str = "https://github.com/aionyx02/keynova";

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
            "show" => {
                if let Some(win) = handle.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
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
