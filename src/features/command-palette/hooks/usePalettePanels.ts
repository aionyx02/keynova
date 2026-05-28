// REF.2.P5 — Derived panel + terminal launch spec.
//
// The command palette can host:
//   - a Panel ui_type result from a builtin command (rendered in
//     `<PanelComponent />` via PanelRegistry lookup),
//   - a Terminal ui_type result (rendered with a per-launch Suspense),
//   - or a live translation preview when the user is typing `/tr <args>`
//     before pressing Enter (mode === "command" && cmdName === "tr").
//
// All five outputs are pure derivations from the input; collapsing them
// into one hook keeps the lookup table close to the panel registry.

import { PanelRegistry } from "../../../components/panel/PanelRegistry";
import type { BuiltinCommandResult } from "../../../hooks/useCommands";
import type { TerminalLaunchSpec } from "../../../types/terminal";

interface Deps {
  mode: "search" | "command" | "terminal";
  cmdName: string;
  cmdArgs: string;
  spaceIdx: number;
  cmdResult: BuiltinCommandResult | null;
}

interface UsePalettePanels {
  liveTranslationPanel: boolean;
  activePanelName: string;
  PanelComponent: (typeof PanelRegistry)[keyof typeof PanelRegistry] | null;
  panelInitialArgs: string;
  terminalLaunchSpec: TerminalLaunchSpec | null;
  panelKey: string;
}

export function usePalettePanels({
  mode,
  cmdName,
  cmdArgs,
  spaceIdx,
  cmdResult,
}: Deps): UsePalettePanels {
  const liveTranslationPanel =
    mode === "command" && cmdName === "tr" && spaceIdx !== -1 && !cmdResult;

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
  const terminalLaunchSpec: TerminalLaunchSpec | null =
    cmdResult?.ui_type.type === "Terminal" ? cmdResult.ui_type.value : null;
  const panelKey = `${cmdResult ? "command" : "live"}:${activePanelName}:${panelInitialArgs}`;

  return {
    liveTranslationPanel,
    activePanelName,
    PanelComponent,
    panelInitialArgs,
    terminalLaunchSpec,
    panelKey,
  };
}
