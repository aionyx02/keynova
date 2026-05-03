import React, { useEffect, useMemo, useRef, useState, Suspense } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { invoke } from "@tauri-apps/api/core";
import { useIPC } from "../hooks/useIPC";
import { useAppStore } from "../stores/appStore";
import { parseInputMode } from "../hooks/useInputMode";
import { useCommands } from "../hooks/useCommands";
import { CommandSuggestions } from "./CommandSuggestions";
import { PanelRegistry } from "./panel/PanelRegistry";
import type { SearchResult } from "../types/search";
import type { BuiltinCommandResult } from "../hooks/useCommands";

const TerminalPanel = React.lazy(() =>
  import("./TerminalPanel").then((m) => ({ default: m.TerminalPanel })),
);

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
  const { filtered, runCommand } = useCommands();

  const [results, setResults] = useState<SearchResult[]>([]);
  const [selected, setSelected] = useState(0);

  // Command mode state
  const [selectedCmd, setSelectedCmd] = useState(0);
  const [cmdResult, setCmdResult] = useState<BuiltinCommandResult | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const searchIdRef = useRef(0);

  const { mode, rawInput } = parseInputMode(query);
  const modeRef = useRef(mode);

  useEffect(() => { modeRef.current = mode; }, [mode]);

  const cmdSuggestions = useMemo(
    () => (mode === "command" ? filtered(rawInput) : []),
    [mode, rawInput, filtered],
  );

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (mode !== "terminal") {
      const raf = requestAnimationFrame(() => { inputRef.current?.focus(); });
      return () => cancelAnimationFrame(raf);
    }
  }, [mode]);

  useEffect(() => {
    const unlisten = listen<void>("window-focused", () => {
      if (modeRef.current === "terminal") return;
      setQuery("");
      setResults([]);
      setSelected(0);
      setCmdResult(null);
      inputRef.current?.focus();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [setQuery]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape" && mode !== "terminal") {
        e.preventDefault();
        if (cmdResult !== null) {
          // Close panel → return to search mode without hiding
          setCmdResult(null);
          setQuery("");
          setResults([]);
          requestAnimationFrame(() => inputRef.current?.focus());
        } else if (query !== "") {
          // Clear input but stay visible
          setQuery("");
          setResults([]);
          inputRef.current?.focus();
        } else {
          void hideWindow();
        }
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [setQuery, mode, query, cmdResult]);

  // Window sizing
  useEffect(() => {
    if (mode === "terminal") {
      getCurrentWindow().setSize(new LogicalSize(640, TERMINAL_HEIGHT)).catch(() => {});
      return;
    }
    const el = containerRef.current;
    if (!el) return;
    const h = Math.ceil(el.getBoundingClientRect().height) || 56;
    getCurrentWindow().setSize(new LogicalSize(640, h)).catch(() => {});
  }, [mode, results, cmdSuggestions, cmdResult]);

  function handleQueryChange(value: string) {
    setQuery(value);
    setCmdResult(null);
    setSelectedCmd(0);
    const { mode: newMode, rawInput: ri } = parseInputMode(value);
    if (newMode !== "search" || ri.trim() === "") {
      setResults([]);
      setSelected(0);
      if (debounceRef.current) clearTimeout(debounceRef.current);
      return;
    }
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      const reqId = ++searchIdRef.current;
      setLoading(true);
      try {
        const data = await dispatch<SearchResult[]>("search.query", { query: ri, limit: 10 });
        if (reqId !== searchIdRef.current) return;
        setResults(data);
        setSelected(0);
      } catch {
        if (reqId === searchIdRef.current) setResults([]);
      } finally {
        if (reqId === searchIdRef.current) setLoading(false);
      }
    }, 200);
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

  async function execCommand(name: string) {
    try {
      const result = await runCommand(name);
      setCmdResult(result);
    } catch {
      // ignore
    }
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (mode === "search") {
      if (e.key === "ArrowDown") { e.preventDefault(); setSelected((i) => Math.min(i + 1, results.length - 1)); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setSelected((i) => Math.max(i - 1, 0)); }
      else if (e.key === "Enter") { e.preventDefault(); const r = results[selected]; if (r) void launchResult(r); }
    } else if (mode === "command") {
      if (e.key === "ArrowDown") { e.preventDefault(); setSelectedCmd((i) => Math.min(i + 1, cmdSuggestions.length - 1)); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setSelectedCmd((i) => Math.max(i - 1, 0)); }
      else if (e.key === "Tab") {
        e.preventDefault();
        const cmd = cmdSuggestions[selectedCmd];
        if (cmd) setQuery("/" + cmd.name);
      }
      else if (e.key === "Enter") {
        e.preventDefault();
        const cmd = cmdSuggestions[selectedCmd];
        if (cmd) void execCommand(cmd.name);
      }
    }
  }

  const hasResults = results.length > 0 && mode === "search";
  const hasCmdSuggestions = cmdSuggestions.length > 0 && mode === "command" && !cmdResult;

  // Resolve panel component if cmd result is a Panel type
  const PanelComponent = cmdResult?.ui_type.type === "Panel"
    ? (PanelRegistry[cmdResult.ui_type.value ?? ""] ?? null)
    : null;

  return (
    <div ref={containerRef} tabIndex={-1} className="w-full outline-none">
      {mode === "terminal" ? (
        <Suspense fallback={<div className="h-[360px] bg-gray-900/95 rounded-xl" />}>
          <TerminalPanel
            onExit={async () => {
              await keepLauncherOpen();
              containerRef.current?.focus();
              setQuery("");
              requestAnimationFrame(() => inputRef.current?.focus());
            }}
          />
        </Suspense>
      ) : (
        <div className="flex flex-col">
          {/* Input bar */}
          <div
            className={`flex items-center bg-gray-900/95 backdrop-blur-md shadow-2xl ${
              hasResults || hasCmdSuggestions || cmdResult
                ? "rounded-t-xl border-b border-gray-700/50"
                : "rounded-xl"
            }`}
          >
            {mode === "command" ? (
              <span className="ml-4 mr-2 text-sm font-bold text-blue-400 select-none">/</span>
            ) : (
              <svg className="ml-4 mr-2 h-4 w-4 shrink-0 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z" />
              </svg>
            )}
            <input
              ref={inputRef}
              value={query}
              onChange={(e) => void handleQueryChange(e.target.value)}
              onKeyDown={onKeyDown}
              placeholder={
                mode === "command"
                  ? "輸入指令… 試試 /help 或 /setting"
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
                        i === selected ? "bg-blue-600/70 text-white" : "text-gray-300 hover:bg-white/8"
                      }`}
                    >
                      <span className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${badge.cls}`}>
                        {badge.label}
                      </span>
                      <span className="truncate font-medium">{r.name}</span>
                      {r.kind !== "app" && (
                        <span className="ml-auto shrink-0 max-w-[220px] truncate text-xs text-gray-500">{r.path}</span>
                      )}
                    </li>
                  );
                })}
              </ul>
              <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
                <span>↑↓ 選擇</span><span>Enter 開啟</span><span>Esc 關閉</span>
              </div>
            </div>
          )}

          {/* Command suggestions */}
          {hasCmdSuggestions && (
            <CommandSuggestions
              commands={cmdSuggestions}
              selectedIndex={selectedCmd}
              onSelect={(name) => void execCommand(name)}
              onHover={setSelectedCmd}
            />
          )}

          {/* Inline command result */}
          {cmdResult?.ui_type.type === "Inline" && cmdResult.text && (
            <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl px-4 py-3">
              <pre className="text-sm text-gray-300 whitespace-pre-wrap leading-relaxed">{cmdResult.text}</pre>
            </div>
          )}

          {/* Panel command result */}
          {PanelComponent && <PanelComponent />}
        </div>
      )}
    </div>
  );
}