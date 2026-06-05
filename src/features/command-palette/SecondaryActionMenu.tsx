import type React from "react";
import { useEffect, useRef } from "react";

import type { SearchResult } from "../../types/search";
import {
  isDestructive,
  type SecondaryActionId,
  type SecondaryActionItem,
} from "../../utils/secondaryActions";
import { useI18n } from "../../i18n/useI18n";
import { fmt } from "../../i18n/format";

interface Props {
  result: SearchResult;
  items: SecondaryActionItem[];
  focusedIndex: number;
  onSelect: (id: SecondaryActionId) => void;
  onHoverEnabled: (enabledIndex: number) => void;
  pendingConfirmId?: SecondaryActionId | null;
  inlineInput?: { for: "rename" | "move"; value: string } | null;
  onInlineInputChange?: (value: string) => void;
  onInlineInputKeyDown?: (e: React.KeyboardEvent<HTMLInputElement>) => void;
}

function riskPillClass(risk: string): string {
  if (risk === "high") {
    return "border-red-300/20 bg-[color:var(--kn-danger-wash)] text-red-100";
  }
  if (risk === "medium") {
    return "border-amber-300/20 bg-amber-300/10 text-amber-100";
  }
  return "border-white/10 bg-white/[0.035] text-[color:var(--kn-text-muted)]";
}

export function SecondaryActionMenu({
  result,
  items,
  focusedIndex,
  onSelect,
  onHoverEnabled,
  pendingConfirmId,
  inlineInput,
  onInlineInputChange,
  onInlineInputKeyDown,
}: Props) {
  const p = useI18n().palette;
  const enabled = items.filter((item) => !item.disabled);
  const focusedId = enabled[focusedIndex]?.id ?? null;
  const itemRefs = useRef<Record<string, HTMLDivElement | null>>({});
  const targetTitle = result.title ?? result.name;
  const targetDetail = result.subtitle ?? result.path;

  useEffect(() => {
    if (!focusedId) return;
    itemRefs.current[focusedId]?.scrollIntoView({ block: "nearest" });
  }, [focusedId]);

  return (
    <div
      role="menu"
      aria-label={p.secondaryActions}
      className="kn-panel-shell absolute right-3 top-3 z-20 flex h-[380px] max-h-[calc(100vh-24px)] w-[400px] max-w-[calc(100%-24px)] flex-col overflow-hidden rounded-[8px]"
    >
      <div className="shrink-0 border-b border-[color:var(--kn-border)] bg-white/[0.025] px-4 py-3">
        <div className="text-[10px] font-semibold uppercase tracking-[0.14em] text-[color:var(--kn-text-faint)]">
          {p.actionsFor}
        </div>
        <div className="mt-1 truncate text-sm font-semibold text-[color:var(--kn-text)]">
          {targetTitle}
        </div>
        {targetDetail && targetDetail !== targetTitle && (
          <div className="mt-0.5 truncate text-[11px] text-[color:var(--kn-text-muted)]">
            {targetDetail}
          </div>
        )}
      </div>

      <div className="kn-scroll min-h-0 flex-1 overflow-y-auto px-2 py-2">
        {items.map((item) => {
          const enabledIndex = enabled.findIndex((entry) => entry.id === item.id);
          const isFocused = !item.disabled && enabledIndex === focusedIndex;
          const isPending = pendingConfirmId === item.id;
          const showInput = isFocused && inlineInput && inlineInput.for === item.id;
          const isArmedDestructive = isFocused && isPending && isDestructive(item.id);
          const riskLabel = p.riskLabels[item.risk] ?? item.risk;
          const showRisk = item.risk !== "low" || isArmedDestructive;
          const tintClass =
            item.risk === "high"
              ? "text-red-200"
              : item.risk === "medium"
                ? "text-amber-100"
                : "text-[color:var(--kn-text-soft)]";

          return (
            <div
              key={item.id}
              ref={(node) => {
                itemRefs.current[item.id] = node;
              }}
            >
              <div
                role="menuitem"
                aria-disabled={item.disabled || undefined}
                onMouseEnter={() => {
                  if (!item.disabled) onHoverEnabled(enabledIndex);
                }}
                onMouseDown={(event) => {
                  event.preventDefault();
                  if (!item.disabled) onSelect(item.id);
                }}
                className={`kn-result-row flex min-h-[40px] items-center justify-between gap-3 px-3 py-2 text-sm transition-all duration-150 ${
                  item.disabled
                    ? "cursor-not-allowed text-[color:var(--kn-text-faint)]"
                    : isArmedDestructive
                      ? "cursor-pointer border-red-400/30 bg-[color:var(--kn-danger-wash)] text-red-100"
                      : isFocused
                        ? "border-[color:rgba(125,216,193,0.24)] bg-[color:var(--kn-accent-wash)] text-white"
                        : `cursor-pointer ${tintClass}`
                }`}
                data-selected={isFocused && !isArmedDestructive ? "true" : "false"}
              >
                <span className="truncate font-medium">
                  {isArmedDestructive ? fmt(p.confirmAction, { label: item.label }) : item.label}
                </span>
                <span className="flex min-w-0 shrink-0 items-center gap-2">
                  {showRisk && (
                    <span
                      className={`rounded-full border px-2 py-0.5 text-[10px] font-semibold ${riskPillClass(item.risk)}`}
                    >
                      {riskLabel}
                    </span>
                  )}
                  {item.disabled && item.disabledReason ? (
                    <span className="max-w-[8rem] truncate text-[10px] text-[color:var(--kn-text-faint)]">
                      {item.disabledReason}
                    </span>
                  ) : item.hint && !isArmedDestructive ? (
                    <span className="max-w-[8rem] truncate text-[10px] text-[color:var(--kn-text-muted)]">
                      {item.hint}
                    </span>
                  ) : null}
                </span>
              </div>

              {showInput && onInlineInputChange ? (
                <div className="px-3 pb-2 pt-1">
                  <input
                    autoFocus
                    type="text"
                    value={inlineInput.value}
                    onChange={(event) => onInlineInputChange(event.target.value)}
                    onKeyDown={onInlineInputKeyDown}
                    onMouseDown={(event) => event.stopPropagation()}
                    className="kn-field w-full text-sm"
                    placeholder={
                      inlineInput.for === "rename" ? p.renamePlaceholder : p.movePlaceholder
                    }
                  />
                  <div className="mt-1 text-[10px] text-[color:var(--kn-text-muted)]">
                    {p.inlineHint}
                  </div>
                </div>
              ) : null}
            </div>
          );
        })}
      </div>

      <div className="kn-panel-footer shrink-0">
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Up/Down</span>
          <span>{p.navigate}</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Enter</span>
          <span>{p.run}</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Esc</span>
          <span>{p.close}</span>
        </span>
      </div>
    </div>
  );
}
