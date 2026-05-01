import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useIPC } from "../hooks/useIPC";
import { useAppStore } from "../stores/appStore";
import type { AppInfo } from "../types/app";

async function hideWindow() {
  try {
    await getCurrentWindow().hide();
  } catch {
    // non-Tauri env: no-op
  }
}

export function CommandPalette() {
  const { dispatch } = useIPC();
  const { searchResults, query, isLoading, setSearchResults, setQuery, setLoading } =
    useAppStore();
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Re-focus input and clear state whenever the window comes into focus
  useEffect(() => {
    const unlisten = listen<void>("window-focused", () => {
      setQuery("");
      setSearchResults([]);
      setSelected(0);
      inputRef.current?.focus();
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setQuery, setSearchResults]);

  // Escape → hide window
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        setQuery("");
        setSearchResults([]);
        void hideWindow();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [setQuery, setSearchResults]);

  async function handleQueryChange(value: string) {
    setQuery(value);
    if (value.trim() === "") {
      setSearchResults([]);
      setSelected(0);
      return;
    }
    setLoading(true);
    try {
      const results = await dispatch<AppInfo[]>("launcher.search", { query: value });
      setSearchResults(results.slice(0, 8));
      setSelected(0);
    } catch {
      setSearchResults([]);
    } finally {
      setLoading(false);
    }
  }

  async function launchApp(app: AppInfo) {
    try {
      await dispatch("launcher.launch", { path: app.path });
    } finally {
      setQuery("");
      setSearchResults([]);
      void hideWindow();
    }
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, searchResults.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const app = searchResults[selected];
      if (app) void launchApp(app);
    }
  }

  const hasResults = searchResults.length > 0 || isLoading;

  return (
    // Full-screen transparent overlay — click outside to dismiss
    <div
      className="fixed inset-0 flex flex-col items-center"
      style={{ paddingTop: "28vh" }}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) void hideWindow();
      }}
    >
      <div className="w-[640px] max-w-[90vw]">
        {/* Input */}
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
            placeholder="搜尋應用程式…"
            className="flex-1 bg-transparent py-4 pr-4 text-base text-gray-100 placeholder-gray-500 outline-none"
            spellCheck={false}
            autoComplete="off"
          />
          {isLoading && (
            <span className="mr-4 text-xs text-gray-500">搜尋中…</span>
          )}
        </div>

        {/* Results — only rendered when there's content */}
        {(hasResults) && (
          <div className="bg-gray-900/92 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
            <ul className="max-h-72 overflow-y-auto py-1">
              {searchResults.map((app, i) => (
                <li
                  key={app.path}
                  onMouseDown={() => void launchApp(app)}
                  onMouseEnter={() => setSelected(i)}
                  className={`flex items-center gap-3 px-5 py-2.5 cursor-pointer text-sm transition-colors ${
                    i === selected
                      ? "bg-blue-600/70 text-white"
                      : "text-gray-300 hover:bg-white/8"
                  }`}
                >
                  <span className="truncate">{app.name}</span>
                  <span className="ml-auto shrink-0 text-xs text-gray-500 truncate max-w-[240px]">
                    {app.path}
                  </span>
                </li>
              ))}
            </ul>
            <div className="border-t border-gray-700/50 px-4 py-1.5 text-xs text-gray-600 flex justify-between">
              <span>↑↓ 選擇</span>
              <span>Enter 啟動</span>
              <span>Esc 關閉</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
