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
import { WorkspaceIndicator } from "./WorkspaceIndicator";
import type { SearchResult } from "../types/search";
import type { BuiltinCommandResult } from "../hooks/useCommands";
import type { WorkspaceState } from "../hooks/useWorkspace";

const TerminalPanel = React.lazy(() =>
  import("./TerminalPanel").then((m) => ({ default: m.TerminalPanel })),
);

const TERMINAL_HEIGHT = 360;

interface SettingEntry {
  key: string;
  value: string;
}

interface ConfigReloadedPayload {
  changed_keys: string[];
}

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
  const { all, filtered, runCommand, suggestArgs } = useCommands();

  const [results, setResults] = useState<SearchResult[]>([]);
  const [selected, setSelected] = useState(0);

  // Command mode state
  const [selectedCmd, setSelectedCmd] = useState(0);
  const [cmdResult, setCmdResult] = useState<BuiltinCommandResult | null>(null);

  // Mount terminal once and keep it alive; only toggle visibility via CSS
  const [terminalMounted, setTerminalMounted] = useState(false);

  // Arg suggestions state (shown when user types space after an exact command match)
  const [argSuggestions, setArgSuggestions] = useState<string[]>([]);
  const [selectedArg, setSelectedArg] = useState(0);

  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const argDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const searchIdRef = useRef(0);
  const searchLimitRef = useRef(10);

  const { mode, rawInput } = parseInputMode(query);
  const modeRef = useRef(mode);

  useEffect(() => { modeRef.current = mode; }, [mode]);


  // Split rawInput into command name and trailing args (Minecraft-style)
  const spaceIdx = rawInput.search(/\s/);
  const cmdName = spaceIdx === -1 ? rawInput : rawInput.slice(0, spaceIdx);
  const cmdArgs = spaceIdx === -1 ? "" : rawInput.slice(spaceIdx + 1).trim();

  const cmdSuggestions = useMemo(
    () => (mode === "command" ? filtered(cmdName) : []),
    [mode, cmdName, filtered],
  );

  // Args phase: user typed a space after a known command name
  const exactCmd = mode === "command" && spaceIdx !== -1
    ? (all.find((c) => c.name === cmdName) ?? null)
    : null;
  const isArgsPhase = exactCmd !== null;

  // Fetch arg suggestions whenever the args phase is active and cmdArgs changes.
  // Clearing on phase exit is handled in handleQueryChange to avoid synchronous setState in effect.
  useEffect(() => {
    if (!isArgsPhase || !exactCmd) return;
    if (argDebounceRef.current) clearTimeout(argDebounceRef.current);
    argDebounceRef.current = setTimeout(() => {
      suggestArgs(exactCmd.name, cmdArgs)
        .then((results) => { setArgSuggestions(results); setSelectedArg(0); })
        .catch(() => {});
    }, 150);
    return () => {
      if (argDebounceRef.current) clearTimeout(argDebounceRef.current);
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isArgsPhase, exactCmd?.name, cmdArgs]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    async function refreshLauncherSettings() {
      try {
        const entries = await invoke<SettingEntry[]>("cmd_dispatch", {
          route: "setting.list_all",
          payload: null,
        });
        const maxResults = entries.find((entry) => entry.key === "launcher.max_results")?.value;
        const parsed = Number.parseInt(maxResults ?? "", 10);
        if (Number.isFinite(parsed) && parsed > 0) {
          searchLimitRef.current = parsed;
        }
      } catch {
        // keep current search limit
      }
    }

    void refreshLauncherSettings();
    const unlisten = listen<ConfigReloadedPayload>("config-reloaded", (event) => {
      if (
        event.payload.changed_keys.length === 0 ||
        event.payload.changed_keys.includes("launcher.max_results")
      ) {
        void refreshLauncherSettings();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
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

  // 工作區切換：載入切換後的 query 並重置 UI 狀態
  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlisten = listen<WorkspaceState>("workspace-switched", (event) => {
      const ws = event.payload;
      setQuery(ws.query ?? "");
      setResults([]);
      setSelected(0);
      setCmdResult(null);
      requestAnimationFrame(() => inputRef.current?.focus());
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [setQuery]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();

      if (mode === "terminal") {
        // Safety net: exit terminal mode even when xterm lost keyboard focus.
        // Capture phase lets us run before xterm can consume the key event.
        containerRef.current?.focus();
        setQuery("");
        requestAnimationFrame(() => inputRef.current?.focus());
        void keepLauncherOpen();
        return;
      }

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
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
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
  }, [mode, results, cmdSuggestions, cmdResult, argSuggestions]);

  function handleQueryChange(value: string) {
    setQuery(value);
    setCmdResult(null);
    setSelectedCmd(0);
    setSelectedArg(0);
    setArgSuggestions([]);
    const { mode: newMode, rawInput: ri } = parseInputMode(value);
    // Mount terminal on first "> " entry; avoids useEffect setState cascade
    if (newMode === "terminal") setTerminalMounted(true);
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
        const data = await dispatch<SearchResult[]>("search.query", {
          query: ri,
          limit: searchLimitRef.current,
        });
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

  async function execCommand(name: string, args = "") {
    try {
      const result = await runCommand(name, args);
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
      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (isArgsPhase) setSelectedArg((i) => Math.min(i + 1, argSuggestions.length - 1));
        else setSelectedCmd((i) => Math.min(i + 1, cmdSuggestions.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        if (isArgsPhase) setSelectedArg((i) => Math.max(i - 1, 0));
        else setSelectedCmd((i) => Math.max(i - 1, 0));
      } else if (e.key === "Tab") {
        e.preventDefault();
        if (isArgsPhase && argSuggestions.length > 0) {
          // Fill in the selected config key and add a trailing space for value input
          const arg = argSuggestions[selectedArg];
          if (arg) setQuery(`/${cmdName} ${arg} `);
        } else if (!isArgsPhase) {
          const cmd = cmdSuggestions[selectedCmd];
          if (cmd) setQuery("/" + cmd.name);
        }
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (isArgsPhase) {
          void execCommand(cmdName, cmdArgs);
        } else {
          const cmd = cmdSuggestions[selectedCmd];
          if (cmd) void execCommand(cmd.name, cmdArgs);
        }
      }
    }
  }

  const hasResults = results.length > 0 && mode === "search";
  const hasCmdSuggestions = cmdSuggestions.length > 0 && mode === "command" && !cmdResult && !isArgsPhase;
  const hasArgSuggestions = isArgsPhase && argSuggestions.length > 0 && !cmdResult;

  // Resolve panel component if cmd result is a Panel type
  const PanelComponent = cmdResult?.ui_type.type === "Panel"
    ? (PanelRegistry[cmdResult.ui_type.value ?? ""] ?? null)
    : null;

  const terminalOnExit = () => {
    // Move focus to container first so terminal becoming display:none
    // doesn't shift focus to document.body and risk hiding the window
    containerRef.current?.focus();
    setQuery("");
    requestAnimationFrame(() => inputRef.current?.focus());
    // Keep window open asynchronously — fire and forget
    void keepLauncherOpen();
  };

  return (
    <div ref={containerRef} tabIndex={-1} className="w-full outline-none">
      {/* Terminal: mounted once on first visit, hidden via CSS when not active */}
      {terminalMounted && (
        <div style={{ display: mode === "terminal" ? "block" : "none" }}>
          <Suspense fallback={<div className="h-[360px] bg-gray-900/95 rounded-xl" />}>
            <TerminalPanel isActive={mode === "terminal"} onExit={terminalOnExit} />
          </Suspense>
        </div>
      )}

      <div style={{ display: mode === "terminal" ? "none" : "block" }}>
        <div className="flex flex-col">
          {/* Input bar */}
          <div
            className={`flex items-center bg-gray-900/95 backdrop-blur-md shadow-2xl ${
              hasResults || hasCmdSuggestions || cmdResult || isArgsPhase
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
              className="flex-1 bg-transparent py-4 text-base text-gray-100 placeholder-gray-500 outline-none"
              spellCheck={false}
              autoComplete="off"
            />
            <div className="pr-3">
              <WorkspaceIndicator />
            </div>
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
              onSelect={(name) => void execCommand(name, cmdArgs)}
              onHover={setSelectedCmd}
            />
          )}

          {/* Args phase: syntax hint bar */}
          {isArgsPhase && exactCmd && !cmdResult && (
            <div className="bg-gray-900/95 backdrop-blur-md px-4 py-1.5 text-xs text-gray-500 border-t border-gray-700/30">
              <span className="text-blue-400">/{exactCmd.name}</span>
              {exactCmd.args_hint && (
                <span className="ml-1 font-mono text-gray-600">{exactCmd.args_hint}</span>
              )}
              <span className="ml-3 text-gray-700">Tab 填入 · Enter 執行</span>
            </div>
          )}

          {/* Args suggestions dropdown */}
          {hasArgSuggestions && (
            <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
              <ul className="max-h-[220px] overflow-y-auto py-1">
                {argSuggestions.map((arg, i) => (
                  <li
                    key={arg}
                    onMouseDown={() => { setQuery(`/${cmdName} ${arg} `); setSelectedArg(i); }}
                    onMouseEnter={() => setSelectedArg(i)}
                    className={`flex items-center gap-2 px-4 py-2 cursor-pointer text-sm font-mono transition-colors ${
                      i === selectedArg ? "bg-blue-600/70 text-white" : "text-gray-300 hover:bg-white/8"
                    }`}
                  >
                    {arg}
                  </li>
                ))}
              </ul>
              <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
                <span>↑↓ 選擇</span><span>Tab 填入</span><span>Enter 執行</span>
              </div>
            </div>
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
      </div>
    </div>
  );
}
