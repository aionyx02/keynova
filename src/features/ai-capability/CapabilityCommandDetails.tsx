import { useState, type ReactElement } from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";
import type { CommandSuggestion, GenCommandCtx } from "./types";

interface Props {
  data: CommandSuggestion;
  riskRequiresConfirmation: boolean;
  assumptions?: GenCommandCtx;
}

export function CapabilityCommandDetails({
  data,
  riskRequiresConfirmation,
  assumptions,
}: Props): ReactElement {
  const c = useI18n().capability;
  const [copyState, setCopyState] = useState<"idle" | "copied">("idle");
  const assumptionParts = assumptions
    ? [
        assumptions.cwd ? `cwd=${assumptions.cwd}` : null,
        assumptions.shell ? `shell=${assumptions.shell}` : null,
        assumptions.os ? `os=${assumptions.os}` : null,
      ].filter((item): item is string => item !== null)
    : [];

  async function handleCopy() {
    if (!data.command) return;
    try {
      await navigator.clipboard.writeText(data.command);
      setCopyState("copied");
      window.setTimeout(() => setCopyState("idle"), 1200);
    } catch {
      // Clipboard failure on insecure context: no-op.
    }
  }

  return (
    <div className="space-y-3">
      <div className="rounded-[16px] border border-[color:var(--kn-border)] bg-black/20 px-3 py-3 font-mono text-[13px] leading-6 text-[color:var(--kn-text-soft)]">
        {data.command}
      </div>
      {assumptions && (
        <div className="text-[11px] leading-5 text-[color:var(--kn-text-faint)]">
          <span className="font-semibold uppercase tracking-[0.12em]">{c.assumes}: </span>
          {assumptionParts.length > 0 ? assumptionParts.join(" · ") : c.assumptionsNone}
        </div>
      )}
      <div className="space-y-1">
        <div className="text-[11px] font-semibold uppercase tracking-[0.16em] text-[color:var(--kn-text-faint)]">
          {c.rationale}
        </div>
        <div className="text-[13px] leading-6 text-[color:var(--kn-text)]">
          {data.rationale || c.noRationale}
        </div>
      </div>
      <div
        className={[
          "rounded-[14px] border px-3 py-2 text-[12px] leading-5",
          riskRequiresConfirmation
            ? "border-amber-300/25 bg-amber-300/8 text-amber-100"
            : "border-cyan-300/20 bg-cyan-300/5 text-cyan-100",
        ].join(" ")}
      >
        {riskRequiresConfirmation ? c.riskConfirm : c.riskReadOnly}
        <div className="mt-1 text-[11px] opacity-75">{c.copyOnly}</div>
      </div>
      <button
        type="button"
        onMouseDown={(event) => {
          event.preventDefault();
          void handleCopy();
        }}
        className="kn-button"
      >
        <UiIcon name="copy" className="h-3.5 w-3.5" />
        {copyState === "copied" ? c.copied : c.copy}
      </button>
    </div>
  );
}
