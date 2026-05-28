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
  const iconShellClass = [
    "flex h-9 w-9 shrink-0 items-center justify-center rounded-[8px] border",
    "shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]",
    mode === "command"
      ? "border-[color:rgba(242,191,112,0.18)] bg-[rgba(242,191,112,0.1)] text-[color:var(--kn-warm)]"
      : "border-[color:var(--kn-border)] bg-white/[0.035] text-[color:var(--kn-text-soft)]",
  ].join(" ");
  const modeLabel = mode === "command" ? "Commands" : "Search";

  return (
    <div
      className={`kn-panel-shell kn-panel-focus flex items-center gap-2.5 px-3 py-2.5 ${
        hasContentBelow ? "rounded-b-none" : ""
      }`}
    >
      <div className={iconShellClass} aria-hidden="true">
        {mode === "command" ? (
          <UiIcon name="command" className="h-4 w-4" />
        ) : (
          <UiIcon name="search" className="h-4 w-4" />
        )}
      </div>

      <div className="min-w-0 flex-1">
        <div className="mb-0.5 flex items-center gap-1.5 text-[9px] text-[color:var(--kn-text-faint)]">
          <span className="kn-chip px-1.5 py-0">{modeLabel}</span>
          <span className="truncate">
            {mode === "command" ? "Run actions without leaving the keyboard" : "Apps, files, notes, commands"}
          </span>
        </div>
        <div className="rounded-[8px] border border-[color:rgba(255,255,255,0.08)] bg-white/[0.02] px-2.5 py-1.5 transition-colors focus-within:border-[color:rgba(138,168,255,0.3)] focus-within:bg-white/[0.035]">
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            onKeyDown={onKeyDown}
            onFocus={onFocus}
            placeholder={
              mode === "command"
                ? t.command.placeholder
                : t.search.placeholder
            }
            className="w-full bg-transparent text-[15px] font-medium leading-5 text-[color:var(--kn-text)] placeholder:text-[color:var(--kn-text-muted)] outline-none focus-visible:shadow-none"
            spellCheck={false}
            autoComplete="off"
          />
        </div>
        <div className="mt-1 flex items-center gap-1.5 text-[10px] text-[color:var(--kn-text-faint)]">
          <span>{mode === "command" ? "Slash to switch back to search" : "Type / for commands"}</span>
          <span className="h-1 w-1 rounded-full bg-white/10" />
          <span>Keyboard-first</span>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2 self-center">
        <span className="hidden text-[10px] font-medium uppercase tracking-[0.08em] text-[color:var(--kn-text-faint)] xl:inline">
          Keynova
        </span>
        <img
          src={keynovaLogo}
          alt="Keynova"
          className="h-[18px] w-[18px] rounded-[4px] opacity-75"
          draggable={false}
        />
      </div>
    </div>
  );
}
