import React, { Suspense } from "react";

import { PanelRegistry } from "../../components/panel/PanelRegistry";
import type { BuiltinCommandResult, CommandMeta } from "../../hooks/useCommands";
import type { TerminalLaunchSpec } from "../../types/terminal";

const TerminalPanel = React.lazy(() =>
  import("../terminal/TerminalPanel").then((module) => ({ default: module.TerminalPanel })),
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
        <div className="border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-2 text-[11px] text-[color:var(--kn-text-muted)]">
          <span className="font-semibold text-[color:var(--kn-accent)]">/{exactCmd.name}</span>
          {exactCmd.args_hint && (
            <span className="ml-2 font-mono text-[color:var(--kn-text-faint)]">
              {exactCmd.args_hint}
            </span>
          )}
          <span className="ml-3">Tab complete / Enter run</span>
        </div>
      )}

      {cmdResult?.ui_type.type === "Inline" && cmdResult.text && (
        <div className="kn-panel-shell rounded-t-none border-t-0 px-4 py-4">
          <pre className="whitespace-pre-wrap text-sm leading-7 text-[color:var(--kn-text-soft)]">
            {cmdResult.text}
          </pre>
        </div>
      )}

      {terminalLaunchSpec && (
        <Suspense
          fallback={<div className="kn-terminal-shell h-[520px] rounded-t-none border-t-0" />}
        >
          <TerminalPanel
            isActive={true}
            onExit={onTerminalCommandExit}
            launchSpec={terminalLaunchSpec}
            attached
          />
        </Suspense>
      )}

      {PanelComponent && (
        <Suspense fallback={<div className="kn-panel-shell h-16 rounded-t-none border-t-0" />}>
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
