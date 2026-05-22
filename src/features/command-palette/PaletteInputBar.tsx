// REF.2.P4 — Search/command input bar.
//
// Owns the leading icon (mode-dependent), the <input>, the optional search
// backend chip, and the workspace indicator. State stays in CommandPalette;
// this component is presentational + keyboard/IME pass-through.

import type React from "react";
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
  /** When true the bar uses rounded-t-xl + bottom border (content rendered below); else rounded-xl. */
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
  return (
    <div
      className={`flex items-center bg-gray-900/95 backdrop-blur-md shadow-2xl ${
        hasContentBelow ? "rounded-t-xl border-b border-gray-700/50" : "rounded-xl"
      }`}
    >
      {mode === "command" ? (
        <span className="ml-4 mr-2 text-sm font-bold text-blue-400 select-none">/</span>
      ) : (
        <svg
          className="ml-4 mr-2 h-4 w-4 shrink-0 text-gray-400"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2}
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z"
          />
        </svg>
      )}
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => onQueryChange(e.target.value)}
        onKeyDown={onKeyDown}
        // Bug-fix 2026-05-19 (round 2) — onFocus + per-keydown throttle in
        // onKeyDown together cover initial mount, post-Esc re-focus, and
        // every subsequent keystroke regardless of IME state. See
        // CommandPalette for the throttle + ref.
        onFocus={onFocus}
        placeholder={
          mode === "command"
            ? "輸入指令… 試試 /help 或 /setting"
            : "搜尋應用程式、檔案或資料夾… 輸入 > 進入終端"
        }
        className="flex-1 bg-transparent py-4 text-base text-gray-100 placeholder-gray-500 outline-none"
        spellCheck={false}
        autoComplete="off"
      />
      {searchBackend && mode === "search" && (
        <span
          title={`configured=${searchBackend.configured}, everything=${searchBackend.everything_available}, tantivy=${searchBackend.tantivy_available}, cache=${searchBackend.file_cache_entries}, tantivy_docs=${searchBackend.tantivy_index_entries}, index=${searchBackend.tantivy_index_dir}`}
          className="mr-2 hidden shrink-0 rounded border border-gray-700/70 bg-gray-950/70 px-2 py-1 text-[10px] font-semibold uppercase text-gray-400 sm:inline-flex"
        >
          {searchBackend.active}
        </span>
      )}
      <div className="pr-3">
        <WorkspaceIndicator />
      </div>
    </div>
  );
}
