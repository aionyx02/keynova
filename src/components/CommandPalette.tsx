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
import { hasCompletedOnboarding } from "../shared/components/onboarding-state";
import { RankTooltip } from "../shared/components/RankTooltip";
import { PALETTE_WIDTH_NARROW } from "../hooks/useWindowResize";
import type { SourceFilter } from "../types/search";
import type { BuiltinCommandResult } from "../hooks/useCommands";
import { unifiedToLegacy } from "../utils/search";
import { usePaletteMode } from "../features/command-palette/hooks/usePaletteMode";
import { useCapabilityStream } from "../features/ai-capability/hooks/useCapabilityStream";
import { useCapabilityRunState } from "../features/ai-capability/hooks/useCapabilityRunState";
import { useGenCommand } from "../features/ai-capability/hooks/useGenCommand";
import { useSuggestNext } from "../features/ai-capability/hooks/useSuggestNext";
import type { CapabilitySurfaceMode } from "../features/command-palette/CapabilityResultArea";
import { CapabilityHintLine } from "../features/command-palette/CapabilityHintLine";
import { classifyNlIntent } from "../features/command-palette/utils/classifyNlIntent";
import type { TerminalLaunchSpec } from "../types/terminal";

const TerminalPanel = React.lazy(() =>
  import("../features/terminal/TerminalPanel").then((m) => ({ default: m.TerminalPanel })),
);
// PERF.1 — Lazy-mount the rarely-used surfaces so the main bundle keeps only
// the always-on palette skeleton. CapabilityResultArea pulls react-markdown
// transitively when its answer card renders, so deferring it saves the most
// memory; OnboardingTour and CheatsheetOverlay only mount on first-run /
// `?` press.
const CapabilityResultArea = React.lazy(() =>
  import("../features/command-palette/CapabilityResultArea").then((m) => ({
    default: m.CapabilityResultArea,
  })),
);
const OnboardingTour = React.lazy(() =>
  import("../shared/components/OnboardingTour").then((m) => ({ default: m.OnboardingTour })),
);
const CheatsheetOverlay = React.lazy(() =>
  import("../shared/components/CheatsheetOverlay").then((m) => ({ default: m.CheatsheetOverlay })),
);
const EMPTY_CAPABILITY_ARGS: Record<string, never> = {};

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

