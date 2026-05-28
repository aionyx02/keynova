import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PanelProps } from "../types/panel";

interface LocalModel {
  name: string;
  provider: string;
  size_gb?: number | null;
  active: boolean;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function ModelRemovePanel({ onClose }: PanelProps) {
  const [models, setModels] = useState<LocalModel[]>([]);
  const [selected, setSelected] = useState(0);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const local = await ipcDispatch<LocalModel[]>("model.list_local", { tool: "ai" });
      setModels(local);
      setSelected((i) => Math.min(i, Math.max(local.length - 1, 0)));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    rootRef.current?.focus();
    const timer = window.setTimeout(() => {
      void load();
    }, 0);
    return () => window.clearTimeout(timer);
  }, [load]);

  async function deleteModel(name: string) {
    setDeleting(name);
    setError("");
    try {
      await ipcDispatch("model.delete", { name });
      setNotice(`Removed ${name}.`);
      setConfirming(null);
      await load();
    } catch (err) {
      setError(String(err));
    } finally {
      setDeleting(null);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (confirming) setConfirming(null);
      else onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, models.length - 1));
      setConfirming(null);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
      setConfirming(null);
    } else if (e.key === "Enter" || e.key === "Delete") {
      e.preventDefault();
      const model = models[selected];
      if (!model) return;
      if (confirming === model.name) {
        void deleteModel(model.name);
      } else {
        setConfirming(model.name);
      }
    }
  }

  return (
    <div
      ref={rootRef}
      tabIndex={-1}
      onKeyDown={handleKeyDown}
      className="kn-panel-shell flex min-h-[320px] flex-col rounded-t-none border-t-0 outline-none"
    >
      <div className="kn-panel-header">
        <div>
          <div className="kn-panel-title">Remove Model</div>
          <div className="kn-panel-subtitle">
            {models.length > 0
              ? `${models.length} local model${models.length === 1 ? "" : "s"} available`
              : loading
                ? "Loading local models..."
                : "Delete local Ollama models"}
          </div>
        </div>
      </div>

      <div className="kn-scroll flex-1 overflow-y-auto px-2 py-2">
        {models.length === 0 ? (
          <div className="flex h-full min-h-[200px] items-center justify-center px-4 text-center text-xs text-[color:var(--kn-text-faint)]">
            {loading ? "Loading local models..." : "No local models to remove."}
          </div>
        ) : (
          <div className="space-y-1">
            {models.map((model, index) => {
              const isSelected = index === selected;
              const isConfirming = confirming === model.name;
              const isDeleting = deleting === model.name;
              return (
                <div key={model.name} className="space-y-1">
                  <button
                    type="button"
                    onMouseEnter={() => setSelected(index)}
                    onMouseDown={() => {
                      setSelected(index);
                      setConfirming(isConfirming ? null : model.name);
                    }}
                    className={`kn-result-row flex w-full items-center gap-3 px-3 py-2.5 text-left ${
                      isConfirming ? "border-red-400/25 bg-[color:var(--kn-danger-wash)]" : ""
                    }`}
                    data-selected={isSelected}
                  >
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-sm font-medium text-[color:var(--kn-text)]">
                        {model.name}
                      </span>
                      <span className="block text-xs text-[color:var(--kn-text-faint)]">
                        {model.active ? "Currently active" : "Local model"}
                      </span>
                    </span>
                    <span className="shrink-0 text-xs text-[color:var(--kn-text-muted)]">
                      {typeof model.size_gb === "number" ? `${model.size_gb.toFixed(1)} GB` : "Local"}
                    </span>
                    {isDeleting && (
                      <span className="shrink-0 text-xs text-red-300">Removing...</span>
                    )}
                    {isConfirming && !isDeleting && (
                      <span className="shrink-0 text-xs text-[color:var(--kn-warm)]">Press Enter</span>
                    )}
                  </button>

                  {isConfirming && !isDeleting && (
                    <div className="kn-muted-surface flex items-center gap-3 border-red-400/20 bg-[color:var(--kn-danger-wash)] px-3 py-2">
                      <span className="flex-1 text-xs text-red-100">
                        Delete <span className="font-semibold">{model.name}</span>? This removes the local copy.
                      </span>
                      <button
                        type="button"
                        onMouseDown={() => void deleteModel(model.name)}
                        className="kn-button kn-button-danger px-3 py-1"
                      >
                        Delete
                      </button>
                      <button
                        type="button"
                        onMouseDown={() => setConfirming(null)}
                        className="kn-button px-3 py-1"
                      >
                        Cancel
                      </button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="kn-panel-footer">
        <span>Enter or Delete confirms</span>
        <span className={error ? "text-red-300" : ""}>{error || notice || "Esc backs out"}</span>
      </div>
    </div>
  );
}
