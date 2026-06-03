import { useEffect, useState, type ReactElement } from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import type { RecalledMemory } from "./types";
import type { CapabilityRunStatus } from "./hooks/useCapabilityRunState";

interface Props {
  status: CapabilityRunStatus;
  items: RecalledMemory[];
  error: string | null;
  startedAtMs: number | null;
  completedAtMs: number | null;
  query: string;
  onCancel: () => void;
  onClose: () => void;
  onSubmit: () => void;
  /** Paste a recalled memory's full content into the palette input. */
  onPaste: (content: string) => void;
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

export function CapabilityRecallCard({
  status,
  items,
  error,
  startedAtMs,
  completedAtMs,
  query,
  onCancel,
  onClose,
  onSubmit,
  onPaste,
}: Props): ReactElement {
  const now = useTickingClock(status === "pending" && completedAtMs === null);
  const latencyMs = (() => {
    if (startedAtMs === null) return 0;
    const stop = completedAtMs ?? now;
    return stop - startedAtMs;
  })();
  const statusSuffix =
    status === "error" ? " - error" : status === "cancelled" ? " - cancelled" : "";

  const footerLabel =
    status === "pending"
      ? "Searching local memories"
      : status === "idle"
        ? "Press Enter to recall"
        : status === "cancelled"
          ? "Cancelled"
          : status === "error"
            ? "Could not recall"
            : items.length > 0
              ? "Click a memory to paste it"
              : "No matching memories";

  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0">
      <div className="flex items-center justify-between border-b border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-3">
        <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--kn-text-soft)]">
          <span aria-hidden>AI </span>
          {`recall - ${formatLatencyMs(latencyMs)}`}
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
          aria-label="Close"
        >
          <UiIcon name="x" className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="kn-scroll max-h-[320px] overflow-y-auto px-2 py-2 text-sm text-[color:var(--kn-text)]">
        {status === "error" && error ? (
          <div className="whitespace-pre-wrap break-words px-2 py-1 text-rose-200">{error}</div>
        ) : status === "cancelled" ? (
          <div className="px-2 py-1 text-[color:var(--kn-text-muted)]">Cancelled.</div>
        ) : status === "pending" ? (
          <div className="px-2 py-1 text-[color:var(--kn-text-muted)]">
            Looking through your saved memories...
          </div>
        ) : status === "idle" ? (
          <div className="space-y-2 px-2 py-1 text-[color:var(--kn-text-muted)]">
            <div>Recall what you have saved.</div>
            <div className="text-[12px] text-[color:var(--kn-text-faint)]">
              Query: <span className="font-medium text-[color:var(--kn-text-soft)]">{query}</span>
            </div>
          </div>
        ) : items.length === 0 ? (
          <div className="px-2 py-1 text-[color:var(--kn-text-muted)]">
            No matching memories yet. Save one with{" "}
            <span className="font-medium text-[color:var(--kn-text-soft)]">remember</span>.
          </div>
        ) : (
          <div className="space-y-1">
            {items.map((item) => (
              <div
                key={item.id}
                role="button"
                tabIndex={0}
                onMouseDown={(e) => {
                  e.preventDefault();
                  onPaste(item.content);
                }}
                className="cursor-pointer rounded-[12px] border border-transparent px-3 py-2 transition-colors hover:border-[color:rgba(138,168,255,0.2)] hover:bg-[color:var(--kn-accent-wash)]"
              >
                <div className="truncate font-medium text-[color:var(--kn-text)]">{item.title}</div>
                {item.snippet && (
                  <div className="mt-0.5 truncate text-[12px] text-[color:var(--kn-text-muted)]">
                    {item.snippet}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      {status !== "pending" && (
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
            {status === "idle" ? "Recall" : "Refresh"}
          </button>
        </div>
      )}

      <div className="border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-2 text-[10px] text-[color:var(--kn-text-muted)]">
        {footerLabel}
      </div>
    </div>
  );
}
