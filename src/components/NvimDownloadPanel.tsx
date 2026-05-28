import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PanelProps } from "../types/panel";

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

  const stageLabel: Record<Stage, string> = {
    idle: "Ready",
    downloading: "Downloading",
    extracting: "Extracting",
    done: "Installed",
    error: "Failed",
  };

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
          <div className="kn-panel-title">Install Neovim</div>
          <div className="kn-panel-subtitle">Fetch a portable Neovim copy for the LazyVim workflow</div>
        </div>
        <span className={`kn-chip ${isActive || stage === "done" ? "kn-chip-active" : ""}`}>
          {stageLabel[stage]}
        </span>
      </div>

      <div className="flex flex-1 flex-col gap-4 px-4 py-4">
        <div className="kn-muted-surface px-4 py-3 text-sm leading-6 text-[color:var(--kn-text-soft)]">
          Neovim was not found on this machine. Keynova can download a portable
          copy of Neovim v0.10.4 and wire it into the note workflow for you.
        </div>

        {stage === "idle" && (
          <button type="button" onClick={startDownload} className="kn-button kn-button-primary self-start px-4 py-2">
            Download Neovim v0.10.4
          </button>
        )}

        {isActive && (
          <div className="kn-muted-surface space-y-3 px-4 py-3">
            <div className="flex items-center justify-between text-xs text-[color:var(--kn-text-muted)]">
              <span>{stageLabel[stage]}</span>
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
              Neovim installed successfully.
            </div>
            {nvimPath && (
              <div className="kn-muted-surface break-all px-4 py-3 font-mono text-xs text-[color:var(--kn-text-muted)]">
                {nvimPath}
              </div>
            )}
            <button type="button" onClick={onClose} className="kn-button kn-button-primary self-start px-4 py-2">
              Close and retry /lazyvim
            </button>
          </div>
        )}

        {stage === "error" && (
          <div className="space-y-3">
            <div className="kn-muted-surface border-red-400/20 bg-[color:var(--kn-danger-wash)] px-4 py-3 text-sm text-red-100">
              {error ?? "The download failed."}
            </div>
            <button type="button" onClick={startDownload} className="kn-button self-start px-4 py-2">
              Retry
            </button>
          </div>
        )}
      </div>

      <div className="kn-panel-footer">
        <span>Esc closes</span>
        <span>{stage === "done" ? "Neovim is ready to use" : "Portable install, no manual setup required"}</span>
      </div>
    </div>
  );
}
