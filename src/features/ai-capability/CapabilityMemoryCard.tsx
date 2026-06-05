import { useEffect, useState, type ReactElement } from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";
import type { RememberOutput } from "./types";
import type { CapabilityRunStatus } from "./hooks/useCapabilityRunState";

interface Props {
  status: CapabilityRunStatus;
  data: RememberOutput | null;
  error: string | null;
  startedAtMs: number | null;
  completedAtMs: number | null;
  /** Raw note the user is asking to remember (shown in the idle/preview state). */
  intent: string;
  onCancel: () => void;
  onClose: () => void;
  onSubmit: () => void;
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

export function CapabilityMemoryCard({
  status,
  data,
  error,
  startedAtMs,
  completedAtMs,
  intent,
  onCancel,
  onClose,
  onSubmit,
}: Props): ReactElement {
  const now = useTickingClock(status === "pending" && completedAtMs === null);
  const latencyMs = (() => {
    if (startedAtMs === null) return 0;
    const stop = completedAtMs ?? now;
    return stop - startedAtMs;
  })();
  const c = useI18n().capability;
  const statusSuffix =
    status === "error" ? c.suffixError : status === "cancelled" ? c.suffixCancelled : "";

  const footerLabel =
    status === "pending"
      ? c.memFooterPending
      : status === "idle"
        ? c.memFooterIdle
        : status === "cancelled"
          ? c.memFooterCancelled
          : status === "error"
            ? c.memFooterError
            : data?.saved
              ? c.memFooterSaved
              : c.memFooterNotSaved;

  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0">
      <div className="flex items-center justify-between border-b border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-3">
        <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--kn-text-soft)]">
          <span aria-hidden>AI </span>
          {`remember - ${formatLatencyMs(latencyMs)}`}
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
        ) : data ? (
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              {data.saved && (
                <span className="inline-flex items-center gap-1 rounded-[10px] border border-emerald-300/25 bg-emerald-300/10 px-2 py-0.5 text-[11px] font-medium text-emerald-100">
                  <span aria-hidden>✓</span>
                  {c.savedBadge}
                </span>
              )}
              <span className="font-semibold text-[color:var(--kn-text)]">{data.title}</span>
            </div>
            <div className="whitespace-pre-wrap break-words text-[13px] leading-6 text-[color:var(--kn-text-soft)]">
              {data.content}
            </div>
          </div>
        ) : status === "pending" ? (
          <div className="text-[color:var(--kn-text-muted)]">{c.memPendingBody}</div>
        ) : (
          <div className="space-y-2 text-[color:var(--kn-text-muted)]">
            <div>{c.memIdleBody}</div>
            <div className="text-[12px] text-[color:var(--kn-text-faint)]">
              {c.noteLabel}{" "}
              <span className="font-medium text-[color:var(--kn-text-soft)]">{intent}</span>
            </div>
          </div>
        )}
      </div>

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
            {status === "idle" ? c.remember : c.retry}
          </button>
        </div>
      )}

      <div className="border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-2 text-[10px] text-[color:var(--kn-text-muted)]">
        {footerLabel}
      </div>
    </div>
  );
}
