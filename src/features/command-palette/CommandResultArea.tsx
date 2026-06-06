import React, { Suspense, useState } from "react";

import { PanelRegistry } from "../../components/panel/PanelRegistry";
import { UiIcon } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";
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
  const p = useI18n().palette;
  const [copiedInline, setCopiedInline] = useState(false);
  const showArgsHint = isArgsPhase && exactCmd && !cmdResult && !PanelComponent;

  async function copyInlineResult(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopiedInline(true);
      window.setTimeout(() => setCopiedInline(false), 1200);
    } catch {
      setCopiedInline(false);
    }
  }

  return (
    <>
      {showArgsHint && (
        <div className="border-t border-[color:var(--kn-border)] bg-white/[0.02] px-4 py-2 text-[11px] text-[color:var(--kn-text-muted)]">
          <span className="font-semibold text-[color:var(--kn-accent)]">/{exactCmd.name}</span>
          {exactCmd.args_hint && (
            <span className="ml-2 font-mono text-[color:var(--kn-text-faint)]">
              {exactCmd.args_hint}
            </span>
          )}
          <span className="ml-3">
            Tab {p.complete} / Enter {p.run}
          </span>
        </div>
      )}

      {cmdResult?.ui_type.type === "Inline" && cmdResult.text && (
        <div className="kn-panel-shell rounded-t-none border-t-0 px-4 py-4">
          <div className="mb-3 flex justify-end">
            <button
              type="button"
              onClick={() => void copyInlineResult(cmdResult.text)}
              className="flex h-7 w-7 items-center justify-center rounded-[8px] border border-[color:var(--kn-border)] bg-white/[0.035] text-[color:var(--kn-text-muted)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
              title={copiedInline ? p.copiedCommandResult : p.copyCommandResult}
              aria-label={copiedInline ? p.copiedCommandResult : p.copyCommandResult}
            >
              <UiIcon name="copy" className="h-3.5 w-3.5" />
            </button>
          </div>
          <pre
            role="region"
            aria-label={p.commandResult}
            tabIndex={0}
            className="kn-scroll max-h-[300px] overflow-y-auto overscroll-contain whitespace-pre-wrap pr-2 text-sm leading-7 text-[color:var(--kn-text-soft)] focus:outline-none focus:ring-1 focus:ring-[color:var(--kn-accent)]"
          >
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
