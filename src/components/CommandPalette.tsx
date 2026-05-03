import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { invoke } from "@tauri-apps/api/core";
import { useIPC } from "../hooks/useIPC";
import { useAppStore } from "../stores/appStore";
import { parseInputMode } from "../hooks/useInputMode";
import { TerminalPanel } from "./TerminalPanel";
import type { SearchResult } from "../types/search";

const TERMINAL_HEIGHT = 360;

async function hideWindow() {
  try {
    await getCurrentWindow().hide();
  } catch {
    // non-Tauri env: no-op
  }
}

async function keepLauncherOpen() {
  try {
    await invoke("cmd_keep_launcher_open");
  } catch {
    try {
      await getCurrentWindow().setFocus();
    } catch {
      // non-Tauri env: no-op
    }
  }
}

const KIND_BADGE: Record<string, { label: string; cls: string }> = {
  app: { label: "App", cls: "bg-violet-500/30 text-violet-300" },
  file: { label: "File", cls: "bg-sky-500/30 text-sky-300" },
  folder: { label: "Dir", cls: "bg-amber-500/30 text-amber-300" },
};

export function CommandPalette() {
  const { dispatch } = useIPC();
  const { query, setQuery, setLoading } = useAppStore();
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const { mode } = parseInputMode(query);
  const modeRef = useRef(mode);

  useEffect(() => {
    modeRef.current = mode;
  }, [mode]);

  // Focus on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Re-focus input when mode changes back from terminal
  // (input is unmounted in terminal mode, so we wait for React to commit the new DOM)
  useEffect(() => {
    if (mode !== "terminal") {
      const raf = requestAnimationFrame(() => {
        inputRef.current?.focus();
      });
      return () => cancelAnimationFrame(raf);
    }
  }, [mode]);

  // Re-focus and clear when window reappears
  useEffect(() => {
    const unlisten = listen<void>("window-focused", () => {
      if (modeRef.current === "terminal") return;
      setQuery("");
      setResults([]);
      setSelected(0);
      inputRef.current?.focus();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setQuery]);

  // ESC in search/command mode → hide window
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape" && mode !== "terminal") {
        e.preventDefault();
        setQuery("");
        setResults([]);
        void hideWindow();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [setQuery, mode]);

  // Window sizing: terminal mode → fixed height; search/command → fit content
  useEffect(() => {
    if (mode === "terminal") {
      getCurrentWindow()
        .setSize(new LogicalSize(640, TERMINAL_HEIGHT))
        .catch(() => {});
      return;
    }
    const el = containerRef.current;
    if (!el) return;
    const h = Math.ceil(el.getBoundingClientRect().height) || 56;
    getCurrentWindow()
      .setSize(new LogicalSize(640, h))
      .catch(() => {});
  }, [mode, results]);

  async function handleQueryChange(value: string) {
    setQuery(value);
    const { mode: newMode, rawInput } = parseInputMode(value);
    if (newMode !== "search" || rawInput.trim() === "") {
      setResults([]);
      setSelected(0);
      return;
    }
    setLoading(true);
    try {
      const data = await dispatch<SearchResult[]>("search.query", {
        query: rawInput,
        limit: 10,
      });
      setResults(data);
      setSelected(0);
    } catch {
      setResults([]);
    } finally {
      setLoading(false);
    }
  }

  async function launchResult(result: SearchResult) {
    try {
      await dispatch("launcher.launch", { path: result.path });
    } finally {
      setQuery("");
      setResults([]);
      void hideWindow();
    }
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (mode !== "search") return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const r = results[selected];
      if (r) void launchResult(r);
    }
  }

  const hasResults = results.length > 0 && mode === "search";

  // Single persistent wrapper keeps a focusable element in the DOM at all times,
  // preventing WebView2 from firing Focused(false) → window.hide() during mode transitions.
  return (
    <div ref={containerRef} tabIndex={-1} className="w-full outline-none">
      {mode === "terminal" ? (
        <TerminalPanel
          onExit={async () => {
            await keepLauncherOpen();
            // Transfer focus to the wrapper before unmounting xterm so the window
            // never loses OS focus while the terminal's textarea is being disposed.
            containerRef.current?.focus();
            setQuery("");
            requestAnimationFrame(() => inputRef.current?.focus());
          }}
        />
      ) : (
        <div className="flex flex-col">
          {/* Input bar */}
          <div
            className={`flex items-center bg-gray-900/95 backdrop-blur-md shadow-2xl ${
              hasResults ? "rounded-t-xl border-b border-gray-700/50" : "rounded-xl"
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
              onChange={(e) => void handleQueryChange(e.target.value)}
              onKeyDown={onKeyDown}
              placeholder={
                mode === "command"
                  ? "輸入指令… 試試 /help"
                  : "搜尋應用程式、檔案或資料夾… 輸入 > 進入終端"
              }
              className="flex-1 bg-transparent py-4 pr-4 text-base text-gray-100 placeholder-gray-500 outline-none"
              spellCheck={false}
              autoComplete="off"
            />
          </div>

          {/* Search results */}
          {hasResults && (
            <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
              <ul className="max-h-[352px] overflow-y-auto py-1">
                {results.map((r, i) => {
                  const badge = KIND_BADGE[r.kind] ?? KIND_BADGE.file;
                  return (
                    <li
                      key={r.path}
                      onMouseDown={() => void launchResult(r)}
                      onMouseEnter={() => setSelected(i)}
                      className={`flex items-center gap-2 px-4 py-2.5 cursor-pointer text-sm transition-colors ${
                        i === selected
                          ? "bg-blue-600/70 text-white"
                          : "text-gray-300 hover:bg-white/8"
                      }`}
                    >
                      <span
                        className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${badge.cls}`}
                      >
                        {badge.label}
                      </span>
                      <span className="truncate font-medium">{r.name}</span>
                      {r.kind !== "app" && (
                        <span className="ml-auto shrink-0 max-w-[220px] truncate text-xs text-gray-500">
                          {r.path}
                        </span>
                      )}
                    </li>
                  );
                })}
              </ul>
              <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
                <span>↑↓ 選擇</span>
                <span>Enter 開啟</span>
                <span>Esc 關閉</span>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
