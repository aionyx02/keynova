import { useEffect, useMemo, useState, type ReactElement } from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import { useI18n } from "../../i18n/useI18n";
import type { SuggestedNextAction } from "./types";
import type { CapabilityRunStatus } from "./hooks/useCapabilityRunState";

interface Props {
  label?: string;
  status: CapabilityRunStatus;
  items: SuggestedNextAction[];
  error: string | null;
  /** Optional discoverability hint appended to the footer (e.g. pin shortcut). */
  hint?: string;
  startedAtMs: number | null;
  completedAtMs: number | null;
  selectedIndex: number;
  onSelectIndex: (index: number) => void;
  onRunSelected: (index: number) => void;
  onCancel: () => void;
  onClose: () => void;
}

function formatLatencyMs(ms: number): string {
  if (ms < 0) return "0.0s";
  return `${(ms / 1000).toFixed(1)}s`;
}

function formatExecutedAt(epochSeconds: number): string {
  return new Date(epochSeconds * 1000).toLocaleString();
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

export function CapabilityListCard({
  label = "next",
  status,
  items,
  error,
  hint,
  startedAtMs,
  completedAtMs,
  selectedIndex,
  onSelectIndex,
  onRunSelected,
  onCancel,
  onClose,
}: Props): ReactElement {
  const now = useTickingClock(status === "pending" && completedAtMs === null);
  const latencyMs = (() => {
    if (startedAtMs === null) return 0;
    const stop = completedAtMs ?? now;
    return stop - startedAtMs;
  })();
  const c = useI18n().capability;
  const selected = items[selectedIndex] ?? null;
  const footerLabel = useMemo(() => {
    if (status === "pending") return c.listFooterPending;
    if (status === "error") return c.listFooterError;
    if (status === "cancelled") return c.listFooterCancelled;
    if (status === "idle") return c.listFooterIdle;
    if (!selected) return c.listFooterNoRecent;
    if (selected.replay) return c.listFooterReplay;
    return c.listFooterHistoryOnly;
  }, [selected, status, c]);

  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0">
      <div className="flex items-center justify-between border-b border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-3">
        <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--kn-text-soft)]">
          <span aria-hidden>AI </span>
          {`${label} - ${formatLatencyMs(latencyMs)}`}
          {status === "error" ? c.suffixError : status === "cancelled" ? c.suffixCancelled : ""}
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

      <div className="kn-scroll max-h-[320px] overflow-y-auto px-2 py-2">
        {status === "pending" ? (
          <div className="px-2 py-3 text-sm text-[color:var(--kn-text-muted)]">
            {c.listPendingBody}
          </div>
        ) : status === "error" && error ? (
          <div className="px-2 py-3 text-sm text-rose-200">{error}</div>
        ) : status === "cancelled" ? (
          <div className="px-2 py-3 text-sm text-[color:var(--kn-text-muted)]">{c.cancelledBody}</div>
        ) : items.length === 0 ? (
          <div className="px-2 py-3 text-sm text-[color:var(--kn-text-muted)]">
            {c.listNoRecentBody}
          </div>
        ) : (
          <div className="space-y-1">
            {items.map((item, index) => {
              const active = index === selectedIndex;
              return (
                <button
                  key={`${item.route}-${item.title}-${item.last_executed_at}`}
                  type="button"
                  onMouseEnter={() => onSelectIndex(index)}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    onSelectIndex(index);
                    if (item.replay) onRunSelected(index);
                  }}
                  className={[
                    "w-full rounded-[16px] border px-3 py-3 text-left transition-colors",
                    active
                      ? "border-[color:var(--kn-accent)] bg-[rgba(110,231,255,0.09)]"
                      : "border-[color:var(--kn-border)] bg-white/[0.02] hover:bg-white/[0.04]",
                  ].join(" ")}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <span
                          className={[
                            "truncate text-sm font-medium",
                            item.replay
                              ? "text-[color:var(--kn-text)]"
                              : "text-[color:var(--kn-text-soft)]",
                          ].join(" ")}
                        >
                          {item.confidence >= 0.75 ? "✨ " : ""}
                          {item.title}
                        </span>
                      </div>
                      <div className="mt-1 text-[11px] text-[color:var(--kn-text-faint)]">
                        {item.subtitle}
                      </div>
                      <div className="mt-2 text-[12px] leading-5 text-[color:var(--kn-text-muted)]">
                        {item.rationale}
                      </div>
                    </div>
                    <div className="shrink-0 text-right">
                      <div className="text-[11px] font-medium text-[color:var(--kn-text-soft)]">
                        {Math.round(item.confidence * 100)}%
                      </div>
                      <div className="mt-1 text-[10px] text-[color:var(--kn-text-faint)]">
                        {item.replay ? c.replay : c.historyOnly}
                      </div>
                    </div>
                  </div>
                  <div className="mt-2 flex items-center gap-2 text-[10px] text-[color:var(--kn-text-faint)]">
                    <UiIcon name={item.replay ? "command" : "history"} className="h-3.5 w-3.5" />
                    {formatExecutedAt(item.last_executed_at)}
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>

      <div className="border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-2 text-[10px] text-[color:var(--kn-text-muted)]">
        {footerLabel}
        {hint ? ` · ${hint}` : ""}
      </div>
    </div>
  );
}
