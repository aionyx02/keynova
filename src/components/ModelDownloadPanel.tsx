import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { PanelProps } from "../types/panel";

interface HardwareInfo {
  ram_mb: number;
  vram_mb: number;
}

interface ModelCandidate {
  name: string;
  provider: string;
  size_gb: number;
  rating: string;
  source: string;
}

interface CheckResponse {
  exists: boolean;
}

interface ModelEventPayload {
  name: string;
  tool?: string;
  status?: string;
  completed_bytes?: number | null;
  total_bytes?: number | null;
  percent?: number | null;
  error?: string;
}

interface CatalogUpdatedPayload {
  models: ModelCandidate[];
}

interface ApiOption {
  kind: "api";
  provider: "claude" | "openai";
  label: string;
  model: string;
  description: string;
}

interface LocalOption extends ModelCandidate {
  kind: "local";
}

type DownloadOption = LocalOption | ApiOption;
const AI_TOOL_LABEL = "AI Chat";

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

const API_OPTIONS: ApiOption[] = [
  {
    kind: "api",
    provider: "claude",
    label: "Claude API",
    model: "claude-sonnet-4-6",
    description: "需要 Anthropic API key",
  },
  {
    kind: "api",
    provider: "openai",
    label: "OpenAI-compatible",
    model: "gpt-4o-mini",
    description: "可使用 OpenAI 或相容服務",
  },
];

function formatMb(mb: number) {
  if (!mb) return "Unknown";
  return `${Math.round(mb / 1024)} GB`;
}

function progressText(payload: ModelEventPayload | null) {
  if (!payload) return "";
  if (typeof payload.percent === "number") return `${Math.round(payload.percent)}%`;
  return payload.status ?? "pulling";
}

function mergeCatalog(current: ModelCandidate[], incoming: ModelCandidate[]) {
  const currentByName = new Map(current.map((model) => [model.name, model]));
  const incomingNames = new Set(incoming.map((model) => model.name));
  const merged = incoming.map((model) => {
    const previous = currentByName.get(model.name);
    return {
      ...(previous ?? {}),
      ...model,
      size_gb: model.size_gb > 0 ? model.size_gb : previous?.size_gb ?? 0,
    };
  });
  current.forEach((model) => {
    if (!incomingNames.has(model.name)) merged.push(model);
  });
  return merged;
}

