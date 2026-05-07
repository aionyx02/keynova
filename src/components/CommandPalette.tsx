import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, Suspense } from "react";
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
import type { SearchChunkPayload, SearchErrorPayload, SearchMetadata, SearchResult } from "../types/search";
import type { ActionRef } from "../types/search";
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

interface SearchBackendInfo {
  configured: string;
  active: string;
  everything_available: boolean;
  tantivy_available: boolean;
  file_cache_entries: number;
  rebuild_supported: boolean;
}

interface SecondaryAction {
  action_ref: ActionRef;
  label: string;
  risk: "low" | "medium" | "high";
}

function isEditorTerminalResult(result: BuiltinCommandResult | null) {
  return result?.ui_type.type === "Terminal" && result.ui_type.value.editor;
}

function isTerminalResult(result: BuiltinCommandResult | null) {
  return result?.ui_type.type === "Terminal";
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
  command: { label: "Cmd", cls: "bg-emerald-500/30 text-emerald-300" },
  note: { label: "Note", cls: "bg-teal-500/30 text-teal-300" },
  history: { label: "Hist", cls: "bg-zinc-500/30 text-zinc-300" },
  model: { label: "AI", cls: "bg-fuchsia-500/30 text-fuchsia-300" },
};

function searchResultKey(result: SearchResult) {
  return `${result.source ?? result.kind}:${result.path}`;
}

