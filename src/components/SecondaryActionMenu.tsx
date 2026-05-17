import type { SearchResult } from "../types/search";
import type { SecondaryActionId, SecondaryActionItem } from "../utils/secondaryActions";

interface Props {
  result: SearchResult;
  items: SecondaryActionItem[];
  focusedIndex: number;
  onSelect: (id: SecondaryActionId) => void;
  onHoverEnabled: (enabledIndex: number) => void;
}

export function SecondaryActionMenu({ result, items, focusedIndex, onSelect, onHoverEnabled }: Props) {
  const enabled = items.filter((it) => !it.disabled);

  return (
    <div
      role="menu"
      aria-label="Secondary actions"
      className="absolute right-3 top-3 z-20 min-w-[240px] rounded-md border border-gray-700/60 bg-gray-950/95 py-1 shadow-2xl backdrop-blur-md"
    >
      <div className="px-3 py-1.5 text-[10px] uppercase tracking-wider text-gray-500 border-b border-gray-700/40">
        Actions · {result.title ?? result.name}
      </div>
      {items.map((it) => {
        const enabledIdx = enabled.findIndex((x) => x.id === it.id);
        const isFocused = !it.disabled && enabledIdx === focusedIndex;
        return (
          <div
            key={it.id}
            role="menuitem"
            aria-disabled={it.disabled || undefined}
            onMouseEnter={() => { if (!it.disabled) onHoverEnabled(enabledIdx); }}
            onMouseDown={(e) => {
              e.preventDefault();
              if (!it.disabled) onSelect(it.id);
            }}
            className={`flex items-center justify-between gap-3 px-3 py-1.5 text-sm transition-colors ${
              it.disabled
                ? "cursor-not-allowed text-gray-600"
                : isFocused
                  ? "cursor-pointer bg-blue-600/70 text-white"
                  : "cursor-pointer text-gray-300 hover:bg-white/8"
            }`}
          >
            <span className="truncate">{it.label}</span>
            {it.disabled && it.disabledReason ? (
              <span className="text-[10px] text-gray-600">{it.disabledReason}</span>
            ) : it.hint ? (
              <span className="text-[10px] text-gray-500">{it.hint}</span>
            ) : null}
          </div>
        );
      })}
      <div className="border-t border-gray-700/40 px-3 py-1 text-[10px] text-gray-600 flex justify-between">
        <span>↑↓ navigate</span><span>Enter run · Esc close</span>
      </div>
    </div>
  );
}
