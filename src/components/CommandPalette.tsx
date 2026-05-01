import { useEffect, useRef, useState } from "react";
import { FloatingWindow } from "./FloatingWindow";
import { useIPC } from "../hooks/useIPC";
import { useAppStore } from "../stores/appStore";
import type { AppInfo } from "../types/app";

interface CommandPaletteProps {
  onClose: () => void;
}

export function CommandPalette({ onClose }: CommandPaletteProps) {
  const { dispatch } = useIPC();
  const { searchResults, query, isLoading, setSearchResults, setQuery, setLoading } =
    useAppStore();
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  async function handleQueryChange(value: string) {
    setQuery(value);
    setLoading(true);
    try {
      if (value.trim() === "") {
        const apps = await dispatch<AppInfo[]>("launcher.list_all");
        setSearchResults(apps.slice(0, 10));
      } else {
        const results = await dispatch<AppInfo[]>("launcher.search", { query: value });
        setSearchResults(results.slice(0, 10));
      }
      setSelected(0);
    } catch {
      setSearchResults([]);
      setSelected(0);
    } finally {
      setLoading(false);
    }
  }

  async function launchSelected() {
    const app = searchResults[selected];
    if (!app) return;
    try {
      await dispatch("launcher.launch", { path: app.path });
    } finally {
      onClose();
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
      launchSelected();
    }
  }

  return (
    <FloatingWindow title="應用程式啟動器" onClose={onClose}>
      <div className="p-3">
        <input
          ref={inputRef}
          value={query}
          onChange={(e) => handleQueryChange(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder="搜尋應用程式…"
          className="w-full bg-gray-800 text-gray-100 placeholder-gray-500 rounded px-3 py-2 text-sm outline-none border border-gray-600 focus:border-blue-500"
        />
      </div>

      {isLoading && (
        <div className="px-4 py-2 text-xs text-gray-500">搜尋中…</div>
      )}

      <ul className="max-h-80 overflow-y-auto pb-2">
        {searchResults.map((app, i) => (
          <li
            key={app.path}
            onClick={() => { setSelected(i); launchSelected(); }}
            className={`flex items-center gap-3 px-4 py-2 cursor-pointer text-sm ${
              i === selected ? "bg-blue-600 text-white" : "text-gray-300 hover:bg-gray-800"
            }`}
          >
            <span className="truncate">{app.name}</span>
          </li>
        ))}
        {!isLoading && searchResults.length === 0 && (
          <li className="px-4 py-3 text-xs text-gray-500">找不到應用程式</li>
        )}
      </ul>
    </FloatingWindow>
  );
}
