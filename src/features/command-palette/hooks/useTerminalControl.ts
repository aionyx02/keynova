// REF.2.P5 — Terminal + Panel exit/close callbacks.
//
// Four small callbacks share the same reset chain (refocus container, clear
// query / cmdResult / results, kick the input back to focus, renew
// launcher_focus_guard). Bundling them keeps the palette body from having
// four near-identical useCallback blocks.
//
// terminalOnExit       — invoked when the persistent terminal panel exits.
// terminalCommandOnExit — invoked when a one-shot terminal command result
//                        finishes (the Suspense-mounted TerminalPanel under
//                        `terminalLaunchSpec`).
// handlePanelCommandResult — passed to every PanelComponent; lets the panel
//                        replace the current cmdResult (e.g. translation
//                        panel commit).
// handlePanelClose     — passed to every panel so Escape inside a textarea
//                        or input can dismiss the panel.

import { useCallback } from "react";

import type { BuiltinCommandResult } from "../../../hooks/useCommands";
import type { UnifiedResult } from "../../../types/unified-result";

interface Deps {
  containerRef: React.RefObject<HTMLDivElement | null>;
  inputRef: React.RefObject<HTMLInputElement | null>;
  setQuery: (q: string) => void;
  setCmdResult: (result: BuiltinCommandResult | null) => void;
  /** REF.6.A — see useFileActions.UseFileActionsDeps.setResults. */
  setResults: React.Dispatch<React.SetStateAction<UnifiedResult[]>>;
  cancelSearch: () => void;
  keepLauncherOpen: () => Promise<void> | void;
}

interface UseTerminalControl {
  terminalOnExit: () => void;
  terminalCommandOnExit: () => void;
  handlePanelCommandResult: (result: BuiltinCommandResult) => void;
  handlePanelClose: () => void;
}

export function useTerminalControl(deps: Deps): UseTerminalControl {
  const {
    containerRef,
    inputRef,
    setQuery,
    setCmdResult,
    setResults,
    cancelSearch,
    keepLauncherOpen,
  } = deps;

  const terminalOnExit = useCallback(() => {
    // Move focus to container first so terminal becoming display:none
    // doesn't shift focus to document.body and risk hiding the window.
    containerRef.current?.focus();
    setQuery("");
    requestAnimationFrame(() => inputRef.current?.focus());
    void keepLauncherOpen();
    // containerRef and inputRef are stable refs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setQuery, keepLauncherOpen]);

  const terminalCommandOnExit = useCallback(() => {
    containerRef.current?.focus();
    setCmdResult(null);
    setQuery("");
    setResults([]);
    requestAnimationFrame(() => inputRef.current?.focus());
    void keepLauncherOpen();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setCmdResult, setQuery, setResults, keepLauncherOpen]);

  const handlePanelCommandResult = useCallback(
    (result: BuiltinCommandResult) => {
      setCmdResult(result);
      setResults([]);
      cancelSearch();
    },
    [setCmdResult, setResults, cancelSearch],
  );

  const handlePanelClose = useCallback(() => {
    setCmdResult(null);
    setQuery("");
    setResults([]);
    requestAnimationFrame(() => inputRef.current?.focus());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setCmdResult, setQuery, setResults]);

  return { terminalOnExit, terminalCommandOnExit, handlePanelCommandResult, handlePanelClose };
}
