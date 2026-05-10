import { useEffect, useState } from "react";
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

async function ipcDispatch<T>(
  route: string,
  payload?: Record<string, unknown>,
): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function NvimDownloadPanel({ onClose }: PanelProps) {
  const [stage, setStage] = useState<Stage>("idle");
  const [pct, setPct] = useState(0);
  const [nvimPath, setNvimPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    let unlisten: (() => void) | undefined;
    listen<ProgressEvent>("nvim-download-progress", (e) => {
      const { stage: s, pct: p, path, message } = e.payload;
      setStage(s);
      if (p !== undefined) setPct(p);
      if (path) setNvimPath(path);
      if (message) setError(message);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  const startDownload = () => {
    if (!window.__TAURI_INTERNALS__) return;
    setStage("downloading");
    setPct(0);
    setError(null);
    ipcDispatch("nvim.download").catch((e: unknown) => {
      setStage("error");
      setError(String(e));
    });
  };

  const stageLabel: Record<Stage, string> = {
    idle: "",
    downloading: "Downloading…",
    extracting: "Extracting…",
    done: "Done",
    error: "Failed",
  };

  const isActive = stage === "downloading" || stage === "extracting";

  return (
    <div className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col p-5 gap-4 min-w-[340px]">
      <div className="flex items-center gap-2">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">
          Install Neovim
        </span>
        <button
          onClick={onClose}
          className="ml-auto text-gray-600 hover:text-gray-400 text-xs leading-none"
          aria-label="Close"
        >
          ✕
        </button>
      </div>

      <p className="text-xs text-gray-400 leading-relaxed">
        Neovim was not found on your system. Keynova can download a portable
        copy of Neovim {/* version */} v0.10.4 for you.
      </p>

      {stage === "idle" && (
        <button
          onClick={startDownload}
          className="bg-blue-600 hover:bg-blue-500 text-white text-xs font-medium py-2 px-4 rounded-lg transition-colors"
        >
          Download Neovim v0.10.4
        </button>
      )}

      {isActive && (
        <div className="space-y-2">
          <div className="flex justify-between text-[10px] text-gray-500">
            <span>{stageLabel[stage]}</span>
            <span className="font-mono text-gray-300">{pct}%</span>
          </div>
          <div className="h-1.5 w-full bg-gray-700 rounded-full overflow-hidden">
            <div
              className="h-full bg-blue-500 rounded-full transition-all duration-300"
              style={{ width: `${pct}%` }}
            />
          </div>
        </div>
      )}

      {stage === "done" && (
        <div className="space-y-3">
          <p className="text-xs text-green-400">
            Neovim installed successfully.
          </p>
          {nvimPath && (
            <p className="text-[10px] text-gray-600 font-mono break-all">
              {nvimPath}
            </p>
          )}
          <button
            onClick={onClose}
            className="w-full bg-green-700 hover:bg-green-600 text-white text-xs font-medium py-2 px-4 rounded-lg transition-colors"
          >
            Close — retry your /lazyvim command
          </button>
        </div>
      )}

      {stage === "error" && (
        <div className="space-y-3">
          <p className="text-xs text-red-400">{error ?? "Download failed."}</p>
          <button
            onClick={startDownload}
            className="bg-gray-700 hover:bg-gray-600 text-gray-200 text-xs font-medium py-2 px-4 rounded-lg transition-colors"
          >
            Retry
          </button>
        </div>
      )}
    </div>
  );
}