import type React from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";

interface Props {
  mode: "search" | "command" | "terminal";
  query: string;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onQueryChange: (value: string) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  onFocus: () => void;
  hasContentBelow: boolean;
}

export function PaletteInputBar({
  mode,
  query,
  inputRef,
  onQueryChange,
  onKeyDown,
  onFocus,
  hasContentBelow,
}: Props) {
  const t = useI18n();
  const p = t.palette;
  const modeLabel = mode === "command" ? p.modeCommands : p.modeSearch;
  const modeIcon = mode === "command" ? "command" : "search";
  const isEmptySearch = mode === "search" && query === "";

  return (
    <div
      className={`kn-launcher-bar kn-panel-shell kn-panel-focus flex items-center gap-3 px-3 py-3 ${
        hasContentBelow ? "rounded-b-none" : ""
      }`}
    >
      <div className="kn-launcher-mark" aria-hidden="true">
        <UiIcon name={modeIcon} className="h-5 w-5" />
      </div>

      <div className="min-w-0 flex-1">
        <div className="kn-input-shell flex items-center gap-2 px-3 py-2.5">
          {isEmptySearch && <span className="kn-caret" aria-hidden="true" />}
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            onKeyDown={onKeyDown}
            onFocus={onFocus}
            aria-label={modeLabel}
            placeholder={mode === "command" ? t.command.placeholder : t.search.placeholder}
            className="min-w-0 flex-1 bg-transparent text-[16px] font-semibold leading-6 text-[color:var(--kn-text)] placeholder:text-[color:var(--kn-text-muted)] outline-none focus-visible:shadow-none"
            spellCheck={false}
            autoComplete="off"
            style={isEmptySearch ? { caretColor: "transparent" } : undefined}
          />

          {mode === "search" && query === "" && (
            <div className="flex shrink-0 items-center gap-1">
              <button
                type="button"
                aria-label={p.commandModeHint}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onQueryChange("/")}
                className="inline-flex items-center gap-1 rounded-[6px] px-1.5 py-1 text-[10px] font-semibold text-[color:var(--kn-text-muted)] transition hover:bg-white/[0.05] hover:text-[color:var(--kn-text-soft)]"
              >
                <span className="kn-kbd">/</span>
                <span>{p.modeCommands}</span>
              </button>
              <button
                type="button"
                aria-label={p.terminalModeHint}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onQueryChange(">")}
                className="inline-flex items-center gap-1 rounded-[6px] px-1.5 py-1 text-[10px] font-semibold text-[color:var(--kn-text-muted)] transition hover:bg-white/[0.05] hover:text-[color:var(--kn-text-soft)]"
              >
                <span className="kn-kbd">&gt;</span>
                <span>{t.terminal.terminal}</span>
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