export function ModelDownloadPanel({ onClose }: PanelProps) {
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [candidates, setCandidates] = useState<ModelCandidate[]>([]);
  const [selected, setSelected] = useState(0);
  const [pendingDownload, setPendingDownload] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);
  const [progress, setProgress] = useState<ModelEventPayload | null>(null);
  const [apiPrompt, setApiPrompt] = useState<ApiOption | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [modelInput, setModelInput] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const rootRef = useRef<HTMLDivElement>(null);
  const apiInputRef = useRef<HTMLInputElement>(null);

  const options = useMemo<DownloadOption[]>(
    () => [
      ...candidates.map((candidate) => ({ ...candidate, kind: "local" as const })),
      ...API_OPTIONS,
    ],
    [candidates],
  );

  useEffect(() => {
    rootRef.current?.focus();
    let cancelled = false;
    async function load() {
      try {
        const [hw, recommended] = await Promise.all([
          ipcDispatch<HardwareInfo>("model.detect_hardware"),
          ipcDispatch<ModelCandidate[]>("model.recommend"),
        ]);
        if (cancelled) return;
        setHardware(hw);
        setCandidates(recommended);
      } catch (err) {
        if (!cancelled) setError(String(err));
      }
    }
    void load();
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlistenCatalog = listen<CatalogUpdatedPayload>("model-catalog-updated", (event) => {
      setCandidates((current) => mergeCatalog(current, event.payload.models));
    });
    const unlistenProgress = listen<ModelEventPayload>("model-pull-progress", (event) => {
      if (event.payload.tool && event.payload.tool !== "ai") return;
      setProgress(event.payload);
      setNotice(`${event.payload.name} ${progressText(event.payload)}`);
    });
    const unlistenDone = listen<ModelEventPayload>("model-pull-done", (event) => {
      if (event.payload.tool && event.payload.tool !== "ai") return;
      setDownloading(null);
      setPendingDownload(null);
      setProgress(null);
      setError("");
      setNotice(`✓ 已為 ${AI_TOOL_LABEL} 啟用 ${event.payload.name}`);
    });
    const unlistenError = listen<ModelEventPayload>("model-pull-error", (event) => {
      if (event.payload.tool && event.payload.tool !== "ai") return;
      setDownloading(null);
      setProgress(null);
      setError(event.payload.error ?? "模型下載失敗");
    });
    return () => {
      unlistenCatalog.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
      unlistenDone.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    apiInputRef.current?.focus();
  }, [apiPrompt]);

  const activateLocal = useCallback(async (name: string) => {
    setError("");
    if (pendingDownload === name) {
      setDownloading(name);
      setNotice(`開始下載 ${name}`);
      await ipcDispatch("model.pull", { name, tool: "ai" });
      return;
    }

    const check = await ipcDispatch<CheckResponse>("model.check", { name });
    if (check.exists) {
      await ipcDispatch("model.set_active", { provider: "ollama", model: name, tool: "ai" });
      setNotice(`✓ 已為 ${AI_TOOL_LABEL} 啟用 ${name}`);
      return;
    }

    setPendingDownload(name);
    setNotice(`${name} 尚未下載，再按 Enter 開始下載`);
  }, [pendingDownload]);

  const activateApi = useCallback(async () => {
    if (!apiPrompt || !apiKey.trim()) return;
    setError("");
    await ipcDispatch("model.set_active", {
      provider: apiPrompt.provider,
      model: apiPrompt.model,
      api_key: apiKey.trim(),
      tool: "ai",
    });
    setApiPrompt(null);
    setApiKey("");
    setNotice(`✓ 已為 ${AI_TOOL_LABEL} 啟用 ${apiPrompt.label}`);
  }, [apiKey, apiPrompt]);

  async function activateOption(option: DownloadOption | undefined) {
    if (!option || downloading) return;
    if (option.kind === "api") {
      setApiPrompt(option);
      setApiKey("");
      setNotice("");
      return;
    }
    await activateLocal(option.name).catch((err) => setError(String(err)));
  }

  async function activateSelected() {
    await activateOption(options[selected]);
  }

  async function activateInputModel() {
    const model = modelInput.trim();
    if (!model || downloading) return;
    await activateLocal(model).catch((err) => setError(String(err)));
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLDivElement>) {
    if (apiPrompt) return;
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, options.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      void activateSelected();
    }
  }

  return (
    <div
      ref={rootRef}
      tabIndex={-1}
      onKeyDown={handleKeyDown}
      className="min-h-[350px] bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl outline-none"
    >
      <div className="flex items-center justify-between border-b border-gray-700/50 px-4 py-2">
        <span className="text-xs font-semibold uppercase tracking-wide text-blue-400">Model Download</span>
        <div className="flex items-center gap-2">
          <div className="flex gap-2 text-[11px] text-gray-500">
          <span>RAM {formatMb(hardware?.ram_mb ?? 0)}</span>
          <span>VRAM {formatMb(hardware?.vram_mb ?? 0)}</span>
          </div>
        </div>
      </div>

      <div className="max-h-[390px] overflow-y-auto py-1">
        <div className="border-b border-gray-700/40 px-4 py-3">
          <label className="mb-1 block text-xs text-gray-500">模型名稱或 Ollama URL</label>
          <div className="flex gap-2">
            <input
              value={modelInput}
              onChange={(e) => setModelInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  e.preventDefault();
                  onClose();
                } else if (e.key === "Enter") {
                  e.preventDefault();
                  void activateInputModel();
                }
              }}
              placeholder="qwen2.5:1.5b 或 https://ollama.com/library/gemma4:e2b"
              className="min-w-0 flex-1 rounded bg-gray-800 px-3 py-2 text-xs text-gray-100 outline-none placeholder-gray-600 focus:ring-1 focus:ring-blue-500"
            />
            <button
              type="button"
              onClick={() => void activateInputModel()}
              disabled={!modelInput.trim() || Boolean(downloading)}
              className="rounded bg-blue-600/70 px-3 py-1 text-xs text-white transition-colors hover:bg-blue-500/70 disabled:opacity-40"
            >
              使用
            </button>
          </div>
        </div>
        {options.map((option, index) => {
          const selectedClass = index === selected ? "bg-blue-600/70 text-white" : "text-gray-300 hover:bg-white/8";
          const isPending = option.kind === "local" && pendingDownload === option.name;
          const isDownloading = option.kind === "local" && downloading === option.name;
          return (
            <button
              key={option.kind === "local" ? option.name : option.provider}
              ref={(el) => {if (index === selected && el) el.scrollIntoView({block: "nearest"})}}
              type="button"
              onMouseEnter={() => setSelected(index)}
              onMouseDown={() => { setSelected(index); void activateOption(option); }}
              className={`flex w-full items-center gap-3 px-4 py-2.5 text-left text-sm transition-colors ${selectedClass}`}
            >
              <span className="w-16 shrink-0 rounded bg-gray-800/80 px-2 py-1 text-center text-[10px] font-semibold uppercase text-gray-400">
                {option.kind === "local" ? option.source : "API"}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate font-medium">
                  {option.kind === "local" ? option.name : option.label}
                </span>
                <span className="block truncate text-xs text-gray-500">
                  {option.kind === "local" ? option.rating : option.description}
                </span>
              </span>
              <span className="shrink-0 text-xs text-gray-500">
                {option.kind === "local"
                  ? option.size_gb > 0 ? `${option.size_gb.toFixed(1)} GB` : "Size unknown"
                  : option.model}
              </span>
              {isPending && <span className="text-xs text-amber-300">Enter 下載</span>}
              {isDownloading && <span className="text-xs text-emerald-300">{progressText(progress)}</span>}
            </button>
          );
        })}
      </div>

      {apiPrompt && (
        <div className="border-t border-gray-700/50 px-4 py-3">
          <label className="mb-1 block text-xs text-gray-400">{apiPrompt.label} API key</label>
          <input
            ref={apiInputRef}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                setApiPrompt(null);
                rootRef.current?.focus();
              } else if (e.key === "Enter") {
                e.preventDefault();
                void activateApi().catch((err) => setError(String(err)));
              }
            }}
            type="password"
            className="w-full rounded bg-gray-800 px-3 py-2 text-sm text-gray-100 outline-none focus:ring-1 focus:ring-blue-500"
          />
        </div>
      )}

      {progress && typeof progress.percent === "number" && (
        <div className="h-1 bg-gray-800">
          <div className="h-full bg-emerald-500 transition-all" style={{ width: `${progress.percent}%` }} />
        </div>
      )}

      <div className="flex justify-between border-t border-gray-700/50 px-4 py-1.5 text-[11px] text-gray-600">
        <span>↑↓ 選擇 · Enter 確認 · Esc 關閉</span>
        <span className={error ? "text-red-400" : "text-emerald-400"}>{error || notice}</span>
      </div>
    </div>
  );
}
