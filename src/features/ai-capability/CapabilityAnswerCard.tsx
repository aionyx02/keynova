// REF.6.B - Streaming answer card for prefix capability mode.
//
// Pure presentational: receives projected stream state from
// `useCapabilityStream` (lifted into `CapabilityResultArea`) and renders the
// header + Markdown body + footer chips. Wires [Copy md] and [Save to note] to
// existing utilities (navigator.clipboard, dispatch("note.save")).

import { useEffect, useState, type ReactElement } from "react";

import { Markdown } from "../../components/Markdown";
import type { DispatchFn } from "../../context/IPCContext";
import type { CapabilityStreamStatus } from "./hooks/useCapabilityStream";

interface Props {
  capabilityLabel: "explain" | "summarize";
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

const LABEL_TITLE: Record<"explain" | "summarize", string> = {
  explain: "Explain",
  summarize: "Summarize",
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
  const now = useTickingClock(
    isTicking && firstChunkAtMs === null && completedAtMs === null,
  );

  const latencyMs = (() => {
    if (startedAtMs === null) return 0;
    const stop = completedAtMs ?? firstChunkAtMs ?? now;
    return stop - startedAtMs;
  })();

  const headerLabel = `${LABEL_TITLE[capabilityLabel]} - ${formatLatencyMs(latencyMs)}`;
  const statusSuffix =
    status === "error"
      ? " - error"
      : status === "cancelled"
        ? " - cancelled"
        : "";

  const isBodyError = status === "error" && error !== null;
  const isBodyCancelled = status === "cancelled";
  const showFooterChips = status === "complete";

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
    <div className="border-t border-gray-700/50 bg-gray-950/60 px-4 py-3">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-[11px] uppercase tracking-wider text-gray-400">
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
          className="text-[11px] text-gray-500 hover:text-gray-300"
          aria-label="Close"
        >
          [x]
        </button>
      </div>

      <div className="max-h-[280px] overflow-y-auto text-sm text-gray-200">
        {isBodyError ? (
          <div className="whitespace-pre-wrap break-words text-red-400">{error}</div>
        ) : isBodyCancelled ? (
          <div className="text-gray-500">Cancelled.</div>
        ) : text ? (
          <Markdown content={text} />
        ) : status === "pending" ? (
          <div className="text-gray-500 italic">
            Asking model...{" "}
            <span className="text-gray-600">
              (first call after launch may take a few seconds while the model loads)
            </span>
          </div>
        ) : status === "idle" && args.text.trim() ? (
          <div className="text-gray-500">
            Press{" "}
            <kbd className="mx-0.5 rounded border border-gray-600 px-1.5 py-0.5 text-[10px]">
              Enter
            </kbd>{" "}
            to ask
          </div>
        ) : null}
      </div>

      {showFooterChips && text && (
        <div className="mt-3 flex flex-wrap gap-2 text-[11px]">
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              void handleCopyMd();
            }}
            className="rounded border border-gray-600 px-2 py-0.5 text-gray-300 hover:border-blue-400/60 hover:text-blue-300"
          >
            {copyState === "copied" ? "Copied" : "Copy md"}
          </button>
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              void handleSaveToNote();
            }}
            disabled={saveState === "saving"}
            className="rounded border border-gray-600 px-2 py-0.5 text-gray-300 hover:border-blue-400/60 hover:text-blue-300 disabled:opacity-60"
          >
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

      <div className="mt-2 text-[10px] text-gray-600">
        {status === "pending" || status === "streaming"
          ? "Esc cancels stream - Backspace past prefix returns to search"
          : status === "idle"
            ? "Enter to ask - Esc clears prefix"
            : "Esc clears prefix - Type more then Enter to re-ask"}
      </div>
    </div>
  );
}
