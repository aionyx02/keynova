// REF.6.B - Streaming answer card for prefix capability mode.
//
// Pure presentational: receives projected stream state from
// `useCapabilityStream` (lifted into `CapabilityResultArea`) and renders the
// header + Markdown body + footer chips. Wires [Copy md] and [Save to note] to
// existing utilities (navigator.clipboard, dispatch("note.save")).

import { useEffect, useState, type ReactElement } from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import { Markdown } from "../../components/Markdown";
import type { DispatchFn } from "../../context/IPCContext";
import type { CapabilityStreamStatus } from "./hooks/useCapabilityStream";

export type AnswerCardCapability = "explain" | "summarize" | "fix";

interface Props {
  capabilityLabel: AnswerCardCapability;
  status: CapabilityStreamStatus;
  text: string;
  error: string | null;
  startedAtMs: number | null;
  firstChunkAtMs: number | null;
  completedAtMs: number | null;
  /** The body the user gave after the prefix; used to auto-name saved notes. */
  args: { text: string };
  dispatch: DispatchFn;
  onCancel: () => void;
  onClose: () => void;
}

const LABEL_TITLE: Record<AnswerCardCapability, string> = {
  explain: "Explain",
  summarize: "Summarize",
  fix: "Fix",
};

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

export function CapabilityAnswerCard({
  capabilityLabel,
  status,
  text,
  error,
  startedAtMs,
  firstChunkAtMs,
  completedAtMs,
  args,
  dispatch,
  onCancel,
  onClose,
}: Props): ReactElement {
  const isTicking = status === "pending" || status === "streaming";
  const now = useTickingClock(isTicking && firstChunkAtMs === null && completedAtMs === null);

  const latencyMs = (() => {
    if (startedAtMs === null) return 0;
    const stop = completedAtMs ?? firstChunkAtMs ?? now;
    return stop - startedAtMs;
  })();

  const headerLabel = `${LABEL_TITLE[capabilityLabel]} - ${formatLatencyMs(latencyMs)}`;
  const statusSuffix =
    status === "error" ? " - error" : status === "cancelled" ? " - cancelled" : "";

  const isBodyError = status === "error" && error !== null;
  const isBodyCancelled = status === "cancelled";
  const showFooterChips = status === "complete";
  const footerLabel =
    status === "pending" || status === "streaming"
      ? "Streaming response"
      : status === "idle"
        ? "Ready"
        : status === "complete"
          ? "Response complete"
          : status === "cancelled"
            ? "Cancelled"
            : "Error";

  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [copyState, setCopyState] = useState<"idle" | "copied">("idle");

  async function handleCopyMd() {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopyState("copied");
      window.setTimeout(() => setCopyState("idle"), 1200);
    } catch {
      // Clipboard failure on insecure context: no-op for now.
    }
  }

  async function handleSaveToNote() {
    if (!text) return;
    setSaveState("saving");
    const noteName = `${LABEL_TITLE[capabilityLabel]}: ${args.text.slice(0, 40).trim()}`;
    try {
      await dispatch("note.save", { name: noteName, content: text });
      setSaveState("saved");
      window.setTimeout(() => setSaveState("idle"), 1500);
    } catch {
      setSaveState("error");
      window.setTimeout(() => setSaveState("idle"), 2000);
    }
  }

  return (
    <div className="kn-panel-shell overflow-hidden rounded-t-none border-t-0">
      <div className="flex items-center justify-between border-b border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-3">
        <span className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[color:var(--kn-text-soft)]">
          <span aria-hidden>AI </span>
          {headerLabel}
          {statusSuffix}
        </span>
        <button
          type="button"
          onMouseDown={(e) => {
            e.preventDefault();
            if (status === "pending" || status === "streaming") {
              onCancel();
            } else {
              onClose();
            }
          }}
          className="flex h-7 w-7 items-center justify-center rounded-[10px] border border-[color:var(--kn-border)] bg-white/[0.035] text-[color:var(--kn-text-muted)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
          aria-label="Close"
        >
          <UiIcon name="x" className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="kn-scroll max-h-[300px] overflow-y-auto px-4 py-3 text-sm text-[color:var(--kn-text)]">
        {isBodyError ? (
          <div className="whitespace-pre-wrap break-words text-rose-200">{error}</div>
        ) : isBodyCancelled ? (
          <div className="text-[color:var(--kn-text-muted)]">Cancelled.</div>
        ) : text ? (
          <Markdown content={text} />
        ) : status === "pending" ? (
          <div className="text-[color:var(--kn-text-muted)]">
            Asking model...{" "}
            <span className="text-[color:var(--kn-text-faint)]">
              (first call after launch may take a few seconds while the model loads)
            </span>
          </div>
        ) : status === "idle" && args.text.trim() ? (
          <div className="text-[color:var(--kn-text-muted)]">Ready to ask.</div>
        ) : null}
      </div>

      {showFooterChips && text && (
        <div className="flex flex-wrap gap-2 border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.42)] px-4 py-3 text-[11px]">
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              void handleCopyMd();
            }}
            className="inline-flex items-center gap-1.5 rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.035] px-2.5 py-1.5 font-medium text-[color:var(--kn-text-soft)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)]"
          >
            <UiIcon name="file" className="h-3.5 w-3.5" />
            {copyState === "copied" ? "Copied" : "Copy md"}
          </button>
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              void handleSaveToNote();
            }}
            disabled={saveState === "saving"}
            className="inline-flex items-center gap-1.5 rounded-[12px] border border-[color:var(--kn-border)] bg-white/[0.035] px-2.5 py-1.5 font-medium text-[color:var(--kn-text-soft)] transition-colors hover:bg-white/[0.06] hover:text-[color:var(--kn-text)] disabled:opacity-60"
          >
            <UiIcon name="note" className="h-3.5 w-3.5" />
            {saveState === "saving"
              ? "Saving..."
              : saveState === "saved"
                ? "Saved"
                : saveState === "error"
                  ? "Save failed"
                  : "Save to note"}
          </button>
        </div>
      )}

      <div className="border-t border-[color:var(--kn-border)] bg-[rgba(7,11,17,0.48)] px-4 py-2 text-[10px] text-[color:var(--kn-text-muted)]">
        {footerLabel}
      </div>
    </div>
  );
}
