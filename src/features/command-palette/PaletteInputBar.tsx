import type React from "react";

import keynovaLogo from "../../assets/keynova_icon.png";
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
        <div className="kn-input-shell flex items-center gap-3 px-3 py-2.5">
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

      <div className="flex shrink-0 items-center gap-2">
        <span className="kn-mode-pill">
          <UiIcon name={modeIcon} className="h-3.5 w-3.5" />
          <span className="hidden sm:inline">{modeLabel}</span>
        </span>
        <img
          src={keynovaLogo}
          alt="Keynova"
          className="h-7 w-7 rounded-[7px] border border-white/10 bg-white/[0.035] p-1 opacity-90"
          draggable={false}
        />
      </div>
    </div>
  );
}
