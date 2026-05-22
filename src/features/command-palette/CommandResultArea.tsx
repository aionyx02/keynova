// REF.2.P5 — Bundles four mutually-exclusive small render regions that all
// appear below the command-suggestion list:
//
//   1. Args-phase syntax hint bar (`/cmd <args-hint> · Tab 填入 · Enter 執行`)
//   2. Inline command result (a `<pre>` block from `runCommand`)
//   3. Terminal command result (lazy TerminalPanel with a launch spec)
//   4. Panel command result (lazy PanelComponent under PanelRegistry)
//
// They were inline in CommandPalette as four separate JSX islands. Each
// guard is preserved as-is — `terminalLaunchSpec` and `PanelComponent` are
// mutually exclusive in practice because both come from `cmdResult.ui_type`.

import React, { Suspense } from "react";

import { PanelRegistry } from "../../components/panel/PanelRegistry";
import type { BuiltinCommandResult, CommandMeta } from "../../hooks/useCommands";
import type { TerminalLaunchSpec } from "../../types/terminal";

const TerminalPanel = React.lazy(() =>
  import("../../components/TerminalPanel").then((m) => ({ default: m.TerminalPanel })),
);

interface Props {
  exactCmd: CommandMeta | null;
  isArgsPhase: boolean;
  cmdResult: BuiltinCommandResult | null;
  terminalLaunchSpec: TerminalLaunchSpec | null;
  PanelComponent: (typeof PanelRegistry)[keyof typeof PanelRegistry] | null;
  panelKey: string;
  panelInitialArgs: string;
  onTerminalCommandExit: () => void;
  onPanelClose: () => void;
  onPanelCommandResult: (result: BuiltinCommandResult) => void;
}

export function CommandResultArea({
  exactCmd,
  isArgsPhase,
  cmdResult,
  terminalLaunchSpec,
  PanelComponent,
  panelKey,
  panelInitialArgs,
  onTerminalCommandExit,
  onPanelClose,
  onPanelCommandResult,
}: Props) {
  return (
    <>
      {isArgsPhase && exactCmd && !cmdResult && (
        <div className="bg-gray-900/95 backdrop-blur-md px-4 py-1.5 text-xs text-gray-500 border-t border-gray-700/30">
          <span className="text-blue-400">/{exactCmd.name}</span>
          {exactCmd.args_hint && (
            <span className="ml-1 font-mono text-gray-600">{exactCmd.args_hint}</span>
          )}
          <span className="ml-3 text-gray-700">Tab 填入 · Enter 執行</span>
        </div>
      )}

      {cmdResult?.ui_type.type === "Inline" && cmdResult.text && (
        <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl px-4 py-3">
          <pre className="text-sm text-gray-300 whitespace-pre-wrap leading-relaxed">
            {cmdResult.text}
          </pre>
        </div>
      )}

      {terminalLaunchSpec && (
        <Suspense fallback={<div className="h-[360px] bg-gray-900/95 rounded-b-xl" />}>
          <TerminalPanel
            isActive={true}
            onExit={onTerminalCommandExit}
            launchSpec={terminalLaunchSpec}
          />
        </Suspense>
      )}

      {PanelComponent && (
        <Suspense fallback={<div className="h-16 bg-gray-900/95 rounded-b-xl" />}>
          <PanelComponent
            key={panelKey}
            onClose={onPanelClose}
            initialArgs={panelInitialArgs}
            onRunCommandResult={onPanelCommandResult}
          />
        </Suspense>
      )}
    </>
  );
}
