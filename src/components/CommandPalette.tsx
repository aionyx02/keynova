import React, { useEffect, useRef, useState, Suspense } from "react";
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
import { CommandResultArea } from "../features/command-palette/CommandResultArea";
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
import { useKeyboardNav } from "../features/command-palette/hooks/useKeyboardNav";
import { useTerminalControl } from "../features/command-palette/hooks/useTerminalControl";
import { useExecCommand } from "../features/command-palette/hooks/useExecCommand";
import { useDerivedView } from "../features/command-palette/hooks/useDerivedView";
import { usePaletteRefs } from "../features/command-palette/hooks/usePaletteRefs";
import { useQueryChange } from "../features/command-palette/hooks/useQueryChange";
import { usePaletteEffects } from "../features/command-palette/hooks/usePaletteEffects";
import { useRankHover } from "../features/command-palette/hooks/useRankHover";
import { OnboardingTour, hasCompletedOnboarding } from "./OnboardingTour";
import { RankTooltip } from "./RankTooltip";
import { PALETTE_WIDTH_NARROW } from "../hooks/useWindowResize";
import type { SourceFilter } from "../types/search";
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

  const { hover, start: startRankHover, end: endRankHover, hoverTimerRef } = useRankHover();

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

  const { mode, rawInput } = parseInputMode(query);

  const { modeRef, cmdResultRef, queryRef, secondaryMenuOpenRef, expandedMetadataRef } =
    usePaletteRefs({ mode, cmdResult, query, secondaryMenuOpen, expandedMetadata });

  const { containerRef, scheduleWindowResize } = useWindowResize(modeRef, cmdResultRef, paletteWidthRef);
  const { metadataByPath, iconsByKey } = useSearchMetadata(results, selected);

  // Split rawInput into command name and trailing args (Minecraft-style).
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

  const handleQueryChange = useQueryChange({
    setQuery,
    setCmdResult,
    clearCopyHint,
    clearPipeline,
    setSelectedCmd,
    setSelectedArg,
    setArgSuggestions,
    clearRecentlyDeleted,
    setTerminalMounted,
    clearSearchResults,
    cancelSearch,
    triggerSearch,
  });

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

  const execCommand = useExecCommand({
    runCommand,
    setQuery,
    setCmdResult,
    setOnboardingOpen,
  });

  const {
    visibleResults,
    safeSelected,
    hasResults,
    hasCmdSuggestions,
    hasArgSuggestions,
    selectedResult,
    selectedMetadata,
    showPreview,
    previewForSelected,
    previewLoading,
    menuItems,
  } = useDerivedView({
    results,
    selected,
    mode,
    activeFilters,
    isPathRecentlyDeleted,
    cmdSuggestionsCount: cmdSuggestions.length,
    cmdResult,
    isArgsPhase,
    argSuggestionsCount: argSuggestions.length,
    previewEnabled,
    metadataByPath,
    secondaryMenuOpenRef,
    expandedMetadataRef,
    closeSecondaryMenu,
    setExpandedMetadata,
  });

  const {
    liveTranslationPanel,
    PanelComponent,
    panelInitialArgs,
    terminalLaunchSpec,
    panelKey,
  } = usePalettePanels({ mode, cmdName, cmdArgs, spaceIdx, cmdResult });

  usePaletteEffects({
    inputRef,
    hoverTimerRef,
    paletteWidthRef,
    showPreview,
    mode,
    scheduleWindowResize,
    query,
    resultsLength: results.length,
    activeFilters,
    cmdSuggestionsLength: cmdSuggestions.length,
    cmdResult,
    argSuggestionsLength: argSuggestions.length,
    expandedMetadata,
    secondaryMenuOpen,
    menuFocusedIndex,
    pendingConfirm,
    inlineInput,
  });

  const searchFooterHint = useSearchFooterHint({
    copyHint,
    copiedPath,
    timedOutProviders,
    fileDiagnostics,
    selectedMetadata,
  });

  const {
    terminalOnExit,
    terminalCommandOnExit,
    handlePanelCommandResult,
    handlePanelClose,
  } = useTerminalControl({
    containerRef,
    inputRef,
    setQuery,
    setCmdResult,
    setResults,
    cancelSearch,
    keepLauncherOpen,
  });

  const { onKeyDown } = useKeyboardNav({
    mode,
    query,
    cmdResult,
    visibleResults,
    safeSelected,
    setSelected,
    secondaryMenuOpen,
    setSecondaryMenuOpen,
    menuFocusedIndex,
    setMenuFocusedIndex,
    closeSecondaryMenu,
    setCheatsheetOpen,
    cmdName,
    cmdArgs,
    cmdSuggestions,
    selectedCmd,
    setSelectedCmd,
    isArgsPhase,
    argSuggestions,
    selectedArg,
    setSelectedArg,
    setQuery,
    copyResultLocation,
    handleSecondaryAction,
    launchResult,
    runFirstSecondary,
    runPipeline,
    execCommand,
    keepLauncherOpen,
  });

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
              onHoverStart={startRankHover}
              onHoverEnd={endRankHover}
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

          {hasArgSuggestions && (
            <ArgsSuggestionsList
              cmdName={cmdName}
              suggestions={argSuggestions}
              selectedIndex={selectedArg}
              onSelect={(arg, i) => { setQuery(`/${cmdName} ${arg} `); setSelectedArg(i); }}
              onHover={setSelectedArg}
            />
          )}

          <CommandResultArea
            exactCmd={exactCmd}
            isArgsPhase={isArgsPhase}
            cmdResult={cmdResult}
            terminalLaunchSpec={terminalLaunchSpec}
            PanelComponent={PanelComponent}
            panelKey={panelKey}
            panelInitialArgs={panelInitialArgs}
            onTerminalCommandExit={terminalCommandOnExit}
            onPanelClose={handlePanelClose}
            onPanelCommandResult={handlePanelCommandResult}
          />

          <PipelineStatusRow running={pipelineRunning} result={pipelineResult} />
        </div>
      </div>
    </div>
  );
}
