// execCommand wrapper.
//
// Runs the named builtin command via `useCommands().runCommand` and stores
// the result, with two special cases handled in the frontend: `/onboard`
// re-triggers the first-run tour, and `/update` runs the in-app updater check
// (the backend command needs the AppHandle the handler lacks) and renders the
// outcome as an inline result.

import { useCallback } from "react";

import { resetOnboarding } from "../../../shared/components/onboarding-state";
import { checkForUpdate, formatUpdateMessage } from "../../../shared/updater";
import { useI18n } from "../../../i18n/useI18n";
import type { BuiltinCommandResult } from "../../../hooks/useCommands";

interface Deps {
  runCommand: (name: string, args?: string) => Promise<BuiltinCommandResult>;
  setQuery: (q: string) => void;
  setCmdResult: (result: BuiltinCommandResult | null) => void;
  setOnboardingOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

export function useExecCommand(deps: Deps) {
  const { runCommand, setQuery, setCmdResult, setOnboardingOpen } = deps;
  const updaterStrings = useI18n().updater;

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
        setCmdResult(result);
      } catch {
        // ignore
      }
    },
    [runCommand, setQuery, setCmdResult, setOnboardingOpen, updaterStrings],
  );
}
