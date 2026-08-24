import { useState } from "react";
import type React from "react";
import type { ReactNode } from "react";

import { UiIcon, type UiIconName } from "../../components/icons/UiIcon";
import { PreviewPane } from "../../shared/components/PreviewPane";
import type {
  FilePreviewResult,
  SearchIconAsset,
  SearchMetadata,
  SearchResult,
  SourceFilter,
} from "../../types/search";
import type { UnifiedResult } from "../../types/unified-result";
import type { SecondaryActionItem, SecondaryActionId } from "../../utils/secondaryActions";
import { FilterChips } from "./FilterChips";
import { SecondaryActionMenu } from "./SecondaryActionMenu";
import type { SecondaryInlineInput } from "./hooks/useSecondaryMenu";
import { useI18n } from "../../i18n/useI18n";

const KIND_BADGE: Record<string, { label: string; cls: string; icon: UiIconName }> = {
  app: {
    label: "App",
    cls: "border-white/10 bg-white/[0.045] text-[color:var(--kn-text-soft)]",
    icon: "app",
  },
  file: {
    label: "File",
    cls: "border-white/10 bg-white/[0.045] text-[color:var(--kn-text-soft)]",
    icon: "file",
  },
  folder: {
    label: "Dir",
    cls: "border-amber-400/18 bg-amber-400/10 text-amber-200",
    icon: "folder",
  },
  command: {
    label: "Cmd",
    cls: "border-[color:rgba(138,168,255,0.2)] bg-[color:var(--kn-accent-wash)] text-[color:var(--kn-accent)]",
    icon: "command",
  },
  note: {
    label: "Note",
    cls: "border-white/10 bg-white/[0.045] text-[color:var(--kn-text-soft)]",
    icon: "note",
  },
  history: {
    label: "Hist",
    cls: "border-white/10 bg-white/[0.045] text-[color:var(--kn-text-soft)]",
    icon: "history",
  },
  model: {
    label: "AI",
    cls: "border-[color:rgba(88,211,166,0.22)] bg-[color:var(--kn-success-wash)] text-[color:var(--kn-success)]",
    icon: "model",
  },
  memory: {
    label: "Mem",
    cls: "border-[color:rgba(138,168,255,0.2)] bg-[color:var(--kn-accent-wash)] text-[color:var(--kn-accent)]",
    icon: "database",
  },
};

function hasEncodingError(s: string | undefined | null): boolean {
  return typeof s === "string" && s.includes("嚙");
}

interface Props {
  visibleResults: SearchResult[];
  unifiedVisible: UnifiedResult[];
  safeSelected: number;
  iconsByKey: Record<string, SearchIconAsset>;
  onSelectIndex: (index: number) => void;
  onLaunch: (result: SearchResult) => void;
  showRankBreakdown: boolean;
  onHoverStart: (index: number, rect: DOMRect) => void;
  onHoverEnd: () => void;
  activeFilters: Set<SourceFilter>;
  onChangeFilters: (next: Set<SourceFilter>) => void;
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
  showPreview: boolean;
  previewForSelected: FilePreviewResult | undefined;
  previewLoading: boolean;
  expandedMetadata: boolean;
  selectedMetadata: SearchMetadata | null | undefined;
  footerHint: ReactNode;
}