function makeLaunchId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return `launch-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function buildGeneratedCommandLaunchSpec(command: string): TerminalLaunchSpec {
  const platform =
    typeof navigator !== "undefined"
      ? `${navigator.platform ?? ""} ${navigator.userAgent ?? ""}`.toLowerCase()
      : "";
  const isWindows = platform.includes("win");
  if (isWindows) {
    return {
      launch_id: makeLaunchId(),
      program: "powershell.exe",
      args: ["-NoLogo", "-Command", command],
      title: command,
      editor: false,
    };
  }
  return {
    launch_id: makeLaunchId(),
    program: "/bin/sh",
    args: ["-lc", command],
    title: command,
    editor: false,
  };
}

export function CommandPalette() {
  const { dispatch } = useIPC();
  const { query, setQuery, setLoading, isLoading } = useAppStore();
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

  // REF.6.A — palette wire format is `UnifiedResult[]`; existing hooks
  // (useDerivedView, useFileActions, useKeyboardNav, useSearchMetadata,
  // useFilePreview, etc.) still consume the legacy `SearchResult` shape.
  // Derive the legacy view once per `results` change and pass it down;
  // `SearchResultsList` and the inline-AI surface consume the canonical
  // `UnifiedResult[]` directly.
  const legacyResults = React.useMemo(() => results.map(unifiedToLegacy), [results]);

  // Command mode state
  const [selectedCmd, setSelectedCmd] = useState(0);
  const [cmdResult, setCmdResult] = useState<BuiltinCommandResult | null>(null);
  const [capabilitySuggestionSelected, setCapabilitySuggestionSelected] = useState(0);

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
  const { previewEnabled, showRankBreakdown, showCapabilityHint } = useLauncherSettings({
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

  const paletteMode = usePaletteMode(query);
  const explicitCapabilityMode = paletteMode.kind === "capability" ? paletteMode : null;
  const trimmedQuery = query.trim();
  const [smartNextDismissed, setSmartNextDismissed] = useState(false);
  const [smartCommandDismissedKey, setSmartCommandDismissedKey] = useState<string | null>(null);
  const [stableSmartCommandQuery, setStableSmartCommandQuery] = useState("");

  useEffect(() => {
    if (mode !== "search" || explicitCapabilityMode !== null || trimmedQuery === "") return;
    const timer = window.setTimeout(() => setStableSmartCommandQuery(trimmedQuery), 260);
    return () => window.clearTimeout(timer);
  }, [explicitCapabilityMode, mode, trimmedQuery]);

  const showSmartNext =
    explicitCapabilityMode === null &&
    mode === "search" &&
    trimmedQuery === "" &&
    cmdResult === null &&
    !pipelineRunning &&
    !pipelineResult &&
    !smartNextDismissed;
  // REF.6.J - resolve which capability (if any) should auto-surface for the
  // current non-result NL query. `null` means no smart card; otherwise the
  // returned id picks the card variant. The dismissal key intentionally keys
  // on the trimmed query so a different query gets a fresh chance to surface.
  const smartIntentMatch =
    explicitCapabilityMode === null &&
    mode === "search" &&
    trimmedQuery !== "" &&
    cmdResult === null &&
    !pipelineRunning &&
    !pipelineResult &&
    !isLoading &&
    results.length === 0 &&
    stableSmartCommandQuery === trimmedQuery &&
    smartCommandDismissedKey !== trimmedQuery
      ? classifyNlIntent(trimmedQuery)
      : null;
  const capabilityMode: CapabilitySurfaceMode | null = (() => {
    if (
      explicitCapabilityMode?.id === "explain" ||
      explicitCapabilityMode?.id === "summarize" ||
      explicitCapabilityMode?.id === "fix"
    ) {
      return {
        id: explicitCapabilityMode.id,
        args: explicitCapabilityMode.args,
        source: "prefix",
      };
    }
    if (explicitCapabilityMode?.id === "cmd") {
      return {
        id: "cmd",
        args: explicitCapabilityMode.args,
        source: "prefix",
      };
    }
    if (explicitCapabilityMode?.id === "next") {
      return {
        id: "next",
        args: EMPTY_CAPABILITY_ARGS,
        source: "prefix",
      };
    }
    if (showSmartNext) {
      return {
        id: "next",
        args: EMPTY_CAPABILITY_ARGS,
        source: "smart",
      };
    }
    if (smartIntentMatch) {
      if (smartIntentMatch.id === "cmd") {
        return {
          id: "cmd",
          args: { text: smartIntentMatch.text },
          source: "smart",
        };
      }
      return {
        id: smartIntentMatch.id,
        args: { text: smartIntentMatch.text },
        source: "smart",
      };
    }
    return null;
  })();
  const textCapabilityMode: {
    id: "explain" | "summarize" | "fix";
    args: { text: string };
  } | null =
    capabilityMode?.id === "explain" ||
    capabilityMode?.id === "summarize" ||
    capabilityMode?.id === "fix"
      ? { id: capabilityMode.id, args: capabilityMode.args }
      : null;
  const commandCapabilityMode: { id: "cmd"; args: { text: string } } | null =
    capabilityMode?.id === "cmd" ? { id: capabilityMode.id, args: capabilityMode.args } : null;
  const nextCapabilityMode: { id: "next"; args: Record<string, never> } | null =
    capabilityMode?.id === "next" ? { id: capabilityMode.id, args: capabilityMode.args } : null;
  const capabilityStream = useCapabilityStream({
    dispatch,
    id: textCapabilityMode
      ? textCapabilityMode.id === "fix"
        ? "fix_error"
        : textCapabilityMode.id
      : "explain",
    args: textCapabilityMode ? textCapabilityMode.args : null,
  });
  const genCommand = useGenCommand({ dispatch });
  const genCommandState = useCapabilityRunState({
    active: commandCapabilityMode !== null,
    argsKey: commandCapabilityMode?.args.text ?? null,
    isLoading: genCommand.isLoading,
    error: genCommand.error,
    run: () =>
      genCommand.run({
        intent: commandCapabilityMode?.args.text ?? "",
        ctx: {},
      }),
    cancelInner: genCommand.cancel,
  });
  const suggestNext = useSuggestNext({ dispatch });
  const suggestNextState = useCapabilityRunState({
    active: nextCapabilityMode !== null,
    argsKey: nextCapabilityMode ? "next" : null,
    autoSubmit: true,
    isLoading: suggestNext.isLoading,
    error: suggestNext.error,
    run: () => suggestNext.run({ ctx: { limit: 5 } }),
    cancelInner: suggestNext.cancel,
  });
  const activeCapabilityLoading =
    (textCapabilityMode !== null &&
      (capabilityStream.status === "pending" || capabilityStream.status === "streaming")) ||
    (commandCapabilityMode !== null && genCommand.isLoading) ||
    (nextCapabilityMode !== null && suggestNext.isLoading);

  const {
    modeRef,
    cmdResultRef,
    queryRef,
    secondaryMenuOpenRef,
    expandedMetadataRef,
    capabilityModeRef,
    capabilityStreamingRef,
  } = usePaletteRefs({
    mode,
    cmdResult,
    query,
    secondaryMenuOpen,
    expandedMetadata,
    capabilityMode: capabilityMode !== null,
    capabilityStreaming: activeCapabilityLoading,
  });

  const { containerRef, scheduleWindowResize } = useWindowResize(
    modeRef,
    cmdResultRef,
    paletteWidthRef,
  );
  const { metadataByPath, iconsByKey } = useSearchMetadata(legacyResults, selected);

  // Split rawInput into command name and trailing args (Minecraft-style).
  const spaceIdx = rawInput.search(/\s/);
  const cmdName = spaceIdx === -1 ? rawInput : rawInput.slice(0, spaceIdx);
  const cmdArgs = spaceIdx === -1 ? "" : rawInput.slice(spaceIdx + 1).trim();

  const { exactCmd, isArgsPhase, argSuggestions, setArgSuggestions, selectedArg, setSelectedArg } =
    useArgSuggestions({ mode, cmdName, cmdArgs, spaceIdx, all, suggestArgs });
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
    capabilityModeRef,
    capabilityStreamingRef,
    onCapabilityCancel:
      textCapabilityMode !== null
        ? capabilityStream.cancel
        : commandCapabilityMode !== null
          ? genCommandState.cancel
          : suggestNextState.cancel,
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

  const { launchResult, copyResultLocation, handleSecondaryAction, runFirstSecondary } =
    useFileActions({
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

  const handleInputQueryChange = React.useCallback(
    (value: string) => {
      setSmartNextDismissed(false);
      setSmartCommandDismissedKey(null);
      void handleQueryChange(value);
    },
    [handleQueryChange, setSmartCommandDismissedKey, setSmartNextDismissed],
  );

  const clearCapabilityQuery = React.useCallback(() => {
    handleQueryChange("");
  }, [handleQueryChange]);

  const editGeneratedCommand = React.useCallback(
    (command: string) => {
      setCmdResult(null);
      setSmartCommandDismissedKey(command.trim());
      handleQueryChange(command);
      requestAnimationFrame(() => inputRef.current?.select());
    },
    [handleQueryChange, setSmartCommandDismissedKey],
  );

  const runGeneratedCommand = React.useCallback(
    (command: string) => {
      clearCapabilityQuery();
      setCmdResult({
        text: command,
        ui_type: {
          type: "Terminal",
          value: buildGeneratedCommandLaunchSpec(command),
        },
      });
    },
    [clearCapabilityQuery],
  );

  const runSuggestedWorkflow = React.useCallback(
    (index: number) => {
      const item = suggestNext.data[index];
      if (!item?.replay) return;
      if (item.replay.route !== "cmd.run") return;
      const name = typeof item.replay.payload.name === "string" ? item.replay.payload.name : "";
      const args = typeof item.replay.payload.args === "string" ? item.replay.payload.args : "";
      if (!name) return;
      clearCapabilityQuery();
      void execCommand(name, args);
    },
    [clearCapabilityQuery, execCommand, suggestNext.data],
  );

  const closeCapabilitySurface = React.useCallback(() => {
    if (explicitCapabilityMode?.id === "next") {
      setSmartNextDismissed(true);
    }
    if (capabilityMode?.source === "smart" && capabilityMode.id === "next") {
      setSmartNextDismissed(true);
      return;
    }
    // REF.6.J - any smart-surfaced capability (cmd / explain / summarize / fix)
    // dismisses on the same trimmed-query key so the user does not lose their
    // typed input when they close the card.
    if (capabilityMode?.source === "smart" && capabilityMode.id !== "next") {
      setSmartCommandDismissedKey(trimmedQuery);
      return;
    }
    clearCapabilityQuery();
  }, [
    capabilityMode,
    clearCapabilityQuery,
    explicitCapabilityMode?.id,
    setSmartCommandDismissedKey,
    setSmartNextDismissed,
    trimmedQuery,
  ]);

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
    results: legacyResults,
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

  // REF.6.A — parallel `UnifiedResult[]` for `SearchResultsList` chip
  // rendering. Filter the canonical `results` via `unified_id` set so the
  // visible rows stay 1-to-1 with the legacy filtered view (kill-set +
  // active filters).
  const visibleUnified = React.useMemo(() => {
    const keep = new Set(
      visibleResults.map((r) => r.unified_id).filter((id): id is string => typeof id === "string"),
    );
    return results.filter((u) => keep.has(u.id));
  }, [results, visibleResults]);

  const safeCapabilitySuggestionSelected =
    nextCapabilityMode === null
      ? 0
      : Math.min(capabilitySuggestionSelected, Math.max(suggestNext.data.length - 1, 0));

  const { liveTranslationPanel, PanelComponent, panelInitialArgs, terminalLaunchSpec, panelKey } =
    usePalettePanels({ mode, cmdName, cmdArgs, spaceIdx, cmdResult });

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

  const { terminalOnExit, terminalCommandOnExit, handlePanelCommandResult, handlePanelClose } =
    useTerminalControl({
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
    capabilityMode: capabilityMode !== null,
    capabilityListMode: nextCapabilityMode !== null,
    capabilityListCount: suggestNext.data.length,
    capabilityListSelected: safeCapabilitySuggestionSelected,
    setCapabilityListSelected: setCapabilitySuggestionSelected,
    onCapabilitySubmit:
      commandCapabilityMode !== null ? genCommandState.submit : capabilityStream.submit,
    onCapabilityRunSelected: () => runSuggestedWorkflow(safeCapabilitySuggestionSelected),
    keepLauncherOpen,
  });

  const showCapabilityResult = capabilityMode !== null;
  const showCapabilityHintLine =
    paletteMode.kind === "search" &&
    mode === "search" &&
    query === "" &&
    showCapabilityHint &&
    !showCapabilityResult;
  const showSearchEmptyState =
    paletteMode.kind === "search" &&
    mode === "search" &&
    query.trim() !== "" &&
    results.length === 0 &&
    !pipelineRunning &&
    !pipelineResult &&
    !showCapabilityResult;
  const showEmptyFilterState =
    paletteMode.kind === "search" &&
    mode === "search" &&
    results.length > 0 &&
    visibleResults.length === 0;
  const hasPaletteContentBelow = Boolean(
    hasResults ||
    hasCmdSuggestions ||
    hasArgSuggestions ||
    cmdResult ||
    isArgsPhase ||
    liveTranslationPanel ||
    pipelineRunning ||
    pipelineResult ||
    showCapabilityResult ||
    showCapabilityHintLine ||
    showSearchEmptyState ||
    showEmptyFilterState,
  );

  return (
    <div ref={containerRef} tabIndex={-1} className="w-full outline-none">
      {/* Terminal: mounted once on first visit, hidden via CSS when not active */}
      {terminalMounted && (
        <div style={{ display: mode === "terminal" ? "block" : "none" }}>
          <Suspense fallback={<div className="kn-terminal-shell h-[520px]" />}>
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
            onQueryChange={(value) => void handleInputQueryChange(value)}
            onKeyDown={onKeyDown}
            onFocus={() => void keepLauncherOpen()}
            searchBackend={searchBackend}
            hasContentBelow={hasPaletteContentBelow}
          />

          {capabilityMode && (
            <Suspense
              fallback={
                <div className="kn-panel-shell h-[120px] rounded-t-none border-t-0" />
              }
            >
              <CapabilityResultArea
                key={capabilityMode.id}
                mode={capabilityMode}
                answerStream={capabilityStream}
                commandCard={{
                  status: genCommandState.status,
                  data: genCommand.data,
                  error: genCommand.error,
                  startedAtMs: genCommandState.startedAtMs,
                  completedAtMs: genCommandState.completedAtMs,
                  riskRequiresConfirmation: Boolean(genCommand.risk?.requires_confirmation),
                  onSubmit: genCommandState.submit,
                  onCancel: genCommandState.cancel,
                  onEditBefore: editGeneratedCommand,
                  onRun: runGeneratedCommand,
                }}
                listCard={{
                  status: suggestNextState.status,
                  items: suggestNext.data,
                  error: suggestNext.error,
                  startedAtMs: suggestNextState.startedAtMs,
                  completedAtMs: suggestNextState.completedAtMs,
                  selectedIndex: safeCapabilitySuggestionSelected,
                  onSelectIndex: setCapabilitySuggestionSelected,
                  onRunSelected: runSuggestedWorkflow,
                  onCancel: suggestNextState.cancel,
                }}
                dispatch={dispatch}
                onClose={closeCapabilitySurface}
              />
            </Suspense>
          )}

          {/* REF.6.B — capability prefix discovery hint, shown on empty
              palette so first-time users see the available prefixes. */}
          {showCapabilityHintLine && <CapabilityHintLine visible={showCapabilityHint} />}

          {showSearchEmptyState && (
            <EmptyStateCTA
              query={query}
              onCreateNote={() => void execCommand("note", `create ${query}`)}
              onJumpHelp={() => {
                setQuery("/help");
              }}
              onJumpSetting={() => {
                setQuery("/setting");
              }}
              onReplayOnboard={() => void execCommand("onboard")}
            />
          )}

          {showEmptyFilterState && (
            <EmptyFilterState
              activeFilters={activeFilters}
              onChangeFilters={setActiveFilters}
              totalResults={results.length}
            />
          )}
          {paletteMode.kind === "search" && hasResults && (
            <SearchResultsList
              visibleResults={visibleResults}
              unifiedVisible={visibleUnified}
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

          {/* ONBOARD.1.A — first-run tour overlay (conditional mount resets step).
              PERF.1: lazy-loaded; while the chunk arrives, the overlay simply
              does not appear yet — acceptable since the tour is informational
              and the trigger (first-run) is non-time-critical. */}
          {onboardingOpen && (
            <Suspense fallback={null}>
              <OnboardingTour onClose={() => setOnboardingOpen(false)} />
            </Suspense>
          )}

          {/* ONBOARD.1.B — `?` cheatsheet overlay, lazy-loaded; user-triggered
              overlay tolerates a one-frame delay before the chunk paints. */}
          {cheatsheetOpen && (
            <Suspense fallback={null}>
              <CheatsheetOverlay onClose={() => setCheatsheetOpen(false)} />
            </Suspense>
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
              onSelect={(arg, i) => {
                setQuery(`/${cmdName} ${arg} `);
                setSelectedArg(i);
              }}
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
