import { useEffect, useRef } from "react";
import type { SearchResult } from "../types/search";
import { isDestructive, type SecondaryActionId, type SecondaryActionItem } from "../utils/secondaryActions";

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
  const enabled = items.filter((it) => !it.disabled);
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
      className="absolute right-3 top-3 z-20 flex h-[360px] max-h-[calc(100vh-24px)] w-[390px] max-w-[calc(100%-24px)] flex-col overflow-hidden rounded-md border border-gray-700/60 bg-gray-950/95 shadow-2xl backdrop-blur-md"
    >
      <div className="shrink-0 border-b border-gray-700/40 px-3 py-1.5 text-[10px] uppercase tracking-wider text-gray-500">
        Actions · {result.title ?? result.name}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto py-1">
        {items.map((it) => {
          const enabledIdx = enabled.findIndex((x) => x.id === it.id);
          const isFocused = !it.disabled && enabledIdx === focusedIndex;
          const isPending = pendingConfirmId === it.id;
          const showInput = isFocused && inlineInput && inlineInput.for === it.id;
          const riskTint = it.risk === "high"
            ? "text-red-300"
            : it.risk === "medium"
              ? "text-amber-200"
              : "text-gray-300";
          return (
            <div
              key={it.id}
              ref={(node) => {
                itemRefs.current[it.id] = node;
              }}
            >
              <div
                role="menuitem"
                aria-disabled={it.disabled || undefined}
                onMouseEnter={() => {
                  if (!it.disabled) onHoverEnabled(enabledIdx);
                }}
                onMouseDown={(e) => {
                  e.preventDefault();
                  if (!it.disabled) onSelect(it.id);
                }}
                className={`flex min-h-[32px] items-center justify-between gap-3 px-3 py-1.5 text-sm transition-colors ${
                  it.disabled
                    ? "cursor-not-allowed text-gray-600"
                    : isFocused
                      ? `cursor-pointer bg-blue-600/70 text-white`
                      : `cursor-pointer ${riskTint} hover:bg-white/8`
                }`}
              >
                <span className="truncate">{it.label}</span>
                {it.disabled && it.disabledReason ? (
                  <span className="shrink-0 text-[10px] text-gray-600">{it.disabledReason}</span>
                ) : it.hint ? (
                  <span className="shrink-0 text-[10px] text-gray-500">{it.hint}</span>
                ) : null}
              </div>
              {showInput && onInlineInputChange ? (
                <div className="bg-gray-900/80 px-3 pb-1.5 pt-1">
                  <input
                    autoFocus
                    type="text"
                    value={inlineInput.value}
                    onChange={(e) => onInlineInputChange(e.target.value)}
                    onKeyDown={onInlineInputKeyDown}
                    onMouseDown={(e) => e.stopPropagation()}
                    className="w-full rounded border border-gray-700/60 bg-gray-950 px-2 py-1 text-xs text-gray-100 focus:border-blue-500/70 focus:outline-none"
                    placeholder={inlineInput.for === "rename" ? "new name" : "target folder absolute path"}
                  />
                  <div className="mt-1 text-[10px] text-gray-500">
                    Enter preview · Enter again to confirm · Esc cancel
                  </div>
                </div>
              ) : null}
              {isFocused && isPending && isDestructive(it.id) ? (
                <div className="mx-2 mb-1 mt-0.5 rounded border border-red-500/70 bg-red-900/30 px-2 py-1 text-[11px] text-red-200">
                  Confirm {it.label} — Enter / y = yes · Esc / n = cancel
                </div>
              ) : null}
            </div>
          );
        })}
      </div>
      <div className="flex shrink-0 justify-between border-t border-gray-700/40 px-3 py-1 text-[10px] text-gray-600">
        <span>↑↓ navigate</span>
        <span>Enter run · Esc close</span>
      </div>
    </div>
  );
}
