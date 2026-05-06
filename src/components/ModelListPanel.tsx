import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PanelProps } from "../types/panel";

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
type AiTool = "ai" | "translation";

const TOOL_OPTIONS: Array<{ id: AiTool; label: string }> = [
  { id: "ai", label: "AI Chat" },
  { id: "translation", label: "Translation" },
];

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

function modelSize(model: ModelRow) {
  if (model.kind === "api") return "API";
  return typeof model.size_gb === "number" ? `${model.size_gb.toFixed(1)} GB` : "Local";
}

export function ModelListPanel({ onClose }: PanelProps) {
  const [selectedTool, setSelectedTool] = useState<AiTool>("ai");
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
        tool: selectedTool,
      });
      setData(next);
      setSelected((i) => Math.min(i, Math.max(next.local_models.length + next.api_models.length - 1, 0)));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [selectedTool]);

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
      setNotice(`請先用 /model_download 設定 ${row.label}`);
      return;
    }
    const payload = row.kind === "local"
      ? { provider: "ollama", model: row.name, tool: selectedTool }
      : { provider: row.provider, model: row.model, tool: selectedTool };
    await ipcDispatch("model.set_active", payload);
    setNotice(`✓ 已為 ${TOOL_OPTIONS.find((tool) => tool.id === selectedTool)?.label} 啟用 ${row.kind === "local" ? row.name : row.label}`);
    await load();
  }

  async function deleteRow(row: ModelRow) {
    if (row.kind !== "local") return;
    setError("");
    await ipcDispatch("model.delete", { name: row.name });
    setNotice(`已刪除 ${row.name}`);
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
      className=" min-h-[350px] bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl outline-none"
    >
      <div className="flex items-center justify-between border-b border-gray-700/50 px-4 py-2">
        <span className="text-xs font-semibold uppercase tracking-wide text-blue-400">Model List</span>
        <div className="flex items-center gap-2">
          <div className="flex rounded bg-gray-800/70 p-0.5">
            {TOOL_OPTIONS.map((tool) => (
              <button
                key={tool.id}
                type="button"
                onClick={() => {
                  setSelectedTool(tool.id);
                  setNotice("");
                  setError("");
                }}
                className={`rounded px-2 py-0.5 text-[11px] transition-colors ${
                  selectedTool === tool.id ? "bg-blue-600/70 text-white" : "text-gray-500 hover:text-gray-300"
                }`}
              >
                {tool.label}
              </button>
            ))}
          </div>
          <span className="max-w-[210px] truncate text-[11px] text-gray-500">
            {data ? `${data.tool_label}: ${data.active_provider}:${data.active_model}` : loading ? "Loading" : ""}
          </span>
        </div>
      </div>

      <div className="max-h-[360px] overflow-y-auto py-1">
        {rows.length === 0 && (
          <p className="px-4 py-6 text-center text-xs text-gray-600">
            {loading ? "讀取模型中…" : "沒有可用模型"}
          </p>
        )}
        {rows.map((row, index) => {
          const isSelected = index === selected;
          const status = row.active ? "使用中" : row.kind === "api" && !row.configured ? "未設定" : "可用";
          return (
            <button
              key={`${row.kind}-${row.kind === "local" ? row.name : row.provider}`}
              type="button"
              onMouseEnter={() => setSelected(index)}
              onMouseDown={() => { setSelected(index); void activateRow(row).catch((err) => setError(String(err))); }}
              className={`grid w-full grid-cols-[92px_minmax(0,1fr)_88px_72px] items-center gap-3 px-4 py-2.5 text-left text-sm transition-colors ${
                isSelected ? "bg-blue-600/70 text-white" : "text-gray-300 hover:bg-white/8"
              }`}
            >
              <span className="rounded bg-gray-800/80 px-2 py-1 text-center text-[10px] font-semibold uppercase text-gray-400">
                {row.kind === "local" ? "Ollama" : row.provider}
              </span>
              <span className="min-w-0">
                <span className="block truncate font-medium">{row.label}</span>
                <span className="block truncate text-xs text-gray-500">
                  {row.kind === "local" ? row.name : row.model}
                </span>
              </span>
              <span className="text-xs text-gray-500">{modelSize(row)}</span>
              <span className={row.active ? "text-xs text-emerald-300" : "text-xs text-gray-500"}>
                {status}
              </span>
            </button>
          );
        })}
      </div>

      <div className="flex justify-between border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600">
        <span>↑↓ 選擇 · Enter 切換 · Delete 刪除本地模型 · Esc 關閉</span>
        <span className={error ? "text-red-400" : "text-emerald-400"}>{error || notice}</span>
      </div>
    </div>
  );
}
