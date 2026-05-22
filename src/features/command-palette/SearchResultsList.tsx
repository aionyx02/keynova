// REF.2.P4 — Main search results card.
//
// Bundles FilterChips, the result <ul> with row-level hover/select callbacks,
// the SecondaryActionMenu overlay anchored to the result column, the optional
// right-side PreviewPane column, the metadata expansion strip toggled from
// the action menu, and the footer hint row.
//
// State stays in CommandPalette; this component is presentational and pipes
// callbacks through. Inline rendering of the result row is kept here (not
// split into another sub-component) because the row-level callbacks share
// the hover-timer ref captured in the parent, and extracting that across two
// files would force a tuple of refs through the prop bundle for no clarity
// gain.

import type React from "react";
import type { ReactNode } from "react";

import { FilterChips } from "./FilterChips";
import { SecondaryActionMenu } from "./SecondaryActionMenu";
import { PreviewPane } from "../../components/PreviewPane";
import type {
  FilePreviewResult,
  SearchIconAsset,
  SearchMetadata,
  SearchResult,
  SourceFilter,
} from "../../types/search";
import type { SecondaryActionItem, SecondaryActionId } from "../../utils/secondaryActions";
import type { SecondaryInlineInput } from "./hooks/useSecondaryMenu";

const KIND_BADGE: Record<string, { label: string; cls: string }> = {
  app: { label: "App", cls: "bg-violet-500/30 text-violet-300" },
  file: { label: "File", cls: "bg-sky-500/30 text-sky-300" },
  folder: { label: "Dir", cls: "bg-amber-500/30 text-amber-300" },
  command: { label: "Cmd", cls: "bg-emerald-500/30 text-emerald-300" },
  note: { label: "Note", cls: "bg-teal-500/30 text-teal-300" },
  history: { label: "Hist", cls: "bg-zinc-500/30 text-zinc-300" },
  model: { label: "AI", cls: "bg-fuchsia-500/30 text-fuchsia-300" },
};

function hasEncodingError(s: string | undefined | null): boolean {
  return typeof s === "string" && s.includes("�");
}

interface Props {
  visibleResults: SearchResult[];
  safeSelected: number;
  iconsByKey: Record<string, SearchIconAsset>;
  onSelectIndex: (index: number) => void;
  onLaunch: (result: SearchResult) => void;
  // Hover for rank tooltip (LAUNCH.1.E).
  showRankBreakdown: boolean;
  onHoverStart: (index: number, rect: DOMRect) => void;
  onHoverEnd: () => void;
  activeFilters: Set<SourceFilter>;
  onChangeFilters: (next: Set<SourceFilter>) => void;
  // Secondary action menu overlay (LAUNCH.1.A).
  secondaryMenuOpen: boolean;
  selectedResult: SearchResult | null;
  menuItems: SecondaryActionItem[];
  menuFocusedIndex: number;
  pendingConfirm: SecondaryActionId | null;
  inlineInput: SecondaryInlineInput | null;
  onSecondaryAction: (id: SecondaryActionId, result: SearchResult) => void;
  onMenuFocus: (index: number) => void;
  onInlineInputChange: (value: string) => void;
  onInlineInputKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  // Preview pane (LAUNCH.1.C).
  showPreview: boolean;
  previewForSelected: FilePreviewResult | undefined;
  previewLoading: boolean;
  // Metadata expansion (LAUNCH.1.A).
  expandedMetadata: boolean;
  selectedMetadata: SearchMetadata | null | undefined;
  // Footer hint text (copy/preview/diagnostic).
  footerHint: ReactNode;
}

