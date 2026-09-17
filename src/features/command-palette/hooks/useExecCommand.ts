// execCommand wrapper.
//
// Runs the named builtin command via `useCommands().runCommand` and stores
// the result, with two special cases handled in the frontend: `/onboard`
// re-triggers the first-run tour, and `/update` runs the in-app updater check
// (the backend command needs the AppHandle the handler lacks) and renders the
// outcome as an inline result.
//
// A `Window` ui_type is the third case, and the only one that produces no
// palette output at all: the result names an OS window to open instead of
// something to render. `/setting` is the only command that returns one today.

import { useCallback } from "react";

import { resetOnboarding } from "../../../shared/components/onboarding-state";
import { checkForUpdate, formatUpdateMessage } from "../../../shared/updater";
import { useI18n } from "../../../i18n/useI18n";
import { openSettingsWindow } from "../../../windows/settingsWindowCommands";
import type { BuiltinCommandResult } from "../../../hooks/useCommands";

interface Deps {
  runCommand: (name: string, args?: string) => Promise<BuiltinCommandResult>;
  setQuery: (q: string) => void;
  setCmdResult: (result: BuiltinCommandResult | null) => void;
  setOnboardingOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

export function useExecCommand(deps: Deps) {
  const { runCommand, setQuery, setCmdResult, setOnboardingOpen } = deps;
  const strings = useI18n();
  const updaterStrings = strings.updater;
  const settingsWindowTitle = strings.settings.windowTitle;

  return useCallback(
    async (name: string, args = "") => {
      try {
        if (name === "onboard") {
          resetOnboarding();
          setQuery("");
          setOnboardingOpen(true);
          return;
        }
        if (name === "update") {
          const result = await checkForUpdate();
          setCmdResult({
            text: formatUpdateMessage(result, updaterStrings),
            ui_type: { type: "Inline" },
          });
          return;
        }
        const result = await runCommand(name, args);
        if (result.ui_type.type === "Window") {
          // Opening the window hides the launcher (the Rust side does it, so
          // the always-on-top strip is gone before the window appears rather
          // than 1500 ms later). Clearing the query means the palette is back
          // at its empty state the next time Ctrl+K shows it.
          if (result.ui_type.value === "settings") {
            await openSettingsWindow(settingsWindowTitle);
            setQuery("");
            setCmdResult(null);
          }
          return;
        }
        setCmdResult(result);
      } catch {
        // ignore
      }
    },
    [
      runCommand,
      setQuery,
      setCmdResult,
      setOnboardingOpen,
      updaterStrings,
      settingsWindowTitle,
    ],
  );
}
