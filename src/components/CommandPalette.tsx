import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, Suspense } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useIPC } from "../hooks/useIPC";
import { useWindowResize } from "../hooks/useWindowResize";
import { useSearchMetadata } from "../hooks/useSearchMetadata";
import { useAppStore } from "../stores/appStore";
import { parseInputMode } from "../hooks/useInputMode";
import { useCommands } from "../hooks/useCommands";
import { CommandSuggestions } from "../features/command-palette/CommandSuggestions";
import { PanelRegistry } from "./panel/PanelRegistry";
import { WorkspaceIndicator } from "./WorkspaceIndicator";
import { SecondaryActionMenu } from "../features/command-palette/SecondaryActionMenu";
import { CheatsheetOverlay } from "./CheatsheetOverlay";
import { FilterChips, clearLegacyFilters, loadFilters } from "../features/command-palette/FilterChips";
import { useRecentlyDeleted } from "../features/command-palette/hooks/useRecentlyDeleted";
import { usePipeline } from "../features/command-palette/hooks/usePipeline";
import { useSearchStream } from "../features/command-palette/hooks/useSearchStream";
import { useCopyHint } from "../features/command-palette/hooks/useCopyHint";
import { useSearchBackend } from "../features/command-palette/hooks/useSearchBackend";
import { useLauncherSettings } from "../features/command-palette/hooks/useLauncherSettings";
import { useWorkspaceLifecycle } from "../features/command-palette/hooks/useWorkspaceLifecycle";
import {
  OnboardingTour,
  hasCompletedOnboarding,
  resetOnboarding,
} from "./OnboardingTour";
import { PreviewPane } from "./PreviewPane";
import { RankTooltip } from "./RankTooltip";
import { useFilePreview, isPreviewable } from "../hooks/useFilePreview";
import { PALETTE_WIDTH_NARROW, PALETTE_WIDTH_WIDE } from "../hooks/useWindowResize";
import {
  basenameFromPath,
  buildSecondaryActions,
  parentDirFromPath,
  type SecondaryActionId,
} from "../utils/secondaryActions";
import { IPC } from "../ipc/routes";
import type { SearchResult, SourceFilter } from "../types/search";
import type { ActionRef } from "../types/search";
import type { BuiltinCommandResult } from "../hooks/useCommands";

const TerminalPanel = React.lazy(() =>
  import("./TerminalPanel").then((m) => ({ default: m.TerminalPanel })),
);

interface SecondaryAction {
  action_ref: ActionRef;
  label: string;
  risk: "low" | "medium" | "high";
}

