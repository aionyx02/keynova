import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, Suspense } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useIPC } from "../hooks/useIPC";
import { useWindowResize } from "../hooks/useWindowResize";
import { useSearchMetadata } from "../hooks/useSearchMetadata";
import { useAppStore } from "../stores/appStore";
import { parseInputMode } from "../hooks/useInputMode";
import { useCommands } from "../hooks/useCommands";
import { CommandSuggestions } from "../features/command-palette/CommandSuggestions";
import { CheatsheetOverlay } from "./CheatsheetOverlay";
import { clearLegacyFilters, loadFilters } from "../features/command-palette/FilterChips";
import { PaletteInputBar } from "../features/command-palette/PaletteInputBar";
import { EmptyStateCTA } from "../features/command-palette/EmptyStateCTA";
import { EmptyFilterState } from "../features/command-palette/EmptyFilterState";
import { PipelineStatusRow } from "../features/command-palette/PipelineStatusRow";
import { ArgsSuggestionsList } from "../features/command-palette/ArgsSuggestionsList";
import { SearchResultsList } from "../features/command-palette/SearchResultsList";
import { useRecentlyDeleted } from "../features/command-palette/hooks/useRecentlyDeleted";
import { usePipeline } from "../features/command-palette/hooks/usePipeline";
import { useSearchStream } from "../features/command-palette/hooks/useSearchStream";
import { useCopyHint } from "../features/command-palette/hooks/useCopyHint";
import { useSearchBackend } from "../features/command-palette/hooks/useSearchBackend";
import { useLauncherSettings } from "../features/command-palette/hooks/useLauncherSettings";
import { useWorkspaceLifecycle } from "../features/command-palette/hooks/useWorkspaceLifecycle";
import { useSecondaryMenu } from "../features/command-palette/hooks/useSecondaryMenu";
import { useEscapeKey } from "../features/command-palette/hooks/useEscapeKey";
import { useArgSuggestions } from "../features/command-palette/hooks/useArgSuggestions";
import { usePalettePanels } from "../features/command-palette/hooks/usePalettePanels";
import { useSearchFooterHint } from "../features/command-palette/hooks/useSearchFooterHint";
import { useFileActions } from "../features/command-palette/hooks/useFileActions";
import {
  OnboardingTour,
  hasCompletedOnboarding,
  resetOnboarding,
} from "./OnboardingTour";
import { RankTooltip } from "./RankTooltip";
import { useFilePreview, isPreviewable } from "../hooks/useFilePreview";
import { PALETTE_WIDTH_NARROW, PALETTE_WIDTH_WIDE } from "../hooks/useWindowResize";
import { buildSecondaryActions } from "../utils/secondaryActions";
import type { SearchResult, SourceFilter } from "../types/search";
import type { BuiltinCommandResult } from "../hooks/useCommands";

