// Workspace and window focus lifecycle.
//
// Three Tauri events trigger a palette reset:
//   - `window-focused`     — user brought the launcher back to focus while
//                             not in terminal mode → drop transient state.
//   - `workspace-switched` — user picked a different workspace → adopt its
//                             saved query, reset transient state.
//   - `workspace-cycled`   — Ctrl+Alt+0: same reset as switched but with a
//                             force-cleared query rather than restoring the
//                             target workspace's saved one.
//
// All three end in `inputRef.current.focus()` (immediate for window-focused,
// RAF-deferred for the workspace events so layout settles first).

import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

import { confirmInputReady, markPaletteOpen } from "../devTiming";
import type { WorkspaceState } from "../../../hooks/useWorkspace";

export interface UseWorkspaceLifecycleDeps {
  /** Current input mode, accessed inside event listeners (closure-stable). */
  modeRef: React.RefObject<string>;
  /** Palette input element; receives focus after each reset. */
  inputRef: React.RefObject<HTMLInputElement | null>;
  setQuery: (query: string) => void;
  setCmdResult: (result: null) => void;
  clearSearchResults: () => void;
  cancelSearch: () => void;
  clearRecentlyDeleted: () => void;
}

export function useWorkspaceLifecycle({
  modeRef,
  inputRef,
  setQuery,
  setCmdResult,
  clearSearchResults,
  cancelSearch,
  clearRecentlyDeleted,
}: UseWorkspaceLifecycleDeps): void {
  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlisten = listen<void>("window-focused", () => {
      if (modeRef.current === "terminal") return;
      markPaletteOpen("window-focused");
      cancelSearch();
      setQuery("");
      clearSearchResults();
      setCmdResult(null);
      inputRef.current?.focus();
      confirmInputReady("window-focused", inputRef);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [modeRef, inputRef, setQuery, setCmdResult, clearSearchResults, cancelSearch]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlisten = listen<WorkspaceState>("workspace-switched", (event) => {
      const ws = event.payload;
      markPaletteOpen("workspace-switched");
      setQuery(ws.query ?? "");
      clearSearchResults();
      setCmdResult(null);
      cancelSearch();
      clearRecentlyDeleted();
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        confirmInputReady("workspace-switched", inputRef);
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [inputRef, setQuery, setCmdResult, clearSearchResults, cancelSearch, clearRecentlyDeleted]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlisten = listen<WorkspaceState>("workspace-cycled", () => {
      markPaletteOpen("workspace-cycled");
      setQuery("");
      clearSearchResults();
      setCmdResult(null);
      cancelSearch();
      clearRecentlyDeleted();
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        confirmInputReady("workspace-cycled", inputRef);
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [inputRef, setQuery, setCmdResult, clearSearchResults, cancelSearch, clearRecentlyDeleted]);
}
