// REF.2.P5 — execCommand wrapper.
//
// Runs the named builtin command via `useCommands().runCommand` and stores
// the result, with one special case: `/onboard` re-triggers the first-run
// tour. It clears the localStorage onboarding flag, blanks the query, and
// opens the OnboardingTour overlay without going through the builtin
// command result UI.

import { useCallback } from "react";

import { resetOnboarding } from "../../../shared/components/onboarding-state";
import type { BuiltinCommandResult } from "../../../hooks/useCommands";

interface Deps {
  runCommand: (name: string, args?: string) => Promise<BuiltinCommandResult>;
  setQuery: (q: string) => void;
  setCmdResult: (result: BuiltinCommandResult | null) => void;
  setOnboardingOpen: React.Dispatch<React.SetStateAction<boolean>>;
}

export function useExecCommand(deps: Deps) {
  const { runCommand, setQuery, setCmdResult, setOnboardingOpen } = deps;

  return useCallback(
    async (name: string, args = "") => {
      try {
        if (name === "onboard") {
          resetOnboarding();
          setQuery("");
          setOnboardingOpen(true);
          return;
        }
        const result = await runCommand(name, args);
        setCmdResult(result);
      } catch {
        // ignore
      }
    },
    [runCommand, setQuery, setCmdResult, setOnboardingOpen],
  );
}
