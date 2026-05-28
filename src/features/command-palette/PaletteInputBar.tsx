import type React from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import { WorkspaceIndicator } from "../../components/WorkspaceIndicator";
import type { SearchBackendInfo } from "../../ipc/types";

interface Props {
  mode: "search" | "command" | "terminal";
  query: string;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onQueryChange: (value: string) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  onFocus: () => void;
  searchBackend: SearchBackendInfo | null;
  hasContentBelow: boolean;
}

export function PaletteInputBar({
  mode,
  query,
  inputRef,
  onQueryChange,
  onKeyDown,
  onFocus,
  searchBackend,
  hasContentBelow,
}: Props) {
  const iconShellClass = [
    "flex h-10 w-10 shrink-0 items-center justify-center rounded-[14px] border",
    "shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]",
    mode === "command"
      ? "border-[color:rgba(127,212,255,0.22)] bg-[rgba(127,212,255,0.12)] text-[color:var(--kn-accent)]"
      : "border-[color:var(--kn-border)] bg-white/[0.035] text-[color:var(--kn-text-soft)]",
  ].join(" ");

  return (
    <div
      className={`kn-panel-shell kn-panel-focus flex items-center gap-3 px-3 py-3 ${
        hasContentBelow ? "rounded-b-none" : ""
      }`}
    >
      <div className={iconShellClass} aria-hidden="true">
        {mode === "command" ? (
          <UiIcon name="command" className="h-[18px] w-[18px]" />
        ) : (
          <UiIcon name="search" className="h-[18px] w-[18px]" />
        )}
      </div>

      <div className="min-w-0 flex-1">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={onKeyDown}
          onFocus={onFocus}
          placeholder={
            mode === "command"
              ? "輸入指令，例如 /help 或 /setting"
              : "搜尋應用、檔案或資料夾，輸入 > 進入終端"
          }
          className="w-full bg-transparent text-[15px] font-medium text-[color:var(--kn-text)] placeholder:text-[color:var(--kn-text-muted)] outline-none"
          spellCheck={false}
          autoComplete="off"
        />
        <div className="mt-1 flex items-center gap-2 text-[11px] text-[color:var(--kn-text-faint)]">
          <span>{mode === "command" ? "Command mode" : "Launcher"}</span>
          <span className="h-1 w-1 rounded-full bg-white/10" />
          <span>Keyboard-first flow</span>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        {searchBackend && mode === "search" && (
          <span
            title={`configured=${searchBackend.configured}, everything=${searchBackend.everything_available}, tantivy=${searchBackend.tantivy_available}, cache=${searchBackend.file_cache_entries}, tantivy_docs=${searchBackend.tantivy_index_entries}, index=${searchBackend.tantivy_index_dir}`}
            className="hidden items-center gap-1.5 rounded-[12px] border border-[color:var(--kn-border)] bg-[rgba(255,255,255,0.035)] px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-[color:var(--kn-text-soft)] sm:inline-flex"
          >
            <UiIcon name="database" className="h-3.5 w-3.5" />
            {searchBackend.active}
          </span>
        )}
        <WorkspaceIndicator />
      </div>
    </div>
  );
}
