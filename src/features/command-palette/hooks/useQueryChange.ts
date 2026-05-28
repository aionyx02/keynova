// REF.2.P5 — `<input>` change handler.
//
// Resets all the transient state that should disappear on every keystroke
// (cmdResult, copy/pipeline hints, command/arg suggestion indices, the
// Bug B recently-deleted kill-set), parses the new query, and either fires
// a new debounced search or cancels the in-flight one if the new query is
// not in search mode.
//
// Pulled out of the palette body because it touches eight setters and the
// LAUNCH.1.B "fresh search context clears suppress-list" reasoning belongs
// near the kill-set entry point.

import { useCallback } from "react";

import { parseInputMode } from "../../../hooks/useInputMode";
import type { BuiltinCommandResult } from "../../../hooks/useCommands";
import { parseCapabilityPrefix } from "../utils/parseCapabilityPrefix";

interface Deps {
  setQuery: (q: string) => void;
  setCmdResult: (result: BuiltinCommandResult | null) => void;
  clearCopyHint: () => void;
  clearPipeline: () => void;
  setSelectedCmd: React.Dispatch<React.SetStateAction<number>>;
  setSelectedArg: React.Dispatch<React.SetStateAction<number>>;
  setArgSuggestions: React.Dispatch<React.SetStateAction<string[]>>;
  clearRecentlyDeleted: () => void;
  setTerminalMounted: React.Dispatch<React.SetStateAction<boolean>>;
  clearSearchResults: () => void;
  cancelSearch: () => void;
  triggerSearch: (rawInput: string) => void;
}

export function useQueryChange(deps: Deps) {
  return useCallback(
    (value: string) => {
      deps.setQuery(value);
      deps.setCmdResult(null);
      deps.clearCopyHint();
      deps.clearPipeline();
      deps.setSelectedCmd(0);
      deps.setSelectedArg(0);
      deps.setArgSuggestions([]);
      // LAUNCH.1.B bugfix — typing a new query enters a fresh search
      // context; suppress-list is no longer relevant.
      deps.clearRecentlyDeleted();
      const { mode: newMode, rawInput: ri } = parseInputMode(value);
      // Mount terminal on first "> " entry; avoids useEffect setState cascade.
      if (newMode === "terminal") deps.setTerminalMounted(true);
      // REF.6.B — capability prefix takes over the result area; suppress
      // search backend calls so the prefix body doesn't double-fire as a
      // search query.
      if (newMode === "search" && parseCapabilityPrefix(value)) {
        deps.clearSearchResults();
        deps.cancelSearch();
        return;
      }
      if (newMode !== "search" || ri.trim() === "") {
        deps.clearSearchResults();
        deps.cancelSearch();
        return;
      }
      deps.triggerSearch(ri);
    },
    [deps],
  );
}
