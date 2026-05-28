// REF.2.P5 — Misc small effects that orchestrate other hooks.
//
// Five short effects that have nothing else to live next to:
//   1. one-shot focus on mount
//   2. cleanup the rank-tooltip hover timer on unmount
//   3. refocus the input whenever mode is not "terminal"
//   4. trigger window resize on any palette state change (driven by the
//      many shape-affecting state pieces listed in deps)
//   5. paletteWidth ref sync + window resize when the preview pane shows
//      or hides
//
// Bundled because individually they each add a 4-line `useEffect` block to
// the palette body without earning their keep. Keeping the comment together
// also makes the resize-trigger dep list discoverable.

import { useEffect } from "react";

import { PALETTE_WIDTH_NARROW, PALETTE_WIDTH_WIDE } from "../../../hooks/useWindowResize";
import type { SourceFilter } from "../../../types/search";
import type { BuiltinCommandResult } from "../../../hooks/useCommands";
import type { SecondaryActionId } from "../../../utils/secondaryActions";
import type { SecondaryInlineInput } from "./useSecondaryMenu";

interface Deps {
  inputRef: React.RefObject<HTMLInputElement | null>;
  hoverTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>;
  paletteWidthRef: React.RefObject<number>;
  showPreview: boolean;
  mode: "search" | "command" | "terminal";
  scheduleWindowResize: () => void;
  scheduleWindowPosition: (widthOverride?: number) => void;
  // Resize-trigger deps (every state that changes the palette's height/width
  // contributes here).
  query: string;
  resultsLength: number;
  activeFilters: Set<SourceFilter>;
  cmdSuggestionsLength: number;
  cmdResult: BuiltinCommandResult | null;
  argSuggestionsLength: number;
  expandedMetadata: boolean;
  secondaryMenuOpen: boolean;
  menuFocusedIndex: number;
  pendingConfirm: SecondaryActionId | null;
  inlineInput: SecondaryInlineInput | null;
}

export function usePaletteEffects(deps: Deps) {
  const {
    inputRef,
    hoverTimerRef,
    paletteWidthRef,
    showPreview,
    mode,
    scheduleWindowResize,
    scheduleWindowPosition,
  } = deps;

  // One-shot focus on mount.
  useEffect(() => {
    inputRef.current?.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Cleanup the rank-tooltip hover timer on unmount.
  useEffect(() => {
    const timer = hoverTimerRef;
    return () => {
      if (timer.current) {
        clearTimeout(timer.current);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Refocus the input whenever mode leaves "terminal".
  useEffect(() => {
    if (mode !== "terminal") {
      const raf = requestAnimationFrame(() => {
        inputRef.current?.focus();
      });
      return () => cancelAnimationFrame(raf);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

  // Resize the palette window on any shape-affecting state change.
  useEffect(() => {
    scheduleWindowResize();
  }, [
    scheduleWindowResize,
    deps.mode,
    deps.query,
    deps.resultsLength,
    deps.activeFilters,
    deps.cmdSuggestionsLength,
    deps.cmdResult,
    deps.argSuggestionsLength,
    deps.expandedMetadata,
    deps.secondaryMenuOpen,
    deps.menuFocusedIndex,
    deps.pendingConfirm,
    deps.inlineInput,
  ]);

  // LAUNCH.1.C — preview pane width sync.
  useEffect(() => {
    paletteWidthRef.current = showPreview ? PALETTE_WIDTH_WIDE : PALETTE_WIDTH_NARROW;
    scheduleWindowResize();
    scheduleWindowPosition(paletteWidthRef.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [showPreview, scheduleWindowResize, scheduleWindowPosition]);
}