function mergeSearchResults(existing: SearchResult[], incoming: SearchResult[]) {
  const seen = new Set(existing.map(searchResultKey));
  const merged = [...existing];
  for (const item of incoming) {
    const key = searchResultKey(item);
    if (seen.has(key)) continue;
    seen.add(key);
    merged.push(item);
  }
  return merged;
}

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
  const [searchBackend, setSearchBackend] = useState<SearchBackendInfo | null>(null);
  const [metadataByPath, setMetadataByPath] = useState<Record<string, SearchMetadata>>({});
  const [timedOutProviders, setTimedOutProviders] = useState<string[]>([]);

  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const argDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const resizeRafRef = useRef<number | null>(null);
  const searchIdRef = useRef(0);
  const activeSearchRequestRef = useRef("");
  const searchLimitRef = useRef(10);

  const { mode, rawInput } = parseInputMode(query);

  // Refs always hold the latest values — read inside the ESC handler
  // to avoid stale closures when the listener is registered only once.
  const modeRef = useRef(mode);
  const cmdResultRef = useRef(cmdResult);
  const queryRef = useRef(query);
  useLayoutEffect(() => {
    modeRef.current = mode;
    cmdResultRef.current = cmdResult;
    queryRef.current = query;
  });


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

  const scheduleWindowResize = useCallback(() => {
    if (!window.__TAURI_INTERNALS__) return;
    if (resizeRafRef.current !== null) {
      cancelAnimationFrame(resizeRafRef.current);
    }
    resizeRafRef.current = requestAnimationFrame(() => {
      resizeRafRef.current = null;
      if (modeRef.current === "terminal") {
        getCurrentWindow().setSize(new LogicalSize(640, TERMINAL_HEIGHT)).catch(() => {});
        return;
      }
      if (isTerminalResult(cmdResultRef.current)) {
        getCurrentWindow().setSize(new LogicalSize(640, TERMINAL_HEIGHT)).catch(() => {});
        return;
      }
      const el = containerRef.current;
      if (!el) return;
      const rectHeight = Math.ceil(el.getBoundingClientRect().height);
      const scrollHeight = Math.ceil(el.scrollHeight);
      const height = Math.max(rectHeight, scrollHeight, 56);
      getCurrentWindow().setSize(new LogicalSize(640, height)).catch(() => {});
    });
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
    if (!window.__TAURI_INTERNALS__) return;

    const chunkListener = listen<SearchChunkPayload>("search-results-chunk", (event) => {
      const payload = event.payload;
      if (payload.request_id !== activeSearchRequestRef.current) return;
      if (payload.timed_out_providers?.length) {
        setTimedOutProviders(payload.timed_out_providers);
      }
      if (payload.items.length > 0) {
        setResults((current) =>
          mergeSearchResults(current, payload.items).slice(0, searchLimitRef.current),
        );
      }
      if (payload.done) {
        setLoading(false);
      }
    });
    const errorListener = listen<SearchErrorPayload>("search-results-error", (event) => {
      if (event.payload.request_id !== activeSearchRequestRef.current) return;
      setTimedOutProviders([]);
      setLoading(false);
    });

    return () => {
      chunkListener.then((fn) => fn());
      errorListener.then((fn) => fn());
    };
  }, [setLoading]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    async function refreshSearchBackend() {
      try {
        const info = await invoke<SearchBackendInfo>("cmd_dispatch", {
          route: "search.backend",
          payload: null,
        });
        setSearchBackend(info);
      } catch {
        setSearchBackend(null);
      }
    }

    void refreshSearchBackend();
    const unlisten = listen<ConfigReloadedPayload>("config-reloaded", (event) => {
      if (
        event.payload.changed_keys.length === 0 ||
        event.payload.changed_keys.includes("search.backend")
      ) {
        void refreshSearchBackend();
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
    if (!window.__TAURI_INTERNALS__) return;
    const result = results[selected];
    if (!result || metadataByPath[result.path]) return;
    if (result.kind !== "file" && result.kind !== "folder" && result.kind !== "app") return;

    let cancelled = false;
    dispatch<SearchMetadata>("search.metadata", {
      path: result.path,
      kind: result.kind,
    })
      .then((metadata) => {
        if (cancelled) return;
        setMetadataByPath((current) => ({ ...current, [metadata.path]: metadata }));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
    // dispatch is intentionally omitted because useIPC returns a fresh wrapper each render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected, results, metadataByPath]);

  useEffect(() => {
    const unlisten = listen<void>("window-focused", () => {
      if (modeRef.current === "terminal") return;
      activeSearchRequestRef.current = "";
      setQuery("");
      setResults([]);
      setSelected(0);
      setCmdResult(null);
      setTimedOutProviders([]);
      void dispatch("search.cancel").catch(() => {});
      inputRef.current?.focus();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [dispatch, setQuery]);

  // 工作區切換：載入切換後的 query 並重置 UI 狀態
  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlisten = listen<WorkspaceState>("workspace-switched", (event) => {
      const ws = event.payload;
      setQuery(ws.query ?? "");
      setResults([]);
      setSelected(0);
      setCmdResult(null);
      setTimedOutProviders([]);
      activeSearchRequestRef.current = "";
      void dispatch("search.cancel").catch(() => {});
      requestAnimationFrame(() => inputRef.current?.focus());
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [dispatch, setQuery]);

  // ESC handler — registered once; reads always-current values via refs
  // so there is no stale-closure race between setCmdResult and effect re-run.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      if (isEditorTerminalResult(cmdResultRef.current)) return;
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();

      if (modeRef.current === "terminal") {
        containerRef.current?.focus();
        setQuery("");
        requestAnimationFrame(() => inputRef.current?.focus());
        void keepLauncherOpen();
        return;
      }

      if (cmdResultRef.current !== null) {
        activeSearchRequestRef.current = "";
        setCmdResult(null);
        setQuery("");
        setResults([]);
        setTimedOutProviders([]);
        void dispatch("search.cancel").catch(() => {});
        requestAnimationFrame(() => inputRef.current?.focus());
      } else if (queryRef.current !== "") {
        activeSearchRequestRef.current = "";
        setQuery("");
        setResults([]);
        setTimedOutProviders([]);
        void dispatch("search.cancel").catch(() => {});
        inputRef.current?.focus();
      } else {
        void hideWindow();
      }
    }
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setQuery]); // stable Zustand setter — listener registered exactly once

  // Window sizing
  useEffect(() => {
    scheduleWindowResize();
  }, [
    scheduleWindowResize,
    mode,
    query,
    results.length,
    cmdSuggestions.length,
    cmdResult,
    argSuggestions.length,
  ]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const el = containerRef.current;
    if (!el) return;

    const observer = new ResizeObserver(() => scheduleWindowResize());
    observer.observe(el);

    const mutationObserver = new MutationObserver(() => scheduleWindowResize());
    mutationObserver.observe(el, {
      attributes: true,
      childList: true,
      subtree: true,
    });

    scheduleWindowResize();

    return () => {
      observer.disconnect();
      mutationObserver.disconnect();
      if (resizeRafRef.current !== null) {
        cancelAnimationFrame(resizeRafRef.current);
        resizeRafRef.current = null;
      }
    };
  }, [scheduleWindowResize]);

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
      setTimedOutProviders([]);
      activeSearchRequestRef.current = "";
      if (debounceRef.current) clearTimeout(debounceRef.current);
      void dispatch("search.cancel").catch(() => {});
      return;
    }
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      const reqId = ++searchIdRef.current;
      const requestId = `search-${reqId}`;
      activeSearchRequestRef.current = requestId;
      setLoading(true);
      setTimedOutProviders([]);
      try {
        const data = await dispatch<SearchResult[]>("search.query", {
          query: ri,
          limit: searchLimitRef.current,
          stream: true,
          request_id: requestId,
          first_batch_limit: Math.min(20, searchLimitRef.current),
        });
        if (reqId !== searchIdRef.current) return;
        setResults(data);
        setSelected(0);
        if (data.length >= searchLimitRef.current) {
          setLoading(false);
        }
      } catch {
        if (reqId === searchIdRef.current) {
          setResults([]);
          setLoading(false);
        }
      }
    }, 200);
  }

  async function launchResult(result: SearchResult) {
    try {
      if (result.primary_action) {
        const actionResult = await dispatch<{ type: string; name?: string; initial_args?: string }>(
          "action.run",
          { action_ref: result.primary_action },
        );
        if (actionResult.type === "panel" && actionResult.name) {
          setCmdResult({
            text: actionResult.initial_args ?? "",
            ui_type: { type: "Panel", value: actionResult.name },
          });
          return;
        }
      } else {
        await dispatch("launcher.launch", { path: result.path });
      }
    } finally {
      if (result.kind === "app" || result.kind === "file" || result.kind === "folder") {
        setQuery("");
        setResults([]);
        void hideWindow();
      }
    }
  }

  async function runFirstSecondary(result: SearchResult) {
    if (!result.primary_action || !result.secondary_action_count) return;
    const actions = await dispatch<SecondaryAction[]>("action.list_secondary", {
      action_ref: result.primary_action,
    });
    const first = actions[0];
    if (first) {
      await dispatch("action.run", { action_ref: first.action_ref });
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
      else if (e.key === "Enter") {
        e.preventDefault();
        const r = results[selected];
        if (r) {
          if (e.shiftKey) void runFirstSecondary(r);
          else void launchResult(r);
        }
      }
    } else if (mode === "command") {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (isArgsPhase) setSelectedArg((i) => Math.min(i + 1, argSuggestions.length - 1));
        else setSelectedCmd((i) => Math.min(i + 1, cmdSuggestions.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        if (isArgsPhase) {
          setSelectedArg((i) => Math.max(i - 1, 0));
        } else {
          // At index 0 or already unselected (-1): deselect all (visual cursor
          // returns to the input bar). Prevents the 0↔-1 oscillation that
          // occurred when Math.max(-2, 0) kept snapping back to 0.
          setSelectedCmd((i) => (i <= 0 ? -1 : i - 1));
        }
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

  const liveTranslationPanel =
    mode === "command" && cmdName === "tr" && spaceIdx !== -1 && !cmdResult;

  // Resolve panel component if cmd result is a Panel type, or stream /tr args live.
  const activePanelName =
    cmdResult?.ui_type.type === "Panel"
      ? (cmdResult.ui_type.value ?? "")
      : liveTranslationPanel
        ? "translation"
        : "";
  const PanelComponent = activePanelName ? (PanelRegistry[activePanelName] ?? null) : null;
  const panelInitialArgs =
    cmdResult?.ui_type.type === "Panel"
      ? cmdResult.text
      : liveTranslationPanel
        ? cmdArgs
        : "";
  const terminalLaunchSpec =
    cmdResult?.ui_type.type === "Terminal" ? cmdResult.ui_type.value : null;
  const panelKey = `${cmdResult ? "command" : "live"}:${activePanelName}`;
  const selectedResult = results[selected] ?? null;
  const selectedMetadata = selectedResult ? metadataByPath[selectedResult.path] : null;
  const searchFooterHint =
    timedOutProviders.length > 0
      ? `Timed out: ${timedOutProviders.join(", ")}`
      : selectedMetadata?.preview
        ? selectedMetadata.preview
        : selectedMetadata?.size_bytes !== undefined
          ? `${selectedMetadata.size_bytes.toLocaleString()} bytes`
          : "↑↓ 選擇";

  const terminalOnExit = () => {
    // Move focus to container first so terminal becoming display:none
    // doesn't shift focus to document.body and risk hiding the window
    containerRef.current?.focus();
    setQuery("");
    requestAnimationFrame(() => inputRef.current?.focus());
    // Keep window open asynchronously — fire and forget
    void keepLauncherOpen();
  };

  const terminalCommandOnExit = useCallback(() => {
    containerRef.current?.focus();
    setCmdResult(null);
    setQuery("");
    setResults([]);
    requestAnimationFrame(() => inputRef.current?.focus());
    void keepLauncherOpen();
  }, [setQuery]);

  // BUG-12: passed to every panel so Escape inside textarea/input can close the panel
  const handlePanelClose = useCallback(() => {
    setCmdResult(null);
    setQuery("");
    setResults([]);
    requestAnimationFrame(() => inputRef.current?.focus());
  }, [setQuery]);

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
              || liveTranslationPanel
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
            {searchBackend && mode === "search" && (
              <span
                title={`configured=${searchBackend.configured}, everything=${searchBackend.everything_available}, tantivy=${searchBackend.tantivy_available}, cache=${searchBackend.file_cache_entries}`}
                className="mr-2 hidden shrink-0 rounded border border-gray-700/70 bg-gray-950/70 px-2 py-1 text-[10px] font-semibold uppercase text-gray-400 sm:inline-flex"
              >
                {searchBackend.active}
              </span>
            )}
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
                      ref={(el) => { if (i === selected && el) el.scrollIntoView({ block: "nearest" }); }}
                      onMouseDown={() => void launchResult(r)}
                      onMouseEnter={() => setSelected(i)}
                      className={`flex items-center gap-2 px-4 py-2.5 cursor-pointer text-sm transition-colors ${
                        i === selected ? "bg-blue-600/70 text-white" : "text-gray-300 hover:bg-white/8"
                      }`}
                    >
                      <span className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${badge.cls}`}>
                        {badge.label}
                      </span>
                      <span className="truncate font-medium">{r.title ?? r.name}</span>
                      {Boolean(r.secondary_action_count) && (
                        <span className="shrink-0 text-[10px] text-gray-500">+{r.secondary_action_count}</span>
                      )}
                      {r.kind !== "app" && (
                        <span className="ml-auto shrink-0 max-w-[220px] truncate text-xs text-gray-500">
                          {r.subtitle ?? r.path}
                        </span>
                      )}
                    </li>
                  );
                })}
              </ul>
              <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
                <span className="min-w-0 max-w-[320px] truncate">{searchFooterHint}</span><span>Enter 開啟</span><span>Shift+Enter 次要動作</span>
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

          {/* Terminal command result */}
          {terminalLaunchSpec && (
            <Suspense fallback={<div className="h-[360px] bg-gray-900/95 rounded-b-xl" />}>
              <TerminalPanel
                isActive={true}
                onExit={terminalCommandOnExit}
                launchSpec={terminalLaunchSpec}
              />
            </Suspense>
          )}

          {/* Panel command result */}
          {PanelComponent && (
            <Suspense fallback={<div className="h-16 bg-gray-900/95 rounded-b-xl" />}>
              <PanelComponent
                key={panelKey}
                onClose={handlePanelClose}
                initialArgs={panelInitialArgs}
              />
            </Suspense>
          )}
        </div>
      </div>
    </div>
  );
}
