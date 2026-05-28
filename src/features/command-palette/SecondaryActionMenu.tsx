import type React from "react";
import { useEffect, useRef } from "react";

import type { SearchResult } from "../../types/search";
import {
  isDestructive,
  type SecondaryActionId,
  type SecondaryActionItem,
} from "../../utils/secondaryActions";

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
  const enabled = items.filter((item) => !item.disabled);
  const focusedId = enabled[focusedIndex]?.id ?? null;
  const itemRefs = useRef<Record<string, HTMLDivElement | null>>({});

  useEffect(() => {
    if (!focusedId) return;
    itemRefs.current[focusedId]?.scrollIntoView({ block: "nearest" });
  }, [focusedId]);

  return (
    <div
      role="menu"
      aria-label="Secondary actions"
      className="kn-panel-shell absolute right-3 top-3 z-20 flex h-[380px] max-h-[calc(100vh-24px)] w-[400px] max-w-[calc(100%-24px)] flex-col overflow-hidden rounded-[18px]"
    >
      <div className="shrink-0 border-b border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.52)] px-4 py-2 text-[10px] uppercase tracking-[0.2em] text-[color:var(--kn-text-faint)]">
        Actions / {result.title ?? result.name}
      </div>

      <div className="kn-scroll min-h-0 flex-1 overflow-y-auto px-2 py-2">
        {items.map((item) => {
          const enabledIndex = enabled.findIndex((entry) => entry.id === item.id);
          const isFocused = !item.disabled && enabledIndex === focusedIndex;
          const isPending = pendingConfirmId === item.id;
          const showInput = isFocused && inlineInput && inlineInput.for === item.id;
          const isArmedDestructive = isFocused && isPending && isDestructive(item.id);
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
                      ? "cursor-pointer border-red-400/30 bg-red-500/15 text-red-100"
                      : isFocused
                        ? "border-[color:rgba(127,212,255,0.18)] bg-[rgba(127,212,255,0.12)] text-white"
                        : `cursor-pointer ${tintClass}`
                }`}
                data-selected={isFocused && !isArmedDestructive ? "true" : "false"}
              >
                <span className="truncate font-medium">
                  {isArmedDestructive ? `Confirm ${item.label}? Press Enter again` : item.label}
                </span>
                {item.disabled && item.disabledReason ? (
                  <span className="shrink-0 text-[10px] text-[color:var(--kn-text-faint)]">
                    {item.disabledReason}
                  </span>
                ) : item.hint && !isArmedDestructive ? (
                  <span className="shrink-0 text-[10px] text-[color:var(--kn-text-muted)]">
                    {item.hint}
                  </span>
                ) : null}
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
                    className="w-full rounded-[12px] border border-[color:var(--kn-border-strong)] bg-[rgba(7,11,17,0.74)] px-3 py-2 text-sm text-[color:var(--kn-text)] outline-none transition-colors focus:border-[color:rgba(127,212,255,0.34)]"
                    placeholder={
                      inlineInput.for === "rename" ? "New name" : "Target folder absolute path"
                    }
                  />
                  <div className="mt-1 text-[10px] text-[color:var(--kn-text-muted)]">
                    Enter preview / Enter again to confirm / Esc cancel
                  </div>
                </div>
              ) : null}
            </div>
          );
        })}
      </div>

      <div className="flex shrink-0 items-center justify-between border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.52)] px-4 py-2 text-[11px] text-[color:var(--kn-text-muted)]">
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Up/Down</span>
          <span>navigate</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Enter</span>
          <span>run</span>
        </span>
        <span className="flex items-center gap-1.5">
          <span className="kn-kbd">Esc</span>
          <span>close</span>
        </span>
      </div>
    </div>
  );
}
