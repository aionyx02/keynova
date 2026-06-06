// Streaming answer card for prefix capability mode.
//
// Pure presentational: receives projected stream state from
// `useCapabilityStream` (lifted into `CapabilityResultArea`) and renders the
// header + Markdown body + footer chips. Wires [Copy md] and [Save to note] to
// existing utilities (navigator.clipboard, dispatch("note.save")).

import { useEffect, useState, type ReactElement } from "react";

import { UiIcon } from "../../components/icons/UiIcon";
import { Markdown } from "../../shared/components/Markdown";
import { useI18n } from "../../i18n/useI18n";
import type { DispatchFn } from "../../context/IPCContext";
import type { CapabilityStreamStatus } from "./hooks/useCapabilityStream";
import { CapabilityCommandDetails } from "./CapabilityCommandDetails";
import { CapabilitySources } from "./CapabilitySources";
import type { CapabilitySource, CommandSuggestion } from "./types";

export type AnswerCardCapability = "explain" | "summarize" | "fix";

interface Props {
  capabilityLabel: AnswerCardCapability;
  status: CapabilityStreamStatus;
  text: string;
  sources: CapabilitySource[];
  suggestedCommand: CommandSuggestion | null;
  riskRequiresConfirmation: boolean;
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
  sources,
  suggestedCommand,
  riskRequiresConfirmation,
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

  const c = useI18n().capability;
  const labelTitle: Record<AnswerCardCapability, string> = {
    explain: c.explain,
    summarize: c.summarize,
    fix: c.fix,
  };
  const headerLabel = `${labelTitle[capabilityLabel]} - ${formatLatencyMs(latencyMs)}`;
  const statusSuffix =
    status === "error" ? c.suffixError : status === "cancelled" ? c.suffixCancelled : "";

  const isBodyError = status === "error" && error !== null;
  const isBodyCancelled = status === "cancelled";
  const showFooterChips = status === "complete";
  const footerLabel =
    status === "pending" || status === "streaming"
      ? c.ansFooterStreaming
      : status === "idle"
        ? c.ansFooterIdle
        : status === "complete"
          ? c.ansFooterComplete
          : status === "cancelled"
            ? c.ansFooterCancelled
            : c.ansFooterError;

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
    const noteName = `${labelTitle[capabilityLabel]}: ${args.text.slice(0, 40).trim()}`;
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
      <div className="kn-panel-header">
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
          className="kn-button h-7 w-7 px-0 py-0"
          aria-label={c.close}
        >
          <UiIcon name="x" className="h-3.5 w-3.5" />
        </button>
      </div>

      <div className="kn-scroll max-h-[300px] overflow-y-auto px-4 py-3 text-sm text-[color:var(--kn-text)]">
        {isBodyError ? (
          <div className="whitespace-pre-wrap break-words text-rose-200">{error}</div>
        ) : isBodyCancelled ? (
          <div className="text-[color:var(--kn-text-muted)]">{c.cancelledBody}</div>
        ) : text ? (
          <div className="space-y-4">
            <Markdown content={text} />
            {status === "complete" && suggestedCommand && (
              <CapabilityCommandDetails
                data={suggestedCommand}
                riskRequiresConfirmation={riskRequiresConfirmation}
              />
            )}
            {status === "complete" && <CapabilitySources sources={sources} />}
          </div>
        ) : status === "pending" ? (
          <div className="text-[color:var(--kn-text-muted)]">
            {c.ansPendingBody}{" "}
            <span className="text-[color:var(--kn-text-faint)]">{c.ansPendingHint}</span>
          </div>
        ) : status === "idle" && args.text.trim() ? (
          <div className="text-[color:var(--kn-text-muted)]">{c.ansIdleBody}</div>
        ) : null}
      </div>

      {showFooterChips && text && (
        <div className="flex flex-wrap gap-2 border-t border-[color:var(--kn-border)] bg-white/[0.015] px-4 py-3 text-[11px]">
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              void handleCopyMd();
            }}
            className="kn-button"
          >
            <UiIcon name="file" className="h-3.5 w-3.5" />
            {copyState === "copied" ? c.copied : c.copyMd}
          </button>
          <button
            type="button"
            onMouseDown={(e) => {
              e.preventDefault();
              void handleSaveToNote();
            }}
            disabled={saveState === "saving"}
            className="kn-button disabled:opacity-60"
          >
            <UiIcon name="note" className="h-3.5 w-3.5" />
            {saveState === "saving"
              ? c.saving
              : saveState === "saved"
                ? c.saved
                : saveState === "error"
                  ? c.saveFailed
                  : c.saveToNote}
          </button>
        </div>
      )}

      <div className="kn-panel-footer text-[10px]">{footerLabel}</div>
    </div>
  );
}
