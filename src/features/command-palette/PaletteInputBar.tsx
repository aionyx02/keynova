import type React from "react";

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
  const isEmptySearch = mode === "search" && query === "";
  // The prompt mark replaces the icon: one mono glyph that also says which
  // mode the bar is in, which the icon never did.
  const promptMark = mode === "command" ? "/" : mode === "terminal" ? "$" : "\u203a";

  return (
    <div
      className={`kn-launcher-bar kn-panel-shell kn-panel-focus flex items-center gap-3 px-4 ${
        hasContentBelow ? "rounded-b-none" : ""
      }`}
    >
      <span
        className="kn-mono shrink-0 text-[15px] font-medium text-[color:var(--kn-accent)]"
        aria-hidden="true"
      >
        {promptMark}
      </span>

      <div className="kn-input-shell flex min-w-0 flex-1 items-center gap-2">
        {isEmptySearch && <span className="kn-caret" aria-hidden="true" />}
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={onKeyDown}
          onFocus={onFocus}
          aria-label={modeLabel}
          placeholder={mode === "command" ? t.command.placeholder : t.search.placeholder}
          className="min-w-0 flex-1 bg-transparent text-[20px] font-normal leading-7 tracking-[-0.018em] text-[color:var(--kn-text)] placeholder:text-[color:var(--kn-text-muted)] outline-none focus-visible:shadow-none"
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
              className="kn-mono inline-flex items-center gap-1.5 px-1 py-1 text-[11px] text-[color:var(--kn-text-faint)] transition hover:text-[color:var(--kn-text-soft)]"
            >
              <span>/</span>
              <span>{p.modeCommands}</span>
            </button>
            <button
              type="button"
              aria-label={p.terminalModeHint}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => onQueryChange(">")}
              className="kn-mono inline-flex items-center gap-1.5 px-1 py-1 text-[11px] text-[color:var(--kn-text-faint)] transition hover:text-[color:var(--kn-text-soft)]"
            >
              <span>&gt;</span>
              <span>{t.terminal.terminal}</span>
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