export function SearchResultsList({
  visibleResults,
  unifiedVisible,
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
  const t = useI18n();
  const [brokenIconKeys, setBrokenIconKeys] = useState<Record<string, true>>({});

  return (
    <div
      className={`kn-panel-shell rounded-t-none border-t-0 ${
        secondaryMenuOpen ? "overflow-visible" : "overflow-hidden"
      }`}
    >
      <FilterChips active={activeFilters} onChange={onChangeFilters} />

      <div className={showPreview ? "grid grid-cols-[minmax(0,1fr)_336px]" : ""}>
        <div className={`relative min-w-0 ${secondaryMenuOpen ? "min-h-[420px]" : ""}`}>
          <ul className="kn-scroll max-h-[360px] space-y-1 overflow-y-auto px-2 py-2">
            {visibleResults.map((result, index) => {
              const isSelected = index === safeSelected;
              const badge = KIND_BADGE[result.kind] ?? KIND_BADGE.file;
              const localizedKind = t.search.kinds[result.kind] ?? badge.label;
              const iconKey = result.icon_key ?? "";
              const icon = iconKey ? iconsByKey[iconKey] : null;
              const showIconImage = Boolean(icon && !brokenIconKeys[iconKey]);
              const unified = unifiedVisible[index];
              const title = hasEncodingError(result.title ?? result.name)
                ? t.search.unavailableText
                : (result.title ?? result.name);
              const detailSource =
                result.kind === "app" ? result.subtitle : (result.subtitle ?? result.path);
              // UX.AUDIT.5 fallback: if the mojibake guard trips on the detail line,
              // show the raw path instead of masking it as "Path unavailable" so the
              // row stays actionable (open/reveal still work off result.path).
              const detail = hasEncodingError(detailSource)
                ? result.path || detailSource
                : detailSource;

              return (
                <li
                  key={unified?.id ?? result.path}
                  ref={(element) => {
                    if (isSelected && element) {
                      element.scrollIntoView({ block: "nearest" });
                    }
                  }}
                  data-selected={isSelected ? "true" : "false"}
                  onMouseDown={() => onLaunch(result)}
                  onMouseEnter={(event) => {
                    onSelectIndex(index);
                    if (!showRankBreakdown) return;
                    onHoverStart(index, event.currentTarget.getBoundingClientRect());
                  }}
                  onMouseLeave={onHoverEnd}
                  className="kn-result-row flex min-h-[42px] cursor-pointer items-center gap-2.5 px-3"
                >
                  {showIconImage ? (
                    <div className="kn-result-icon">
                      <img
                        src={icon?.data_url}
                        alt=""
                        className="h-5 w-5 shrink-0 rounded-[4px] object-contain"
                        draggable={false}
                        onError={() => {
                          if (!iconKey) return;
                          setBrokenIconKeys((prev) =>
                            prev[iconKey] ? prev : { ...prev, [iconKey]: true },
                          );
                        }}
                      />
                    </div>
                  ) : (
                    <div className="kn-result-icon" title={localizedKind}>
                      <UiIcon name={badge.icon} className="h-[18px] w-[18px]" />
                    </div>
                  )}

                  <span
                    className={`min-w-0 flex-1 truncate text-[13px] ${
                      isSelected
                        ? "font-medium text-[color:var(--kn-text)]"
                        : "text-[color:var(--kn-text-soft)]"
                    }`}
                  >
                    {title}
                  </span>

                  {isSelected && detail && (
                    <span className="hidden max-w-[45%] shrink-0 truncate text-[11.5px] text-[color:var(--kn-text-muted)] sm:block">
                      {detail}
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
            className={`border-l border-[color:var(--kn-border)] bg-[color:var(--kn-panel-bg-strong)] ${
              secondaryMenuOpen ? "h-[420px]" : "max-h-[360px]"
            }`}
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
        <div className="border-t border-[color:var(--kn-border)] bg-[color:var(--kn-panel-bg-strong)] px-4 py-3 text-xs text-[color:var(--kn-text-soft)]">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-[10px] uppercase tracking-[0.2em] text-[color:var(--kn-text-faint)]">
              {t.search.metadata}
            </span>
            <span className="text-[10px] text-[color:var(--kn-text-muted)]">
              {t.search.escCollapse}
            </span>
          </div>
          <div className="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-1 font-mono text-[11px]">
            <span className="text-[color:var(--kn-text-faint)]">{t.search.path}</span>
            <span className="truncate text-[color:var(--kn-text-soft)]">{selectedResult.path}</span>
            {selectedMetadata?.size_bytes !== undefined && (
              <>
                <span className="text-[color:var(--kn-text-faint)]">{t.search.size}</span>
                <span className="text-[color:var(--kn-text-soft)]">
                  {selectedMetadata.size_bytes.toLocaleString()} bytes
                </span>
              </>
            )}
            {selectedMetadata?.modified_ms !== undefined && (
              <>
                <span className="text-[color:var(--kn-text-faint)]">{t.search.modified}</span>
                <span className="text-[color:var(--kn-text-soft)]">
                  {new Date(selectedMetadata.modified_ms).toLocaleString()}
                </span>
              </>
            )}
            {selectedMetadata?.is_dir !== undefined && (
              <>
                <span className="text-[color:var(--kn-text-faint)]">{t.search.type}</span>
                <span className="text-[color:var(--kn-text-soft)]">
                  {selectedMetadata.is_dir ? t.search.kinds.folder : t.search.kinds.file}
                </span>
              </>
            )}
            {selectedMetadata?.preview && (
              <>
                <span className="text-[color:var(--kn-text-faint)]">{t.search.preview}</span>
                <span className="whitespace-pre-wrap break-words text-[color:var(--kn-text-soft)]">
                  {selectedMetadata.preview}
                </span>
              </>
            )}
          </div>
        </div>
      )}

      <div className="kn-panel-footer kn-result-footer flex-wrap">
        <span className="min-w-0 flex-1 truncate">{footerHint}</span>
        <div className="flex items-center gap-3">
          <span className="flex items-center gap-1.5">
            <span className="kn-kbd">Enter</span>
            <span>{t.search.open}</span>
          </span>
          <span className="flex items-center gap-1.5">
            <span className="kn-kbd">Tab</span>
            <span>{t.search.actions}</span>
          </span>
        </div>
      </div>
    </div>
  );
}
