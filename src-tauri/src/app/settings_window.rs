//! The settings window.
//!
//! Settings used to be a palette panel. It is now a real OS window, because it
//! is the one surface in Keynova that is *read*, not *typed through*: the
//! launcher is 700x50 and hides itself the moment it loses focus, which is
//! exactly wrong for a form somebody scrolls, filters, and thinks about.
//!
//! Two consequences drive everything in this module.
//!
//! 1. The window is created on demand and destroyed on close. There is no
//!    hidden instance parked in the background, so an unopened settings window
//!    costs nothing — and `get_webview_window(SETTINGS_WINDOW_LABEL).is_some()`
//!    *is* the "settings is open" state. No `AppState` flag can drift out of
//!    sync with it.
//! 2. The launcher is `alwaysOnTop` and hides on blur (see `app::window`).
//!    Opening settings therefore hides the launcher explicitly rather than
//!    letting the 1500 ms blur timer get there eventually.

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

use crate::app::window::hide_launcher_window;
use crate::core::IpcError;

pub(crate) const SETTINGS_WINDOW_LABEL: &str = "settings";

/// Same `index.html` as the launcher; `main.tsx` branches on the query and
/// dynamic-imports one of two trees, so this webview never loads the palette,
/// xterm, or markdown bundles. Keeping one entry also keeps `vite.config.ts`
/// single-input.
const SETTINGS_URL: &str = "index.html?window=settings";

const SETTINGS_WIDTH: f64 = 880.0;
const SETTINGS_HEIGHT: f64 = 640.0;
const SETTINGS_MIN_WIDTH: f64 = 640.0;
const SETTINGS_MIN_HEIGHT: f64 = 480.0;

const DEFAULT_TITLE: &str = "Keynova Settings";

/// What the launcher hotkey — and the tray's "show" item, which advertises
/// itself as that hotkey — should do when pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LauncherHotkeyTarget {
    /// Settings is open and owns the foreground: bring it forward.
    FocusSettings,
    /// Normal behaviour: show the launcher, or hide it if already visible.
    ToggleLauncher,
}

/// The shared rule behind Ctrl+K and the tray. Opening settings deliberately
/// hides the launcher, so a Ctrl+K that re-showed it would put a 50 px
/// always-on-top strip over the window the user just asked for — the hide would
/// mean nothing.
pub(crate) fn launcher_hotkey_target(settings_window_open: bool) -> LauncherHotkeyTarget {
    if settings_window_open {
        LauncherHotkeyTarget::FocusSettings
    } else {
        LauncherHotkeyTarget::ToggleLauncher
    }
}

/// The window's existence *is* the "settings is open" state — see the module
/// note. Nothing else records it, so nothing else can disagree with it.
pub(crate) fn settings_window_open(app: &tauri::AppHandle) -> bool {
    app.get_webview_window(SETTINGS_WINDOW_LABEL).is_some()
}

/// Brings an existing settings window to the front. `false` means there was
/// none, which is also the caller's cue that the hotkey should fall through to
/// its normal launcher behaviour.
pub(crate) fn focus_settings_window(app: &tauri::AppHandle) -> bool {
    let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) else {
        return false;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
    true
}

/// Opens settings, or focuses it if it is already open.
///
/// **Must not be called from the main thread.** `WebviewWindowBuilder::build()`
/// deadlocks there on Windows; the caller is an `async` Tauri command for that
/// reason alone (see `cmd_open_settings_window`).
///
/// `title` comes from the frontend because the localized strings live there and
/// Rust has no i18n layer. An absent or blank title falls back to the English
/// name rather than shipping an untitled window.
pub(crate) fn open_settings_window(
    app: &tauri::AppHandle,
    title: Option<String>,
) -> Result<(), IpcError> {
    // Before anything else: the launcher must go. It is always-on-top, so it
    // would otherwise sit over the new window until the blur timer fired.
    if let Some(main) = app.get_webview_window("main") {
        let _ = hide_launcher_window(&main);
    }

    // `/setting` is not the only way in — the CLI bin and the control server
    // both reach `cmd_dispatch` — so a second open must focus, never build a
    // second window (which would fail on the duplicate label anyway).
    if focus_settings_window(app) {
        return Ok(());
    }

    let title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| DEFAULT_TITLE.to_string());

    let builder = WebviewWindowBuilder::new(
        app,
        SETTINGS_WINDOW_LABEL,
        WebviewUrl::App(SETTINGS_URL.into()),
    )
    .title(title)
    .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
    .min_inner_size(SETTINGS_MIN_WIDTH, SETTINGS_MIN_HEIGHT)
    // An ordinary window in every way the launcher is not: it has decorations,
    // it resizes, it appears in the taskbar and in Alt-Tab, and it does not
    // float. Size and position are deliberately not remembered — centred every
    // time is predictable, and a remembered position is one more piece of state
    // that can restore a window onto a monitor that is no longer there.
    .center()
    .resizable(true)
    .decorations(true)
    .visible(true)
    .focused(true);

    #[cfg(target_os = "windows")]
    let builder = match main_window_browser_args(app) {
        Some(args) => builder.additional_browser_args(&args),
        None => builder,
    };

    builder
        .build()
        .map_err(|e| IpcError::tauri_api("window.build", e.to_string()))?;

    Ok(())
}

/// TRAP (Windows): WebView2 runs one browser process per user-data folder, and
/// a second webview whose environment options differ from the running one fails
/// to be created at all — the settings window would simply never appear. `main`
/// sets `additionalBrowserArgs` in `tauri.conf.json` because it needs
/// `--disable-gpu`, so this window has to pass byte-identical arguments.
/// Reading them back out of the config rather than repeating the literal is
/// what stops the two from drifting the next time that line is edited.
#[cfg(target_os = "windows")]
fn main_window_browser_args(app: &tauri::AppHandle) -> Option<String> {
    app.config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .and_then(|window| window.additional_browser_args.clone())
}

/// Destroys the settings window. Safe to call when there is none.
///
/// The frontend closes through this Rust command rather than through
/// `getCurrentWindow().close()` because `plugin:core:window|close` is ACL-gated
/// per window, while application commands are not — see the capability note in
/// `docs/decisions.md`.
pub(crate) fn close_settings_window(app: &tauri::AppHandle) -> Result<(), IpcError> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        window
            .destroy()
            .map_err(|e| IpcError::tauri_api("window.destroy", e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_focuses_settings_while_it_is_open() {
        assert_eq!(
            launcher_hotkey_target(true),
            LauncherHotkeyTarget::FocusSettings
        );
    }

    #[test]
    fn hotkey_toggles_launcher_when_settings_is_closed() {
        assert_eq!(
            launcher_hotkey_target(false),
            LauncherHotkeyTarget::ToggleLauncher
        );
    }
}
