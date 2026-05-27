// REF.2.P3 — Global ESC key priority chain for the command palette.
// REF.6.B — Inserted a capability-cancel branch between metadata collapse
// and terminal exit so first Esc in capability mode aborts the in-flight
// stream (body shows `Cancelled.`) while the second Esc falls through to
// the `queryRef !== ""` branch and clears the prefix.
//
// Registers a `capture`-phase `keydown` listener once and consults a set of
// refs to decide which reset to run. Refs (not props) are used so the
// listener can read always-current state without re-registering on every
// render — that re-registration window has historically been the seam for
// Bug-A class focus races, so we keep the listener exactly one mount long.
//
// Priority chain (first match wins, all branches preventDefault / stopProp):
//   1. `shouldIgnoreEscape()` truthy  → no-op (e.g. inline editor terminal).
//   2. `secondaryMenuOpenRef`         → close menu via `closeSecondaryMenu`.
//   3. `expandedMetadataRef`          → collapse metadata only.
//   4. capability mode + streaming    → cancel capability stream only.
//   5. `modeRef === "terminal"`       → exit terminal back to launcher input.
//   6. `cmdResultRef !== null`        → unwind command result + pipeline.
//   7. `queryRef !== ""`              → clear typed query only.
//   8. otherwise                      → hide launcher window.

import { useEffect } from "react";

export interface UseEscapeKeyDeps {
  /** Returns true to swallow Escape (e.g. focus is inside an editor terminal). */
  shouldIgnoreEscape: () => boolean;
  modeRef: React.RefObject<string>;
  cmdResultRef: React.RefObject<unknown>;
  queryRef: React.RefObject<string>;
  secondaryMenuOpenRef: React.RefObject<boolean>;
  expandedMetadataRef: React.RefObject<boolean>;
  /** REF.6.B — true when palette is in capability mode (prefix matched). */
  capabilityModeRef: React.RefObject<boolean>;
  /** REF.6.B — true when capability stream is pending/streaming. */
  capabilityStreamingRef: React.RefObject<boolean>;
  /** REF.6.B — cancel callback for the active capability stream. */
  onCapabilityCancel: () => void;
  inputRef: React.RefObject<HTMLInputElement | null>;
  containerRef: React.RefObject<HTMLDivElement | null>;
  closeSecondaryMenu: () => void;
  setExpandedMetadata: (value: boolean) => void;
  setCmdResult: (value: null) => void;
  setQuery: (value: string) => void;
  clearPipeline: () => void;
  clearSearchResults: () => void;
  cancelSearch: () => void;
  hideWindow: () => Promise<void> | void;
  keepLauncherOpen: () => Promise<void> | void;
}

export function useEscapeKey(deps: UseEscapeKeyDeps): void {
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      if (deps.shouldIgnoreEscape()) return;
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();

      if (deps.secondaryMenuOpenRef.current) {
        deps.closeSecondaryMenu();
        return;
      }
      if (deps.expandedMetadataRef.current) {
        deps.setExpandedMetadata(false);
        return;
      }

      // REF.6.B — first Esc in capability mode cancels the in-flight stream
      // and leaves `Cancelled.` body visible. Second Esc falls through to the
      // queryRef branch and clears the prefix.
      if (
        deps.capabilityModeRef.current &&
        deps.capabilityStreamingRef.current
      ) {
        deps.onCapabilityCancel();
        return;
      }

      if (deps.modeRef.current === "terminal") {
        deps.containerRef.current?.focus();
        deps.setQuery("");
        requestAnimationFrame(() => deps.inputRef.current?.focus());
        void deps.keepLauncherOpen();
        return;
      }

      if (deps.cmdResultRef.current !== null) {
        deps.cancelSearch();
        deps.setCmdResult(null);
        deps.clearPipeline();
        deps.setQuery("");
        deps.clearSearchResults();
        requestAnimationFrame(() => deps.inputRef.current?.focus());
      } else if (deps.queryRef.current !== "") {
        deps.cancelSearch();
        deps.setQuery("");
        deps.clearSearchResults();
        deps.clearPipeline();
        deps.inputRef.current?.focus();
      } else {
        void deps.hideWindow();
      }
    }
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
    // Listener captures `deps` by closure; refs + stable callbacks make this
    // safe across renders without re-registering. Mirrors the original
    // CommandPalette effect deps decision.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}
