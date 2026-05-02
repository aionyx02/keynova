import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useIPC } from "../hooks/useIPC";
import { useAppStore } from "../stores/appStore";
import type { SearchResult } from "../types/search";

async function hideWindow() {
  try {
    await getCurrentWindow().hide();
  } catch {
    // non-Tauri env: no-op
  }
}

export function CommandPalette() {
  const { dispatch } = useIPC();
  const { query, setQuery, setLoading } = useAppStore();
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Re-focus and clear when window reappears
  useEffect(() => {
    const unlisten = listen<void>("window-focused", () => {
      setQuery("");
      setResults([]);
      setSelected(0);
      inputRef.current?.focus();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [setQuery]);

  // Escape → hide window
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        setQuery("");
        setResults([]);
        void hideWindow();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [setQuery]);

  async function handleQueryChange(value: string) {
    setQuery(value);
    if (value.trim() === "") {
      setResults([]);
      setSelected(0);
      return;
    }
    setLoading(true);
    try {
      const data = await dispatch<SearchResult[]>("search.query", {
        query: value,
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

  const hasResults = results.length > 0;

  return (
    <div
      className="fixed inset-0 flex flex-col items-center"
      style={{ paddingTop: "28vh" }}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) void hideWindow();
      }}
    >
      <div className="w-[640px] max-w-[90vw]">
        {/* Search input */}
        <div
          className={`flex items-center bg-gray-900/92 backdrop-blur-md shadow-2xl ${
            hasResults ? "rounded-t-xl border-b border-gray-700/50" : "rounded-xl"
          }`}
        >
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
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => void handleQueryChange(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="搜尋應用程式或檔案…"
            className="flex-1 bg-transparent py-4 pr-4 text-base text-gray-100 placeholder-gray-500 outline-none"
            spellCheck={false}
            autoComplete="off"
          />
        </div>

        {/* Results — only when there's something to show */}
        {hasResults && (
          <div className="bg-gray-900/92 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
            <ul className="max-h-72 overflow-y-auto py-1">
              {results.map((r, i) => (
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
                  {/* Kind badge */}
                  <span
                    className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${
                      r.kind === "app"
                        ? "bg-violet-500/30 text-violet-300"
                        : "bg-sky-500/30 text-sky-300"
                    }`}
                  >
                    {r.kind === "app" ? "App" : "File"}
                  </span>
                  <span className="truncate font-medium">{r.name}</span>
                  {r.kind === "file" && (
                    <span className="ml-auto shrink-0 max-w-[220px] truncate text-xs text-gray-500">
                      {r.path}
                    </span>
                  )}
                </li>
              ))}
            </ul>
            <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
              <span>↑↓ 選擇</span>
              <span>Enter 開啟</span>
              <span>Esc 關閉</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
