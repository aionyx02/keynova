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
      setNotice(`已刪除 ${name}`);
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
      if (confirming) {
        setConfirming(null);
      } else {
        onClose();
      }
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
      className="min-h-[300px] bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl outline-none"
    >
      <div className="flex items-center justify-between border-b border-gray-700/50 px-4 py-2">
        <span className="text-xs font-semibold uppercase tracking-wide text-red-400">
          Remove Model
        </span>
        <span className="text-[11px] text-gray-500">
          {models.length > 0
            ? `${models.length} local model${models.length !== 1 ? "s" : ""}`
            : ""}
        </span>
      </div>

      <div className="max-h-[320px] overflow-y-auto py-1">
        {models.length === 0 && (
          <p className="px-4 py-6 text-center text-xs text-gray-600">
            {loading ? "讀取模型中…" : "沒有本機模型可刪除"}
          </p>
        )}
        {models.map((model, index) => {
          const isSelected = index === selected;
          const isConfirming = confirming === model.name;
          const isDeleting = deleting === model.name;
          return (
            <div key={model.name}>
              <button
                type="button"
                onMouseEnter={() => setSelected(index)}
                onMouseDown={() => {
                  setSelected(index);
                  setConfirming(isConfirming ? null : model.name);
                }}
                className={`flex w-full items-center gap-3 px-4 py-2.5 text-left text-sm transition-colors ${
                  isSelected
                    ? isConfirming
                      ? "bg-red-600/30 text-white"
                      : "bg-blue-600/70 text-white"
                    : "text-gray-300 hover:bg-white/8"
                }`}
              >
                <span className="min-w-0 flex-1">
                  <span className="block truncate font-medium">{model.name}</span>
                  {model.active && (
                    <span className="block text-[10px] text-emerald-300">使用中</span>
                  )}
                </span>
                <span className="shrink-0 text-xs text-gray-500">
                  {typeof model.size_gb === "number"
                    ? `${model.size_gb.toFixed(1)} GB`
                    : "Local"}
                </span>
                {isDeleting && (
                  <span className="shrink-0 animate-pulse text-[11px] text-red-400">
                    刪除中…
                  </span>
                )}
                {isConfirming && !isDeleting && (
                  <span className="shrink-0 text-[11px] text-amber-300">按 Enter 確認</span>
                )}
              </button>

              {isConfirming && !isDeleting && (
                <div className="flex items-center gap-2 border-t border-red-500/20 bg-red-900/20 px-4 py-2">
                  <span className="flex-1 text-xs text-red-300">
                    確認刪除{" "}
                    <span className="font-semibold text-white">{model.name}</span>？此操作無法復原。
                  </span>
                  <button
                    type="button"
                    onMouseDown={() => void deleteModel(model.name)}
                    className="rounded bg-red-600/80 px-3 py-1 text-xs font-medium text-white hover:bg-red-500"
                  >
                    刪除
                  </button>
                  <button
                    type="button"
                    onMouseDown={() => setConfirming(null)}
                    className="rounded bg-gray-700 px-3 py-1 text-xs text-gray-300 hover:bg-gray-600"
                  >
                    取消
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>

      <div className="flex justify-between border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600">
        <span>↑↓ 選擇 · Enter/Del 確認刪除 · Esc 取消/關閉</span>
        <span className={error ? "text-red-400" : "text-emerald-400"}>{error || notice}</span>
      </div>
    </div>
  );
}