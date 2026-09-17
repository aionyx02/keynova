// Frontend side of the settings window.
//
// Opening and closing go through application commands rather than through
// `@tauri-apps/api/window`. That is not a style preference: Tauri's ACL gates
// every `plugin:core:window|*` call per window, and `capabilities/default.json`
// is scoped to `["main"]` — so the settings webview holds no window permissions
// at all. Application commands registered in `invoke_handler` are outside the
// ACL (this app defines no app-level permission manifest), which is what lets
// the settings window open, title, and close itself without touching a
// capability definition. See docs/decisions.md.

import { invoke } from "@tauri-apps/api/core";

/**
 * Opens the settings window, or focuses it if it is already open.
 *
 * The title is passed from here because the localized strings live in the
 * frontend and Rust has no i18n layer.
 */
export function openSettingsWindow(title: string): Promise<void> {
  return invoke("cmd_open_settings_window", { title });
}

/** Destroys the settings window. Safe to call when there is none. */
export function closeSettingsWindow(): Promise<void> {
  return invoke("cmd_close_settings_window");
}
