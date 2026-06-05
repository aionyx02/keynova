import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "../../i18n/useI18n";
import type { PanelProps } from "../../types/panel";

type Stage = "idle" | "downloading" | "extracting" | "done" | "error";

interface ProgressEvent {
  stage: Stage;
  pct?: number;
  path?: string;
  message?: string;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function NvimDownloadPanel({ onClose }: PanelProps) {
  const t = useI18n().nvim;
  const [stage, setStage] = useState<Stage>("idle");
  const [pct, setPct] = useState(0);
  const [nvimPath, setNvimPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    rootRef.current?.focus();
    if (!window.__TAURI_INTERNALS__) return;
    let unlisten: (() => void) | undefined;
    void listen<ProgressEvent>("nvim-download-progress", (event) => {
      const { stage: nextStage, pct: nextPct, path, message } = event.payload;
      setStage(nextStage);
      if (nextPct !== undefined) setPct(nextPct);
      if (path) setNvimPath(path);
      if (message) setError(message);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  function startDownload() {
    if (!window.__TAURI_INTERNALS__) return;
    setStage("downloading");
    setPct(0);
    setError(null);
    void ipcDispatch("nvim.download").catch((err: unknown) => {
      setStage("error");
      setError(String(err));
    });
  }

  const isActive = stage === "downloading" || stage === "extracting";

  return (
    <div
      ref={rootRef}
      tabIndex={-1}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          onClose();
        }
      }}
      className="kn-panel-shell flex min-h-[320px] flex-col rounded-t-none border-t-0 outline-none"
    >
      <div className="kn-panel-header">
        <div>
          <div className="kn-panel-title">{t.title}</div>
          <div className="kn-panel-subtitle">{t.subtitle}</div>
        </div>
        <span className={`kn-chip ${isActive || stage === "done" ? "kn-chip-active" : ""}`}>
          {t.stages[stage]}
        </span>
      </div>

      <div className="flex flex-1 flex-col gap-4 px-4 py-4">
        <div className="kn-muted-surface px-4 py-3 text-sm leading-6 text-[color:var(--kn-text-soft)]">
          {t.body}
        </div>

        {stage === "idle" && (
          <button
            type="button"
            onClick={startDownload}
            className="kn-button kn-button-primary self-start px-4 py-2"
          >
            {t.download}
          </button>
        )}

        {isActive && (
          <div className="kn-muted-surface space-y-3 px-4 py-3">
            <div className="flex items-center justify-between text-xs text-[color:var(--kn-text-muted)]">
              <span>{t.stages[stage]}</span>
              <span className="font-mono text-[color:var(--kn-text-soft)]">{pct}%</span>
            </div>
            <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/[0.05]">
              <div
                className="h-full rounded-full bg-[color:var(--kn-accent)] transition-all duration-300"
                style={{ width: `${pct}%` }}
              />
            </div>
          </div>
        )}

        {stage === "done" && (
          <div className="space-y-3">
            <div className="kn-muted-surface border-emerald-400/20 bg-[color:var(--kn-success-wash)] px-4 py-3 text-sm text-emerald-100">
              {t.installed}
            </div>
            {nvimPath && (
              <div className="kn-muted-surface break-all px-4 py-3 font-mono text-xs text-[color:var(--kn-text-muted)]">
                {nvimPath}
              </div>
            )}
            <button
              type="button"
              onClick={onClose}
              className="kn-button kn-button-primary self-start px-4 py-2"
            >
              {t.closeRetry}
            </button>
          </div>
        )}

        {stage === "error" && (
          <div className="space-y-3">
            <div className="kn-muted-surface border-red-400/20 bg-[color:var(--kn-danger-wash)] px-4 py-3 text-sm text-red-100">
              {error ?? t.downloadFailed}
            </div>
            <button
              type="button"
              onClick={startDownload}
              className="kn-button self-start px-4 py-2"
            >
              {t.retry}
            </button>
          </div>
        )}
      </div>

      <div className="kn-panel-footer">
        <span>{t.escCloses}</span>
        <span>{stage === "done" ? t.readyFooter : t.portableFooter}</span>
      </div>
    </div>
  );
}