function isEditorTerminalResult(result: BuiltinCommandResult | null) {
  return result?.ui_type.type === "Terminal" && result.ui_type.value.editor;
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

function hasEncodingError(s: string | undefined | null): boolean {
  return typeof s === "string" && s.includes("�");
}


function isCopyableLocationResult(result: SearchResult | null) {
  return result?.kind === "app" || result?.kind === "file" || result?.kind === "folder";
}

function isCopyShortcut(e: React.KeyboardEvent) {
  return (e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === "c";
}

export function CommandPalette() {
  const { dispatch } = useIPC();
  const { query, setQuery, setLoading } = useAppStore();
  const { all, filtered, runCommand, suggestArgs } = useCommands();

  const {
    results,
    setResults,
    selected,
    setSelected,
    timedOutProviders,
    fileDiagnostics,
    triggerSearch,
    cancelSearch,
    setSearchLimit,
    clearResults: clearSearchResults,
  } = useSearchStream({ dispatch, setLoading });

  // Command mode state
  const [selectedCmd, setSelectedCmd] = useState(0);
  const [cmdResult, setCmdResult] = useState<BuiltinCommandResult | null>(null);

  // Mount terminal once and keep it alive; only toggle visibility via CSS
  const [terminalMounted, setTerminalMounted] = useState(false);

  // Arg suggestions state (shown when user types space after an exact command match)
  const [argSuggestions, setArgSuggestions] = useState<string[]>([]);
  const [selectedArg, setSelectedArg] = useState(0);
  const { searchBackend } = useSearchBackend({ dispatch });
  const {
    copiedPath,
    copyHint,
    flashCopiedPath,
    flashCopyHint,
    clear: clearCopyHint,
  } = useCopyHint();
  const {
    pipelineResult,
    pipelineRunning,
    runPipeline,
    clear: clearPipeline,
  } = usePipeline({ dispatch });

  // LAUNCH.1.A — Secondary action menu state (keyboard-driven via onKeyDown below)
  const [secondaryMenuOpen, setSecondaryMenuOpen] = useState(false);
  const [menuFocusedIndex, setMenuFocusedIndex] = useState(0);
  const [expandedMetadata, setExpandedMetadata] = useState(false);

  // LAUNCH.1.B — two-phase confirm gate for destructive file ops.
  // pendingConfirm: set after a dry-run dispatch; next Enter on the same action runs for real.
  // inlineInput: open for rename/move (need a target name/path).
  const [pendingConfirm, setPendingConfirm] = useState<SecondaryActionId | null>(null);
  const [inlineInput, setInlineInput] = useState<{ for: "rename" | "move"; value: string } | null>(null);

  // LAUNCH.1.D — source-type filter chips (multi-select, session-scoped).
  // Initial set is always empty: persistence across launches caused users to
  // silently hide entire result kinds without realising why. See FilterChips
  // module comment for context.
  const [activeFilters, setActiveFilters] = useState<Set<SourceFilter>>(() => loadFilters());
  // Remove any legacy persisted chip selection from earlier builds so existing
  // users aren't left stuck with hidden results until they manually click Clear.
  useEffect(() => {
    clearLegacyFilters();
  }, []);

  // LAUNCH.1.C/E — settings as state (consumed during render to gate UI).
  const { previewEnabled, showRankBreakdown } = useLauncherSettings({
    dispatch,
    onMaxResultsChange: setSearchLimit,
  });

  // LAUNCH.1.C — dynamic palette width: 640 normally, 960 when preview pane visible.
  const paletteWidthRef = useRef<number>(PALETTE_WIDTH_NARROW);

  // LAUNCH.1.E — rank tooltip hover state. `rect` is captured at the moment the
  // hover delay fires so subsequent renders can read it without touching a ref.
  const [hover, setHover] = useState<{ index: number; rect: DOMRect } | null>(null);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ONBOARD.1.A — first-run tour overlay state.
  const [onboardingOpen, setOnboardingOpen] = useState(() => !hasCompletedOnboarding());

  // ONBOARD.1.B — `?` cheatsheet overlay (only triggers from Shift+/ when input
  // is empty or non-search mode; otherwise typing `?` flows into the input).
  const [cheatsheetOpen, setCheatsheetOpen] = useState(false);

  // Bug B kill-set extracted to `useRecentlyDeleted`. The hook keeps render-time
  // deleted-path semantics + 30 s TTL; see its module comment for full rationale.
  // Destructure to stable callback identities so effect deps don't re-register.
  const {
    markDeleted: markPathDeleted,
    clear: clearRecentlyDeleted,
    isDeleted: isPathRecentlyDeleted,
  } = useRecentlyDeleted();

  const inputRef = useRef<HTMLInputElement>(null);
  // Bug-fix 2026-05-19 (round 2) — throttle for keepLauncherOpen() renewal
  // on every keystroke. Backend launcher_focus_guard TTL is 2s; we refresh
  // it at most every 200ms while user is interacting → guard always wins
  // against the 1.5s Focused(false) grace, regardless of English/IME path.
  const lastGuardRef = useRef<number>(0);
  const argDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const { mode, rawInput } = parseInputMode(query);

  // Refs always hold the latest values — read inside the ESC handler
  // to avoid stale closures when the listener is registered only once.
  const modeRef = useRef(mode);
  const cmdResultRef = useRef(cmdResult);
  const queryRef = useRef(query);
  const secondaryMenuOpenRef = useRef(secondaryMenuOpen);
  const expandedMetadataRef = useRef(expandedMetadata);
  useLayoutEffect(() => {
    modeRef.current = mode;
    cmdResultRef.current = cmdResult;
    queryRef.current = query;
    secondaryMenuOpenRef.current = secondaryMenuOpen;
    expandedMetadataRef.current = expandedMetadata;
  });

  const { containerRef, scheduleWindowResize } = useWindowResize(modeRef, cmdResultRef, paletteWidthRef);
  const { metadataByPath, iconsByKey } = useSearchMetadata(results, selected);


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
    return () => {
      if (hoverTimerRef.current) {
        clearTimeout(hoverTimerRef.current);
      }
    };
  }, []);


  // Chunk + error listeners moved into useSearchStream; their bookkeeping
  // (activeSearchRequestRef equality, replace/merge dispatch, done → setLoading)
  // is identical inside the hook.


  useEffect(() => {
    if (mode !== "terminal") {
      const raf = requestAnimationFrame(() => { inputRef.current?.focus(); });
      return () => cancelAnimationFrame(raf);
    }
  }, [mode]);

  // window-focused / workspace-switched / workspace-cycled handled by hook;
  // see its module comment for the per-event reset behavior.
  useWorkspaceLifecycle({
    modeRef,
    inputRef,
    setQuery,
    setCmdResult,
    clearSearchResults,
    cancelSearch,
    clearRecentlyDeleted,
  });

  // ESC handler — registered once; reads always-current values via refs
  // so there is no stale-closure race between setCmdResult and effect re-run.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      if (isEditorTerminalResult(cmdResultRef.current)) return;
      e.preventDefault();
      e.stopPropagation();
      e.stopImmediatePropagation();

      // Highest priority: close secondary menu / collapse metadata first
      if (secondaryMenuOpenRef.current) {
        setSecondaryMenuOpen(false);
        setMenuFocusedIndex(0);
        setPendingConfirm(null);
        setInlineInput(null);
        return;
      }
      if (expandedMetadataRef.current) {
        setExpandedMetadata(false);
        return;
      }

      if (modeRef.current === "terminal") {
        containerRef.current?.focus();
        setQuery("");
        requestAnimationFrame(() => inputRef.current?.focus());
        void keepLauncherOpen();
        return;
      }

      if (cmdResultRef.current !== null) {
        cancelSearch();
        setCmdResult(null);
        clearPipeline();
        setQuery("");
        clearSearchResults();
        requestAnimationFrame(() => inputRef.current?.focus());
      } else if (queryRef.current !== "") {
        cancelSearch();
        setQuery("");
        clearSearchResults();
        clearPipeline();
        inputRef.current?.focus();
      } else {
        void hideWindow();
      }
    }
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setQuery]); // stable Zustand setter — listener registered exactly once

  // Close secondary menu / collapse metadata when result selection or list changes.
  useEffect(() => {
    if (secondaryMenuOpenRef.current) {
      setSecondaryMenuOpen(false);
      setMenuFocusedIndex(0);
      setPendingConfirm(null);
      setInlineInput(null);
    }
    if (expandedMetadataRef.current) {
      setExpandedMetadata(false);
    }
  }, [results, selected]);

  // Window sizing
  useEffect(() => {
    scheduleWindowResize();
  }, [
    scheduleWindowResize,
    mode,
    query,
    results.length,
    activeFilters,
    cmdSuggestions.length,
    cmdResult,
    argSuggestions.length,
    expandedMetadata,
    secondaryMenuOpen,
    menuFocusedIndex,
    pendingConfirm,
    inlineInput,
  ]);


  function handleQueryChange(value: string) {
    setQuery(value);
    setCmdResult(null);
    clearCopyHint();
    clearPipeline();
    setSelectedCmd(0);
    setSelectedArg(0);
    setArgSuggestions([]);
    // LAUNCH.1.B bugfix — typing a new query enters a fresh search context;
    // suppress-list is no longer relevant.
    clearRecentlyDeleted();
    const { mode: newMode, rawInput: ri } = parseInputMode(value);
    // Mount terminal on first "> " entry; avoids useEffect setState cascade
    if (newMode === "terminal") setTerminalMounted(true);
    if (newMode !== "search" || ri.trim() === "") {
      clearSearchResults();
      cancelSearch();
      return;
    }
    triggerSearch(ri);
  }

  async function launchResult(result: SearchResult) {
    try {
      await dispatch(IPC.SEARCH_RECORD_SELECTION, {
        source: result.source ?? result.kind,
        path: result.path,
      }).catch(() => {});
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
        await dispatch(IPC.LAUNCHER_LAUNCH, { path: result.path });
      }
    } finally {
      if (result.kind === "app" || result.kind === "file" || result.kind === "folder") {
        setQuery("");
        setResults([]);
        void hideWindow();
      }
    }
  }

  async function copyResultLocation(result: SearchResult) {
    if (!isCopyableLocationResult(result)) return;
    try {
      await navigator.clipboard.writeText(result.path);
      flashCopiedPath(result.path);
    } catch {
      clearCopyHint();
    }
  }

  async function copyText(text: string, hint: string) {
    try {
      await navigator.clipboard.writeText(text);
      flashCopyHint(hint);
    } catch {
      clearCopyHint();
    }
  }

  async function revealResult(result: SearchResult) {
    if (!result.path) return;
    try {
      await revealItemInDir(result.path);
    } catch {
      // Plugin failure (e.g. missing permission, non-existent path) — surface in footer.
      flashCopyHint("Reveal failed", 1500);
    }
  }

  function closeSecondaryMenu() {
    setSecondaryMenuOpen(false);
    setMenuFocusedIndex(0);
    setPendingConfirm(null);
    setInlineInput(null);
  }

  /**
   * LAUNCH.1.B bug-fix — after delete/rename/move succeeds the original path
   * must vanish from the result list. Strategy:
   *
   * Add the path to `recentlyDeleted` state. The `visibleResults` derivation
   * filters by this set on every render, so it doesn't matter which code
   * path (initial debounce response, stream chunk, merge…) reintroduces the
   * path: the row simply cannot render.
   *
   * No re-fire of the search — Windows Everything still indexes Recycle Bin
   * entries, so re-firing would resurrect the trashed file in the chunk
   * stream. The render-time kill set is the only authoritative gate.
   *
   * Cleared on next query change / workspace switch.
   */
  function refreshAfterFileMutation(droppedPath: string) {
    markPathDeleted(droppedPath);
    setSelected(0);
  }

  function showHint(message: string, durationMs = 1500) {
    flashCopyHint(message, durationMs);
  }

  async function handleSecondaryAction(id: SecondaryActionId, result: SearchResult) {
    switch (id) {
      case "reveal":
        await revealResult(result);
        closeSecondaryMenu();
        break;
      case "copy_path":
        await copyResultLocation(result);
        if (!isCopyableLocationResult(result) && result.path) {
          // Notes / non-fs items still expose `path`; copy raw.
          await copyText(result.path, `Copied: ${result.path}`);
        }
        closeSecondaryMenu();
        break;
      case "copy_name": {
        const name = result.name || basenameFromPath(result.path);
        await copyText(name, `Copied name: ${name}`);
        closeSecondaryMenu();
        break;
      }
      case "show_metadata":
        setExpandedMetadata(true);
        closeSecondaryMenu();
        break;
      case "open_with":
        try {
          await dispatch(IPC.FILE_OPEN_WITH, { path: result.path });
          showHint(`Opened: ${basenameFromPath(result.path)}`);
        } catch (err) {
          showHint(`Open failed: ${(err as Error).message}`, 2500);
        }
        closeSecondaryMenu();
        break;
      case "open_as_text":
        try {
          await dispatch(IPC.FILE_OPEN_AS_TEXT, { path: result.path });
          showHint("Opened as text");
        } catch (err) {
          showHint(`Open failed: ${(err as Error).message}`, 2500);
        }
        closeSecondaryMenu();
        break;
      case "hash":
        try {
          showHint("Hashing…", 60_000);
          const res = await dispatch<{ hex: string; bytes: number }>(IPC.FILE_HASH, {
            path: result.path,
          });
          await navigator.clipboard.writeText(res.hex);
          showHint(`SHA-256 copied: ${res.hex.slice(0, 12)}… (${res.bytes} bytes)`, 2500);
        } catch (err) {
          showHint(`Hash failed: ${(err as Error).message}`, 2500);
        }
        closeSecondaryMenu();
        break;
      case "rename": {
        if (!inlineInput || inlineInput.for !== "rename") {
          setInlineInput({ for: "rename", value: basenameFromPath(result.path) });
          setPendingConfirm(null);
          return;
        }
        const confirm = pendingConfirm === "rename";
        try {
          const res = await dispatch<{ preview?: boolean; target?: string }>(IPC.FILE_RENAME, {
            path: result.path,
            new_name: inlineInput.value,
            confirm,
          });
          if (!confirm) {
            showHint(`⚠ Press Enter again to rename → ${res.target ?? inlineInput.value} · Esc cancel`, 4000);
            setPendingConfirm("rename");
          } else {
            showHint(`Renamed to ${inlineInput.value}`);
            closeSecondaryMenu();
            refreshAfterFileMutation(result.path);
          }
        } catch (err) {
          showHint(`Rename failed: ${(err as Error).message}`, 3000);
          setPendingConfirm(null);
        }
        break;
      }
      case "move": {
        if (!inlineInput || inlineInput.for !== "move") {
          setInlineInput({ for: "move", value: parentDirFromPath(result.path) });
          setPendingConfirm(null);
          return;
        }
        const confirm = pendingConfirm === "move";
        try {
          const res = await dispatch<{ preview?: boolean; target?: string }>(IPC.FILE_MOVE, {
            path: result.path,
            target_dir: inlineInput.value,
            confirm,
          });
          if (!confirm) {
            showHint(`⚠ Press Enter again to move → ${res.target ?? inlineInput.value} · Esc cancel`, 4000);
            setPendingConfirm("move");
          } else {
            showHint(`Moved to ${inlineInput.value}`);
            closeSecondaryMenu();
            refreshAfterFileMutation(result.path);
          }
        } catch (err) {
          showHint(`Move failed: ${(err as Error).message}`, 3000);
          setPendingConfirm(null);
        }
        break;
      }
      case "delete": {
        const confirm = pendingConfirm === "delete";
        try {
          const res = await dispatch<{
            preview?: boolean;
            size?: number | null;
            kind?: string;
            destination?: string;
          }>(IPC.FILE_DELETE, { path: result.path, confirm });
          if (!confirm) {
            const sizeStr = res.size != null ? `${res.size} bytes` : "unknown size";
            const name = result.name || basenameFromPath(result.path);
            showHint(
              `⚠ Press Enter again to delete ${name} · ${sizeStr} → ${res.destination ?? "recycle bin"} · Esc cancel`,
              4000,
            );
            setPendingConfirm("delete");
          } else {
            // 2026-05-19 Bug B1 — Windows Explorer desktop view does not always
            // re-enumerate on SHCNE_DELETE (OneDrive redirect, Defender scan,
            // multi-instance Explorer). File IS in Recycle Bin; the icon may
            // linger until F5. Hint surfaces that so user is not confused.
            showHint("Moved to recycle bin · Press F5 on desktop if icon lingers", 2500);
            closeSecondaryMenu();
            refreshAfterFileMutation(result.path);
          }
        } catch (err) {
          // 5000ms: backend now returns a longer diagnostic message when trash
          // returns Ok but the file remains on disk (in-use, Recycle Bin
          // disabled, network drive…). Give the user time to read.
          showHint(`Delete failed: ${(err as Error).message}`, 5000);
          setPendingConfirm(null);
        }
        break;
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
      await dispatch(IPC.ACTION_RUN, { action_ref: first.action_ref });
    }
  }

  async function execCommand(name: string, args = "") {
    try {
      // ONBOARD.1.A — `/onboard` re-triggers the tour without going through
      // the builtin command result UI: clear the localStorage flag, open the
      // overlay, and skip rendering a Panel/Inline result.
      if (name === "onboard") {
        resetOnboarding();
        setQuery("");
        setOnboardingOpen(true);
        return;
      }
      const result = await runCommand(name, args);
      setCmdResult(result);
    } catch {
      // ignore
    }
  }

  function onKeyDown(e: React.KeyboardEvent) {
    // Bug-fix 2026-05-19 (round 2) — Bug A真根因：英文打字也會觸發
    // WindowEvent::Focused(false) blip。每次 keydown 都 renew guard (200ms
    // throttle) → backend grace 期內 guard 必有效 → 不 hide。
    // eslint-disable-next-line react-hooks/purity -- event handler, not render path
    const now = Date.now();
    if (now - lastGuardRef.current > 200) {
      lastGuardRef.current = now;
      void keepLauncherOpen();
    }
    // ONBOARD.1.B — `?` opens cheatsheet when input is empty, so it doesn't
    // collide with typing `?` as part of a query.
    if (e.key === "?" && query === "" && !secondaryMenuOpen && !cmdResult) {
      e.preventDefault();
      setCheatsheetOpen(true);
      return;
    }
    if (mode === "search") {
      // Menu open: route arrows/Enter/Left to menu actions; let typed text fall through.
      if (secondaryMenuOpen) {
        const r = visibleResults[safeSelected] ?? null;
        const enabled = r ? buildSecondaryActions(r).filter((it) => !it.disabled) : [];
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setMenuFocusedIndex((i) => Math.min(i + 1, Math.max(enabled.length - 1, 0)));
          return;
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          setMenuFocusedIndex((i) => Math.max(i - 1, 0));
          return;
        }
        if (e.key === "Tab") {
          e.preventDefault();
          setMenuFocusedIndex((i) => {
            const next = e.shiftKey ? i - 1 : i + 1;
            const max = Math.max(enabled.length - 1, 0);
            return Math.min(Math.max(next, 0), max);
          });
          return;
        }
        if (e.key === "Enter") {
          e.preventDefault();
          const item = enabled[menuFocusedIndex];
          if (item && r) void handleSecondaryAction(item.id, r);
          return;
        }
        if (e.key === "ArrowLeft") {
          // Only close menu if input caret is at start; otherwise let cursor move.
          const target = e.currentTarget as HTMLInputElement;
          if (target.selectionStart === 0 && target.selectionEnd === 0) {
            e.preventDefault();
            closeSecondaryMenu();
            return;
          }
        }
        // Other keys fall through to input.
      }
      if (isCopyShortcut(e)) {
        const target = e.currentTarget as HTMLInputElement;
        if (target.selectionStart !== target.selectionEnd) return;
        const r = visibleResults[safeSelected] ?? null;
        if (isCopyableLocationResult(r)) {
          e.preventDefault();
          void copyResultLocation(r);
          return;
        }
      }
      // Open secondary menu when cursor at end of input and a result is selected.
      if (!secondaryMenuOpen && (e.key === "ArrowRight" || e.key === "Tab")) {
        const target = e.currentTarget as HTMLInputElement;
        const atEnd = target.selectionStart === target.value.length
          && target.selectionEnd === target.value.length;
        const r = visibleResults[safeSelected] ?? null;
        if (atEnd && r) {
          e.preventDefault();
          setSecondaryMenuOpen(true);
          setMenuFocusedIndex(0);
          return;
        }
      }
      if (e.key === "ArrowDown") { e.preventDefault(); setSelected((i) => Math.min(i + 1, visibleResults.length - 1)); }
      else if (e.key === "ArrowUp") { e.preventDefault(); setSelected((i) => Math.max(i - 1, 0)); }
      else if (e.key === "Enter") {
        e.preventDefault();
        if (query.includes("|")) {
          void runPipeline(query);
          return;
        }
        const r = visibleResults[safeSelected];
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

  // LAUNCH.1.D + LAUNCH.1.B bugfix — derived view:
  //   1. chip filter (file/note/app/command/history/model);
  //   2. recently-deleted kill set with 30 s TTL so trashed paths can't
  //      reappear from Everything's Recycle-Bin index via a streaming chunk
  //      or stale response. TTL means a user-initiated restore from Recycle
  //      Bin starts being searchable again ~30 s later, even without a
  //      workspace switch / query change.
  // Filter is O(n) over a small list capped at launcher.max_results; no
  // useMemo needed and react-hooks/preserve-manual-memoization complains
  // when we add one.
  const visibleResults: SearchResult[] = results.filter((r) => {
    if (isPathRecentlyDeleted(r.path)) return false;
    if (activeFilters.size === 0) return true;
    if (r.kind === "folder") return activeFilters.has("file");
    return activeFilters.has(r.kind as SourceFilter);
  });

  // Read-side clamp so a stale `selected` (e.g. after filter toggle shrinks the
  // list) doesn't index past the array. User input via ArrowDown/Up already
  // clamps against `visibleResults.length` so no state-sync effect is needed.
  const safeSelected = visibleResults.length === 0
    ? 0
    : Math.min(selected, visibleResults.length - 1);

  const hasResults = visibleResults.length > 0 && mode === "search";
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
  const panelKey = `${cmdResult ? "command" : "live"}:${activePanelName}:${panelInitialArgs}`;
  const selectedResult = visibleResults[safeSelected] ?? null;
  const selectedMetadata = selectedResult ? metadataByPath[selectedResult.path] : null;

  // LAUNCH.1.C — preview pane only renders for previewable kinds. The palette
  // window expands to PALETTE_WIDTH_WIDE while this is true and snaps back
  // otherwise; useWindowResize reads `paletteWidthRef.current` each tick.
  const showPreview =
    previewEnabled &&
    !!selectedResult &&
    (selectedResult.kind === "file" ||
      selectedResult.kind === "folder" ||
      selectedResult.kind === "note") &&
    mode === "search" &&
    visibleResults.length > 0;

  useEffect(() => {
    paletteWidthRef.current = showPreview ? PALETTE_WIDTH_WIDE : PALETTE_WIDTH_NARROW;
    scheduleWindowResize();
  }, [showPreview, scheduleWindowResize]);

  const { previewByPath } = useFilePreview(visibleResults, safeSelected);
  const previewForSelected = selectedResult ? previewByPath[selectedResult.path] : undefined;
  const previewLoading = isPreviewable(selectedResult) && !previewForSelected;
  const menuItems = useMemo(
    () => (selectedResult ? buildSecondaryActions(selectedResult) : []),
    [selectedResult],
  );
  const fileSearchDiagHint = (() => {
    if (!fileDiagnostics) return null;
    const d = fileDiagnostics;
    if (d.timed_out) return `File search: provider timed out after 800ms`;
    if (d.fallback_reason) return `File search: ${d.fallback_reason}`;
    const hidden = d.pre_balance_count - d.returned_count;
    if (hidden > 0) return `File search: ${d.returned_count} shown, ${hidden} hidden by display limit`;
    return null;
  })();

  const searchFooterHint = copyHint
    ? copyHint
    : copiedPath
      ? `Copied path: ${copiedPath}`
      : timedOutProviders.length > 0 && !fileDiagnostics
        ? `Timed out: ${timedOutProviders.join(", ")}`
        : fileSearchDiagHint ?? (
            selectedMetadata?.preview
              ? selectedMetadata.preview
              : selectedMetadata?.size_bytes !== undefined
                ? `${selectedMetadata.size_bytes.toLocaleString()} bytes`
                : "↑↓ 選擇"
          );

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
    // containerRef is a stable ref object from useWindowResize — omitting is intentional.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [setQuery]);

  const handlePanelCommandResult = useCallback((result: BuiltinCommandResult) => {
    setCmdResult(result);
    setResults([]);
    cancelSearch();
  }, [setResults, cancelSearch]);

  // BUG-12: passed to every panel so Escape inside textarea/input can close the panel
  const handlePanelClose = useCallback(() => {
    setCmdResult(null);
    setQuery("");
    setResults([]);
    requestAnimationFrame(() => inputRef.current?.focus());
  }, [setQuery, setResults]);

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
              || liveTranslationPanel || pipelineRunning || pipelineResult
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
              // Bug-fix 2026-05-19 (round 2) — switched from
              // onCompositionStart/Update/End (IME-only, did not cover English
              // typing reported by user) to onFocus + per-keydown throttle
              // (lastGuardRef) inside onKeyDown above. onFocus covers initial
              // mount + post-Esc re-focus; throttled onKeyDown covers every
              // subsequent keystroke regardless of IME state.
              onFocus={() => void keepLauncherOpen()}
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

          {/* ONBOARD.1.C — empty-state CTA: search mode, non-empty query, zero raw results */}
          {mode === "search" && query.trim() !== "" && results.length === 0 && !pipelineRunning && !pipelineResult && (
            <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
              <div className="px-4 py-3 text-sm text-gray-400">
                <div className="mb-2">No results for <span className="font-mono text-gray-300">{query}</span>.</div>
                <div className="flex flex-wrap gap-2 text-xs">
                  <button
                    type="button"
                    onClick={() => void execCommand("note", `create ${query}`)}
                    className="rounded bg-sky-600/30 px-2 py-1 text-sky-200 ring-1 ring-sky-600/40 hover:bg-sky-600/50"
                  >
                    Create note &quot;{query}&quot;
                  </button>
                  <button
                    type="button"
                    onClick={() => { setQuery("/help"); }}
                    className="rounded bg-gray-800/60 px-2 py-1 text-gray-300 ring-1 ring-gray-700/40 hover:bg-gray-800"
                  >
                    Try /help
                  </button>
                  <button
                    type="button"
                    onClick={() => { setQuery("/setting"); }}
                    className="rounded bg-gray-800/60 px-2 py-1 text-gray-300 ring-1 ring-gray-700/40 hover:bg-gray-800"
                  >
                    Open /setting
                  </button>
                  <button
                    type="button"
                    onClick={() => void execCommand("onboard")}
                    className="rounded bg-gray-800/60 px-2 py-1 text-gray-300 ring-1 ring-gray-700/40 hover:bg-gray-800"
                  >
                    Replay /onboard
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Search results — show chip bar whenever raw results exist so the user can always clear filters */}
          {mode === "search" && results.length > 0 && visibleResults.length === 0 && (
            <div className="relative bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
              <FilterChips active={activeFilters} onChange={setActiveFilters} />
              <div className="px-4 py-3 text-sm text-gray-400">
                Filter hides all {results.length} results.
                <button
                  type="button"
                  onClick={() => setActiveFilters(new Set())}
                  className="ml-2 text-sky-300 hover:text-sky-200 underline"
                >
                  Clear filter
                </button>
              </div>
            </div>
          )}
          {hasResults && (
            <div className={`bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl ${secondaryMenuOpen ? "overflow-visible" : "overflow-hidden"}`}>
              <FilterChips active={activeFilters} onChange={setActiveFilters} />
              <div className={showPreview ? "grid grid-cols-[1fr_320px]" : ""}>
                {/* Left column — result list + secondary menu overlay anchored here */}
                <div className={`relative min-w-0 ${secondaryMenuOpen ? "min-h-[384px]" : ""}`}>
                  <ul className="max-h-[352px] overflow-y-auto py-1">
                    {visibleResults.map((r, i) => {
                      const badge = KIND_BADGE[r.kind] ?? KIND_BADGE.file;
                      const icon = r.icon_key ? iconsByKey[r.icon_key] : null;
                      return (
                        <li
                          key={r.path}
                          ref={(el) => { if (i === safeSelected && el) el.scrollIntoView({ block: "nearest" }); }}
                          onMouseDown={() => void launchResult(r)}
                          onMouseEnter={(e) => {
                            setSelected(i);
                            if (!showRankBreakdown) return;
                            const rect = e.currentTarget.getBoundingClientRect();
                            if (hoverTimerRef.current) clearTimeout(hoverTimerRef.current);
                            hoverTimerRef.current = setTimeout(() => {
                              setHover({ index: i, rect });
                            }, 400);
                          }}
                          onMouseLeave={() => {
                            if (hoverTimerRef.current) {
                              clearTimeout(hoverTimerRef.current);
                              hoverTimerRef.current = null;
                            }
                            setHover(null);
                          }}
                          className={`flex items-center gap-2 px-4 py-2.5 cursor-pointer text-sm transition-colors ${
                            i === safeSelected ? "bg-blue-600/70 text-white" : "text-gray-300 hover:bg-white/8"
                          }`}
                        >
                          {icon ? (
                            <img
                              src={icon.data_url}
                              alt=""
                              className="h-6 w-6 shrink-0 rounded"
                              draggable={false}
                            />
                          ) : (
                            <span className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide ${badge.cls}`}>
                              {badge.label}
                            </span>
                          )}
                          <span className={`truncate font-medium${hasEncodingError(r.title ?? r.name) ? " text-gray-500 italic" : ""}`}>
                            {hasEncodingError(r.title ?? r.name) ? "(無法解碼的名稱)" : (r.title ?? r.name)}
                          </span>
                          {Boolean(r.secondary_action_count) && (
                            <span className="shrink-0 text-[10px] text-gray-500">+{r.secondary_action_count}</span>
                          )}
                          {r.kind !== "app" && (
                            <span className="ml-auto shrink-0 max-w-[220px] truncate text-xs text-gray-500">
                              {hasEncodingError(r.subtitle ?? r.path) ? "(無法解碼的路徑)" : (r.subtitle ?? r.path)}
                            </span>
                          )}
                        </li>
                      );
                    })}
                  </ul>

                  {/* LAUNCH.1.A — Secondary action menu overlay (keyboard-driven, anchored to left column) */}
                  {secondaryMenuOpen && selectedResult && menuItems.length > 0 && (
                    <SecondaryActionMenu
                      result={selectedResult}
                      items={menuItems}
                      focusedIndex={menuFocusedIndex}
                      onSelect={(id) => void handleSecondaryAction(id, selectedResult)}
                      onHoverEnabled={setMenuFocusedIndex}
                      pendingConfirmId={pendingConfirm}
                      inlineInput={inlineInput}
                      onInlineInputChange={(value) =>
                        setInlineInput((prev) => (prev ? { ...prev, value } : prev))
                      }
                      onInlineInputKeyDown={(e) => {
                        if (e.key === "Enter") {
                          e.preventDefault();
                          e.stopPropagation();
                          if (inlineInput && selectedResult) {
                            void handleSecondaryAction(inlineInput.for, selectedResult);
                          }
                        } else if (e.key === "Escape") {
                          e.preventDefault();
                          e.stopPropagation();
                          setInlineInput(null);
                          setPendingConfirm(null);
                        }
                      }}
                    />
                  )}
                </div>

                {/* LAUNCH.1.C — right preview column */}
                {showPreview && (
                  <div className={`${secondaryMenuOpen ? "h-[384px]" : "max-h-[352px]"} border-l border-gray-700/50 bg-gray-950/40`}>
                    <PreviewPane
                      result={selectedResult}
                      preview={previewForSelected}
                      loading={previewLoading}
                    />
                  </div>
                )}
              </div>

              {/* LAUNCH.1.A — Show metadata expanded view (toggled from action menu, spans both columns) */}
              {expandedMetadata && selectedResult && (
                <div className="border-t border-gray-700/50 px-4 py-2 text-xs text-gray-400 bg-gray-950/60">
                  <div className="mb-1 flex items-center justify-between">
                    <span className="text-[10px] uppercase tracking-wider text-gray-500">Metadata</span>
                    <span className="text-[10px] text-gray-600">Esc 收起</span>
                  </div>
                  <div className="grid grid-cols-[max-content_1fr] gap-x-3 gap-y-0.5 font-mono">
                    <span className="text-gray-500">path</span>
                    <span className="truncate text-gray-300">{selectedResult.path}</span>
                    {selectedMetadata?.size_bytes !== undefined && (
                      <>
                        <span className="text-gray-500">size</span>
                        <span className="text-gray-300">{selectedMetadata.size_bytes.toLocaleString()} bytes</span>
                      </>
                    )}
                    {selectedMetadata?.modified_ms !== undefined && (
                      <>
                        <span className="text-gray-500">modified</span>
                        <span className="text-gray-300">{new Date(selectedMetadata.modified_ms).toLocaleString()}</span>
                      </>
                    )}
                    {selectedMetadata?.is_dir !== undefined && (
                      <>
                        <span className="text-gray-500">type</span>
                        <span className="text-gray-300">{selectedMetadata.is_dir ? "folder" : "file"}</span>
                      </>
                    )}
                    {selectedMetadata?.preview && (
                      <>
                        <span className="text-gray-500">preview</span>
                        <span className="text-gray-300 whitespace-pre-wrap break-words">{selectedMetadata.preview}</span>
                      </>
                    )}
                  </div>
                </div>
              )}

              <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between gap-2">
                <span className="min-w-0 flex-1 truncate">{searchFooterHint}</span>
                <span className="shrink-0">Enter 開啟</span>
                <span className="shrink-0">Shift+Enter 次要</span>
                <span className="shrink-0">→ Actions</span>
              </div>
            </div>
          )}

          {/* LAUNCH.1.E — rank tooltip (rendered last so it overlays everything) */}
          <RankTooltip
            breakdown={hover ? visibleResults[hover.index]?.score_breakdown : undefined}
            anchorRect={hover ? hover.rect : null}
            visible={hover !== null}
          />

          {/* ONBOARD.1.A — first-run tour overlay (conditional mount resets step) */}
          {onboardingOpen && (
            <OnboardingTour onClose={() => setOnboardingOpen(false)} />
          )}

          {/* ONBOARD.1.B — `?` cheatsheet overlay */}
          {cheatsheetOpen && (
            <CheatsheetOverlay onClose={() => setCheatsheetOpen(false)} />
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
                onRunCommandResult={handlePanelCommandResult}
              />
            </Suspense>
          )}

          {/* Pipeline running indicator */}
          {pipelineRunning && (
            <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl px-4 py-3">
              <span className="text-sm text-blue-400 animate-pulse">Pipeline running…</span>
            </div>
          )}

          {/* Pipeline execution result */}
          {!pipelineRunning && pipelineResult && (
            <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl overflow-hidden">
              <ul className="py-1">
                {pipelineResult.actions.map((stage) => (
                  <li key={stage.index} className="flex items-start gap-2 px-4 py-1.5 text-sm">
                    <span className={`shrink-0 font-mono text-xs mt-0.5 ${
                      stage.status === "completed" ? "text-emerald-400" : "text-red-400"
                    }`}>
                      {stage.status === "completed" ? "✓" : "✗"}
                    </span>
                    <span className="font-mono text-gray-400 shrink-0">{stage.route}</span>
                    {stage.error && (
                      <span className="text-red-400 truncate">{stage.error}</span>
                    )}
                  </li>
                ))}
                {pipelineResult.log.status === "failed" && pipelineResult.log.error && pipelineResult.actions.length === 0 && (
                  <li className="px-4 py-1.5 text-sm text-red-400">{pipelineResult.log.error}</li>
                )}
              </ul>
              <div className="border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600 flex justify-between">
                <span className={pipelineResult.log.status === "completed" ? "text-emerald-600" : "text-red-600"}>
                  {pipelineResult.log.status}
                </span>
                <span>{pipelineResult.log.action_count} stages</span>
                <span>Esc 清除</span>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