export function SearchResultsList({
  visibleResults,
  safeSelected,
  iconsByKey,
  onSelectIndex,
  onLaunch,
  showRankBreakdown,
  onHoverStart,
  onHoverEnd,
  activeFilters,
  onChangeFilters,
  secondaryMenuOpen,
  selectedResult,
  menuItems,
  menuFocusedIndex,
  pendingConfirm,
  inlineInput,
  onSecondaryAction,
  onMenuFocus,
  onInlineInputChange,
  onInlineInputKeyDown,
  showPreview,
  previewForSelected,
  previewLoading,
  expandedMetadata,
  selectedMetadata,
  footerHint,
}: Props) {
  return (
    <div
      className={`bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl ${
        secondaryMenuOpen ? "overflow-visible" : "overflow-hidden"
      }`}
    >
      <FilterChips active={activeFilters} onChange={onChangeFilters} />
      <div className={showPreview ? "grid grid-cols-[1fr_320px]" : ""}>
        <div className={`relative min-w-0 ${secondaryMenuOpen ? "min-h-[384px]" : ""}`}>
          <ul className="max-h-[352px] overflow-y-auto py-1">
            {visibleResults.map((r, i) => {
              const badge = KIND_BADGE[r.kind] ?? KIND_BADGE.file;
              const icon = r.icon_key ? iconsByKey[r.icon_key] : null;
              return (
                <li
                  key={r.path}
                  ref={(el) => {
                    if (i === safeSelected && el) el.scrollIntoView({ block: "nearest" });
                  }}
                  onMouseDown={() => onLaunch(r)}
                  onMouseEnter={(e) => {
                    onSelectIndex(i);
                    if (!showRankBreakdown) return;
                    onHoverStart(i, e.currentTarget.getBoundingClientRect());
                  }}
                  onMouseLeave={onHoverEnd}
                  className={`flex items-center gap-2 px-4 py-2.5 cursor-pointer text-sm transition-colors ${
                    i === safeSelected ? "bg-blue-600/70 text-white" : "text-gray-300 hover:bg-white/8"
                  }`}
                >
                  {icon ? (
                    <img
                      src={icon.data_url}
                      alt=""
                      className="h-6 w-6 shrink-0 rounded"
                      draggable={false}
                    />
                  ) : (
                    <span
                      className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${badge.cls}`}
                    >
                      {badge.label}
                    </span>
                  )}
                  <span
                    className={`truncate font-medium${
                      hasEncodingError(r.title ?? r.name) ? " text-gray-500 italic" : ""
                    }`}
                  >
                    {hasEncodingError(r.title ?? r.name)
                      ? "(無法解碼的名稱)"
                      : (r.title ?? r.name)}
                  </span>
                  {Boolean(r.secondary_action_count) && (
                    <span className="shrink-0 text-[10px] text-gray-500">
                      +{r.secondary_action_count}
                    </span>
                  )}
                  {r.kind !== "app" && (
                    <span className="ml-auto shrink-0 max-w-[220px] truncate text-xs text-gray-500">
                      {hasEncodingError(r.subtitle ?? r.path)
                        ? "(無法解碼的路徑)"
                        : (r.subtitle ?? r.path)}
                    </span>
                  )}
                </li>
              );
            })}
          </ul>

          {secondaryMenuOpen && selectedResult && menuItems.length > 0 && (
            <SecondaryActionMenu
              result={selectedResult}
              items={menuItems}
              focusedIndex={menuFocusedIndex}
              onSelect={(id) => onSecondaryAction(id, selectedResult)}
              onHoverEnabled={onMenuFocus}
              pendingConfirmId={pendingConfirm}
              inlineInput={inlineInput}
              onInlineInputChange={onInlineInputChange}
              onInlineInputKeyDown={onInlineInputKeyDown}
            />
          )}
        </div>

        {showPreview && (
          <div
            className={`${
              secondaryMenuOpen ? "h-[384px]" : "max-h-[352px]"
            } border-l border-gray-700/50 bg-gray-950/40`}
          >
            <PreviewPane
              result={selectedResult}
              preview={previewForSelected}
              loading={previewLoading}
            />
          </div>
        )}
      </div>

      {expandedMetadata && selectedResult && (
        <div className="border-t border-gray-700/50 px-4 py-2 text-xs text-gray-400 bg-gray-950/60">
          <div className="mb-1 flex items-center justify-between">
            <span className="text-[10px] uppercase tracking-wider text-gray-500">Metadata</span>
            <span className="text-[10px] text-gray-600">Esc 收起</span>
          </div>
          <div className="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-0.5 font-mono">
            <span className="text-gray-500">path</span>
            <span className="truncate text-gray-300">{selectedResult.path}</span>
            {selectedMetadata?.size_bytes !== undefined && (
              <>
                <span className="text-gray-500">size</span>
                <span className="text-gray-300">
                  {selectedMetadata.size_bytes.toLocaleString()} bytes
                </span>
              </>
            )}
            {selectedMetadata?.modified_ms !== undefined && (
              <>
                <span className="text-gray-500">modified</span>
                <span className="text-gray-300">
                  {new Date(selectedMetadata.modified_ms).toLocaleString()}
                </span>
              </>
            )}
            {selectedMetadata?.is_dir !== undefined && (
              <>
                <span className="text-gray-500">type</span>
                <span className="text-gray-300">
                  {selectedMetadata.is_dir ? "folder" : "file"}
                </span>
              </>
            )}
            {selectedMetadata?.preview && (
              <>
                <span className="text-gray-500">preview</span>
                <span className="text-gray-300 whitespace-pre-wrap break-words">
                  {selectedMetadata.preview}
                </span>
              </>
            )}
          </div>
        </div>
      )}

      <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between gap-2">
        <span className="min-w-0 flex-1 truncate">{footerHint}</span>
        <span className="shrink-0">Enter 開啟</span>
        <span className="shrink-0">Shift+Enter 次要</span>
        <span className="shrink-0">→ Actions</span>
      </div>
    </div>
  );
}
