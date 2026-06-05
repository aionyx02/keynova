// Derived render-time view of the palette.
//
// Centralises everything the JSX reads but the palette never stores directly:
//   - `visibleResults` filter chain (filter chips + recently deleted kill set)
//   - `safeSelected` read-side clamp (so a stale `selected` after the list
//     shrinks via filter toggle never indexes past the array)
//   - `hasResults` / `hasCmdSuggestions` / `hasArgSuggestions` render gates
//   - `selectedResult` + `selectedMetadata`
//   - `showPreview` decision + previewForSelected + previewLoading
//   - `menuItems` for the secondary action menu
//   - the small "close menu / collapse metadata on selection change" effect
//
// Two side effects live here because they are reads-only consumers of the
// derived values and folding them into the parent component just leaves
// `useEffect(...)` calls dangling next to the JSX. Keeping them next to the
// derivation is more legible.

import { useEffect, useMemo } from "react";

import { useFilePreview, isPreviewable } from "../../../hooks/useFilePreview";
import { useI18n } from "../../../i18n/useI18n";
import { buildSecondaryActions, type SecondaryActionItem } from "../../../utils/secondaryActions";
import type {
  FilePreviewResult,
  SearchMetadata,
  SearchResult,
  SourceFilter,
} from "../../../types/search";

interface Deps {
  results: SearchResult[];
  selected: number;
  mode: "search" | "command" | "terminal";
  activeFilters: Set<SourceFilter>;
  isPathRecentlyDeleted: (path: string, now?: number) => boolean;
  // Command-mode derived inputs (for hasCmdSuggestions / hasArgSuggestions gates).
  cmdSuggestionsCount: number;
  cmdResult: unknown;
  isArgsPhase: boolean;
  argSuggestionsCount: number;
  // Preview gating.
  previewEnabled: boolean;
  metadataByPath: Record<string, SearchMetadata>;
  // Secondary menu auto-close inputs.
  secondaryMenuOpenRef: React.RefObject<boolean>;
  expandedMetadataRef: React.RefObject<boolean>;
  closeSecondaryMenu: () => void;
  setExpandedMetadata: React.Dispatch<React.SetStateAction<boolean>>;
}

interface UseDerivedView {
  visibleResults: SearchResult[];
  safeSelected: number;
  hasResults: boolean;
  hasCmdSuggestions: boolean;
  hasArgSuggestions: boolean;
  selectedResult: SearchResult | null;
  selectedMetadata: SearchMetadata | null | undefined;
  showPreview: boolean;
  previewForSelected: FilePreviewResult | undefined;
  previewLoading: boolean;
  menuItems: SecondaryActionItem[];
}

export function useDerivedView(deps: Deps): UseDerivedView {
  const {
    results,
    selected,
    mode,
    activeFilters,
    isPathRecentlyDeleted,
    cmdSuggestionsCount,
    cmdResult,
    isArgsPhase,
    argSuggestionsCount,
    previewEnabled,
    metadataByPath,
    secondaryMenuOpenRef,
    expandedMetadataRef,
    closeSecondaryMenu,
    setExpandedMetadata,
  } = deps;

  // Derived view: chip filter plus the 30 s recently deleted kill set
  // recently-deleted kill set (the only authoritative gate against Recycle
  // Bin entries reappearing from Everything's index).
  const visibleResults: SearchResult[] = results.filter((r) => {
    if (isPathRecentlyDeleted(r.path)) return false;
    if (activeFilters.size === 0) return true;
    if (r.kind === "folder") return activeFilters.has("file");
    return activeFilters.has(r.kind as SourceFilter);
  });

  // Read-side clamp so a stale `selected` after a filter shrink never
  // indexes past the array.
  const safeSelected =
    visibleResults.length === 0 ? 0 : Math.min(selected, visibleResults.length - 1);

  const hasResults = visibleResults.length > 0 && mode === "search";
  const hasCmdSuggestions =
    cmdSuggestionsCount > 0 && mode === "command" && !cmdResult && !isArgsPhase;
  const hasArgSuggestions = isArgsPhase && argSuggestionsCount > 0 && !cmdResult;

  const selectedResult = visibleResults[safeSelected] ?? null;
  const selectedMetadata = selectedResult ? metadataByPath[selectedResult.path] : null;

  // Preview pane only renders for previewable kinds. The
  // palette window expansion to PALETTE_WIDTH_WIDE is driven by an external
  // effect that reads `showPreview` (see CommandPalette).
  const showPreview =
    previewEnabled &&
    !!selectedResult &&
    (selectedResult.kind === "file" ||
      selectedResult.kind === "folder" ||
      selectedResult.kind === "note") &&
    mode === "search" &&
    visibleResults.length > 0;

  const { previewByPath } = useFilePreview(visibleResults, safeSelected);
  const previewForSelected = selectedResult ? previewByPath[selectedResult.path] : undefined;
  const previewLoading = isPreviewable(selectedResult) && !previewForSelected;

  const actionLabels = useI18n().palette.actions;
  const menuItems = useMemo(
    () => (selectedResult ? buildSecondaryActions(selectedResult, actionLabels) : []),
    [selectedResult, actionLabels],
  );

  // Close secondary menu / collapse metadata when result selection or list
  // changes. Read via refs so this stays a one-line effect.
  useEffect(() => {
    if (secondaryMenuOpenRef.current) {
      closeSecondaryMenu();
    }
    if (expandedMetadataRef.current) {
      setExpandedMetadata(false);
    }
  }, [
    results,
    selected,
    closeSecondaryMenu,
    setExpandedMetadata,
    secondaryMenuOpenRef,
    expandedMetadataRef,
  ]);

  return {
    visibleResults,
    safeSelected,
    hasResults,
    hasCmdSuggestions,
    hasArgSuggestions,
    selectedResult,
    selectedMetadata,
    showPreview,
    previewForSelected,
    previewLoading,
    menuItems,
  };
}