const TerminalPanel = React.lazy(() =>
  import("./TerminalPanel").then((m) => ({ default: m.TerminalPanel })),
);

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

  // LAUNCH.1.A + LAUNCH.1.B — secondary menu state + two-phase confirm gate.
  // pendingConfirm: set after a dry-run dispatch; next Enter on the same action runs for real.
  // inlineInput: open for rename/move (need a target name/path).
  const {
    secondaryMenuOpen,
    setSecondaryMenuOpen,
    menuFocusedIndex,
    setMenuFocusedIndex,
    expandedMetadata,
    setExpandedMetadata,
    pendingConfirm,
    setPendingConfirm,
    inlineInput,
    setInlineInput,
    closeSecondaryMenu,
  } = useSecondaryMenu();

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

  const {
    exactCmd,
    isArgsPhase,
    argSuggestions,
    setArgSuggestions,
    selectedArg,
    setSelectedArg,
  } = useArgSuggestions({ mode, cmdName, cmdArgs, spaceIdx, all, suggestArgs });
  const cmdSuggestions = mode === "command" ? filtered(cmdName) : [];

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

  // ESC priority chain handled by useEscapeKey; see its module comment for the
  // ordered branches it consults.
  useEscapeKey({
    shouldIgnoreEscape: () => isEditorTerminalResult(cmdResultRef.current),
    modeRef,
    cmdResultRef,
    queryRef,
    secondaryMenuOpenRef,
    expandedMetadataRef,
    inputRef,
    containerRef,
    closeSecondaryMenu,
    setExpandedMetadata,
    setCmdResult,
    setQuery,
    clearPipeline,
    clearSearchResults,
    cancelSearch,
    hideWindow,
    keepLauncherOpen,
  });

  // Close secondary menu / collapse metadata when result selection or list changes.
  useEffect(() => {
    if (secondaryMenuOpenRef.current) {
      closeSecondaryMenu();
    }
    if (expandedMetadataRef.current) {
      setExpandedMetadata(false);
    }
  }, [results, selected, closeSecondaryMenu, setExpandedMetadata]);

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

  const {
    launchResult,
    copyResultLocation,
    handleSecondaryAction,
    runFirstSecondary,
  } = useFileActions({
    dispatch,
    flashCopiedPath,
    flashCopyHint,
    clearCopyHint,
    setQuery,
    setResults,
    setCmdResult,
    setSelected,
    markPathDeleted,
    closeSecondaryMenu,
    setExpandedMetadata,
    inlineInput,
    setInlineInput,
    pendingConfirm,
    setPendingConfirm,
  });

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

  const {
    liveTranslationPanel,
    PanelComponent,
    panelInitialArgs,
    terminalLaunchSpec,
    panelKey,
  } = usePalettePanels({ mode, cmdName, cmdArgs, spaceIdx, cmdResult });
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
  const searchFooterHint = useSearchFooterHint({
    copyHint,
    copiedPath,
    timedOutProviders,
    fileDiagnostics,
    selectedMetadata,
  });

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
          <PaletteInputBar
            mode={mode}
            query={query}
            inputRef={inputRef}
            onQueryChange={(value) => void handleQueryChange(value)}
            onKeyDown={onKeyDown}
            onFocus={() => void keepLauncherOpen()}
            searchBackend={searchBackend}
            hasContentBelow={Boolean(
              hasResults
                || hasCmdSuggestions
                || cmdResult
                || isArgsPhase
                || liveTranslationPanel
                || pipelineRunning
                || pipelineResult,
            )}
          />

          {mode === "search" && query.trim() !== "" && results.length === 0 && !pipelineRunning && !pipelineResult && (
            <EmptyStateCTA
              query={query}
              onCreateNote={() => void execCommand("note", `create ${query}`)}
              onJumpHelp={() => { setQuery("/help"); }}
              onJumpSetting={() => { setQuery("/setting"); }}
              onReplayOnboard={() => void execCommand("onboard")}
            />
          )}

          {mode === "search" && results.length > 0 && visibleResults.length === 0 && (
            <EmptyFilterState
              activeFilters={activeFilters}
              onChangeFilters={setActiveFilters}
              totalResults={results.length}
            />
          )}
          {hasResults && (
            <SearchResultsList
              visibleResults={visibleResults}
              safeSelected={safeSelected}
              iconsByKey={iconsByKey}
              onSelectIndex={setSelected}
              onLaunch={(r) => void launchResult(r)}
              showRankBreakdown={showRankBreakdown}
              onHoverStart={(i, rect) => {
                if (hoverTimerRef.current) clearTimeout(hoverTimerRef.current);
                hoverTimerRef.current = setTimeout(() => {
                  setHover({ index: i, rect });
                }, 400);
              }}
              onHoverEnd={() => {
                if (hoverTimerRef.current) {
                  clearTimeout(hoverTimerRef.current);
                  hoverTimerRef.current = null;
                }
                setHover(null);
              }}
              activeFilters={activeFilters}
              onChangeFilters={setActiveFilters}
              secondaryMenuOpen={secondaryMenuOpen}
              selectedResult={selectedResult}
              menuItems={menuItems}
              menuFocusedIndex={menuFocusedIndex}
              pendingConfirm={pendingConfirm}
              inlineInput={inlineInput}
              onSecondaryAction={(id, r) => void handleSecondaryAction(id, r)}
              onMenuFocus={setMenuFocusedIndex}
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
              showPreview={showPreview}
              previewForSelected={previewForSelected}
              previewLoading={previewLoading}
              expandedMetadata={expandedMetadata}
              selectedMetadata={selectedMetadata}
              footerHint={searchFooterHint}
            />
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

          {hasArgSuggestions && (
            <ArgsSuggestionsList
              cmdName={cmdName}
              suggestions={argSuggestions}
              selectedIndex={selectedArg}
              onSelect={(arg, i) => { setQuery(`/${cmdName} ${arg} `); setSelectedArg(i); }}
              onHover={setSelectedArg}
            />
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

          <PipelineStatusRow running={pipelineRunning} result={pipelineResult} />
        </div>
      </div>
    </div>
  );
}
