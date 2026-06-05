import { useEffect, useState, type ReactElement } from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";
import type { GenCommandOutput } from "./types";
import type { CapabilityRunStatus } from "./hooks/useCapabilityRunState";

interface Props {
  label?: string;
  status: CapabilityRunStatus;
  data: GenCommandOutput | null;
  error: string | null;
  startedAtMs: number | null;
  completedAtMs: number | null;
  intent: string;
  riskRequiresConfirmation: boolean;
  onCancel: () => void;
  onClose: () => void;
  onSubmit: () => void;
  onRun: (command: string) => void;
  onEditBefore: (command: string) => void;
}

function formatLatencyMs(ms: number): string {
  if (ms < 0) return "0.0s";
  return `${(ms / 1000).toFixed(1)}s`;
}

function useTickingClock(active: boolean, intervalMs = 100): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return;
    const handle = window.setInterval(() => setNow(Date.now()), intervalMs);
    return () => window.clearInterval(handle);
  }, [active, intervalMs]);
  return now;
}

function confidenceLabel(confidence: number): "low" | "medium" | "high" {
  if (confidence < 0.4) return "low";
  if (confidence < 0.75) return "medium";
  return "high";
}

export function CapabilityCommandCard({
  label = "cmd",
  status,
  data,
  error,
  startedAtMs,
  completedAtMs,
  intent,
  riskRequiresConfirmation,
  onCancel,
  onClose,
  onSubmit,
  onRun,
  onEditBefore,
}: Props): ReactElement {
  const now = useTickingClock(status === "pending" && completedAtMs === null);
  const latencyMs = (() => {
    if (startedAtMs === null) return 0;
    const stop = completedAtMs ?? now;
    return stop - startedAtMs;
  })();
  const c = useI18n().capability;
  const confidence = data ? confidenceLabel(data.confidence) : null;
  const headerSuffix = confidence ? ` - confidence: ${confidence}` : "";
  const statusSuffix =
    status === "error" ? c.suffixError : status === "cancelled" ? c.suffixCancelled : "";

  const [copyState, setCopyState] = useState<"idle" | "copied">("idle");

  async function handleCopy() {
    if (!data?.command) return;
    try {
      await navigator.clipboard.writeText(data.command);
      setCopyState("copied");
      window.setTimeout(() => setCopyState("idle"), 1200);
    } catch {
      // Clipboard failure on insecure context: no-op for now.
    }
  }

  const footerLabel =
    status === "pending"
      ? c.cmdFooterPending
      : status === "idle"
        ? c.cmdFooterIdle
        : status === "cancelled"
          ? c.cmdFooterCancelled
          : status === "error"
            ? c.cmdFooterError
            : data?.command
              ? c.cmdFooterReady
              : c.cmdFooterUnavailable;

  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0">
      <div className="flex items-center justify-between border-b border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-3">
        <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--kn-text-soft)]">
          <span aria-hidden>AI </span>
          {`${label} - ${formatLatencyMs(latencyMs)}`}
          {headerSuffix}
          {statusSuffix}
        </span>
        <button
          type="button"
          onMouseDown={(e) => {
            e.preventDefault();
            if (status === "pending") onCancel();
            else onClose();
          }}
          className="flex h-7 w-7 items-center justify-center rounded-[10px] border border-[color:var(--kn-border)] bg-white/[0.035] text-[color:var(--kn-text-muted)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
          aria-label={c.close}
        >
          <UiIcon name="x" className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="kn-scroll max-h-[300px] overflow-y-auto px-4 py-3 text-sm text-[color:var(--kn-text)]">
        {status === "error" && error ? (
          <div className="whitespace-pre-wrap break-words text-rose-200">{error}</div>
        ) : status === "cancelled" ? (
          <div className="text-[color:var(--kn-text-muted)]">{c.cancelledBody}</div>
        ) : data?.command ? (
          <div className="space-y-3">
            <div
              className={[
                "rounded-[16px] border border-[color:var(--kn-border)] bg-black/20 px-3 py-3 font-mono text-[13px] leading-6 text-[color:var(--kn-text-soft)]",
                confidence === "low" ? "opacity-70" : "",
              ].join(" ")}
            >
              {data.command}
            </div>
            <div className="space-y-1">
              <div className="text-[11px] font-semibold uppercase tracking-[0.16em] text-[color:var(--kn-text-faint)]">
                {c.rationale}
              </div>
              <div className="text-[13px] leading-6 text-[color:var(--kn-text)]">
                {data.rationale || c.noRationale}
              </div>
            </div>
            {riskRequiresConfirmation && (
              <div className="rounded-[14px] border border-amber-300/25 bg-amber-300/8 px-3 py-2 text-[12px] leading-5 text-amber-100">
                {c.riskWarning}
              </div>
            )}
          </div>
        ) : status === "pending" ? (
          <div className="text-[color:var(--kn-text-muted)]">{c.cmdPendingBody}</div>
        ) : status === "complete" ? (
          <div className="text-[color:var(--kn-text-muted)]">{c.cmdUnreadable}</div>
        ) : (
          <div className="space-y-2 text-[color:var(--kn-text-muted)]">
            <div>{c.cmdIdleBody}</div>
            <div className="text-[12px] text-[color:var(--kn-text-faint)]">
              {c.intentLabel}{" "}
              <span className="font-medium text-[color:var(--kn-text-soft)]">{intent}</span>
            </div>
          </div>
        )}
      </div>

      {status === "complete" && data?.command && (
        <div className="flex flex-wrap gap-2 border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.42)] px-4 py-3 text-[11px]">
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              onRun(data.command);
            }}
            className="inline-flex items-center gap-1.5 rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.035] px-2.5 py-1.5 font-medium text-[color:var(--kn-text-soft)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
          >
            <UiIcon name="command" className="h-3.5 w-3.5" />
            {c.run}
          </button>
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              onEditBefore(data.command);
            }}
            className="inline-flex items-center gap-1.5 rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.035] px-2.5 py-1.5 font-medium text-[color:var(--kn-text-soft)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
          >
            <UiIcon name="command" className="h-3.5 w-3.5" />
            {c.editBefore}
          </button>
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              void handleCopy();
            }}
            className="inline-flex items-center gap-1.5 rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.035] px-2.5 py-1.5 font-medium text-[color:var(--kn-text-soft)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
          >
            <UiIcon name="file" className="h-3.5 w-3.5" />
            {copyState === "copied" ? c.copied : c.copy}
          </button>
        </div>
      )}

      {status !== "pending" && status !== "complete" && (
        <div className="border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.42)] px-4 py-3 text-[11px]">
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              onSubmit();
            }}
            className="inline-flex items-center gap-1.5 rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.035] px-2.5 py-1.5 font-medium text-[color:var(--kn-text-soft)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
          >
            <UiIcon name="command" className="h-3.5 w-3.5" />
            {status === "idle" ? c.generate : c.retry}
          </button>
        </div>
      )}

      <div className="border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-2 text-[10px] text-[color:var(--kn-text-muted)]">
        {footerLabel}
      </div>
    </div>
  );
}
