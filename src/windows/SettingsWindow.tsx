// Root of the settings window.
//
// Deliberately thin. `SettingPanel` reaches the backend through `invoke`
// directly and `useI18n` reads the browser locale, so none of the launcher's
// provider stack (IPC, feature flags, the feature registry) is needed here —
// the settings webview loads almost nothing.
//
// What this component owns is the two things a window has and a panel did not:
// the theme, which is written onto <html> and belongs to no component, and its
// own closing.

import { useCallback, useEffect } from "react";

import { SettingPanel } from "../features/settings/SettingPanel";
import { ErrorBoundary } from "../shared/components/ErrorBoundary";
import { applyTheme, resolveTheme } from "../shared/theme";
import { closeSettingsWindow } from "./settingsWindowCommands";

export function SettingsWindow() {
  const close = useCallback(() => {
    void closeSettingsWindow().catch(() => {});
  }, []);

  // Escape closes the window. Safe without a confirmation because every row
  // persists on blur or Enter — there is no unsaved state to lose. A non-empty
  // filter swallows the key first (SettingPanel stops it propagating) so the
  // first Escape clears the filter and the second closes.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      close();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [close]);

  const onThemeChange = useCallback((value: string | undefined) => {
    applyTheme(resolveTheme(value));
  }, []);

  return (
    <ErrorBoundary>
      <div className="flex h-full flex-col overflow-hidden">
        <SettingPanel onThemeChange={onThemeChange} />
      </div>
    </ErrorBoundary>
  );
}
