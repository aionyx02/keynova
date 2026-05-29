import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PanelProps } from "../../types/panel";

interface LocalModel {
  name: string;
  provider: string;
  size_gb?: number | null;
  active: boolean;
}

interface ApiModel {
  name: string;
  provider: "claude" | "openai";
  model: string;
  configured: boolean;
  active: boolean;
}

interface ModelListResponse {
  tool: string;
  tool_label: string;
  active_provider: string;
  active_model: string;
  local_models: LocalModel[];
  api_models: ApiModel[];
}

type ModelRow =
  | (LocalModel & { kind: "local"; label: string })
  | (ApiModel & { kind: "api"; label: string });

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

function modelSize(model: ModelRow) {
  if (model.kind === "api") return "API";
  return typeof model.size_gb === "number" ? `${model.size_gb.toFixed(1)} GB` : "Local";
}

export function ModelListPanel({ onClose }: PanelProps) {
  const [data, setData] = useState<ModelListResponse | null>(null);
  const [selected, setSelected] = useState(0);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  const rows = useMemo<ModelRow[]>(() => {
    if (!data) return [];
    return [
      ...data.local_models.map((model) => ({
        ...model,
        kind: "local" as const,
        label: model.name,
      })),
      ...data.api_models.map((model) => ({
        ...model,
        kind: "api" as const,
        label: model.name,
      })),
    ];
  }, [data]);

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const next = await ipcDispatch<ModelListResponse>("model.list_available", {
        tool: "ai",
      });
      setData(next);
      setSelected((i) => Math.min(i, Math.max(next.local_models.length + next.api_models.length - 1, 0)));
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

  async function activateRow(row: ModelRow) {
    setError("");
    if (row.kind === "api" && !row.configured) {
      setNotice(`Open /model_download to configure ${row.label}.`);
      return;
    }
    const payload =
      row.kind === "local"
        ? { provider: "ollama", model: row.name, tool: "ai" }
        : { provider: row.provider, model: row.model, tool: "ai" };
    await ipcDispatch("model.set_active", payload);
    setNotice(`AI Chat is now using ${row.kind === "local" ? row.name : row.label}.`);
    await load();
  }

  async function deleteRow(row: ModelRow) {
    if (row.kind !== "local") return;
    setError("");
    await ipcDispatch("model.delete", { name: row.name });
    setNotice(`Removed ${row.name}.`);
    await load();
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, rows.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const row = rows[selected];
      if (row) void activateRow(row).catch((err) => setError(String(err)));
    } else if (e.key === "Delete") {
      e.preventDefault();
      const row = rows[selected];
      if (row) void deleteRow(row).catch((err) => setError(String(err)));
    }
  }

  return (
    <div
      ref={rootRef}
      tabIndex={-1}
      onKeyDown={handleKeyDown}
      className="kn-panel-shell flex min-h-[360px] flex-col rounded-t-none border-t-0 outline-none"
    >
      <div className="kn-panel-header">
        <div>
          <div className="kn-panel-title">Model List</div>
          <div className="kn-panel-subtitle">
            {data
              ? `${data.tool_label}: ${data.active_provider}:${data.active_model}`
              : loading
                ? "Loading available models..."
                : "Inspect the active AI Chat model"}
          </div>
        </div>
      </div>

      <div className="kn-scroll flex-1 overflow-y-auto px-2 py-2">
        {rows.length === 0 ? (
          <div className="flex h-full min-h-[220px] items-center justify-center px-4 text-center text-xs text-[color:var(--kn-text-faint)]">
            {loading ? "Loading available models..." : "No models are currently available."}
          </div>
        ) : (
          <div className="space-y-1">
            {rows.map((row, index) => {
              const isSelected = index === selected;
              const status = row.active
                ? "Active"
                : row.kind === "api" && !row.configured
                  ? "Needs key"
                  : "Available";
              return (
                <button
                  key={`${row.kind}-${row.kind === "local" ? row.name : row.provider}`}
                  type="button"
                  onMouseEnter={() => setSelected(index)}
                  onMouseDown={() => {
                    setSelected(index);
                    void activateRow(row).catch((err) => setError(String(err)));
                  }}
                  className="kn-result-row grid w-full grid-cols-[92px_minmax(0,1fr)_88px_72px] items-center gap-3 px-3 py-2.5 text-left"
                  data-selected={isSelected}
                >
                  <span className="kn-chip justify-center">
                    {row.kind === "local" ? "Ollama" : row.provider}
                  </span>
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium text-[color:var(--kn-text)]">
                      {row.label}
                    </span>
                    <span className="block truncate text-xs text-[color:var(--kn-text-faint)]">
                      {row.kind === "local" ? row.name : row.model}
                    </span>
                  </span>
                  <span className="text-xs text-[color:var(--kn-text-muted)]">{modelSize(row)}</span>
                  <span
                    className={`text-xs ${
                      row.active ? "text-[color:var(--kn-success)]" : "text-[color:var(--kn-text-muted)]"
                    }`}
                  >
                    {status}
                  </span>
                </button>
              );
            })}
          </div>
        )}
      </div>

      <div className="kn-panel-footer">
        <span>Enter activates</span>
        <span className={error ? "text-red-300" : ""}>{error || notice || "Delete removes a local model"}</span>
      </div>
    </div>
  );
}
