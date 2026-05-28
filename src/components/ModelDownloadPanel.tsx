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

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

const API_OPTIONS: ApiOption[] = [
  {
    kind: "api",
    provider: "claude",
    label: "Claude API",
    model: "claude-sonnet-4-6",
    description: "Use an Anthropic API key for remote chat.",
  },
  {
    kind: "api",
    provider: "openai",
    label: "OpenAI-compatible",
    model: "gpt-4o-mini",
    description: "Point AI Chat at any OpenAI-style endpoint.",
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

function sourceLabel(source: string) {
  switch (source) {
    case "recommended":
      return "Recommended";
    case "catalog":
      return "Catalog";
    case "library":
      return "Ollama";
    default:
      return source.toUpperCase();
  }
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
    return () => {
      cancelled = true;
    };
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
      setNotice(`AI Chat is now using ${event.payload.name}.`);
    });
    const unlistenError = listen<ModelEventPayload>("model-pull-error", (event) => {
      if (event.payload.tool && event.payload.tool !== "ai") return;
      setDownloading(null);
      setProgress(null);
      setError(event.payload.error ?? "The model download failed.");
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

  const activateLocal = useCallback(
    async (name: string) => {
      setError("");
      if (pendingDownload === name) {
        setDownloading(name);
        setNotice(`Downloading ${name}...`);
        await ipcDispatch("model.pull", { name, tool: "ai" });
        return;
      }

      const check = await ipcDispatch<CheckResponse>("model.check", { name });
      if (check.exists) {
        await ipcDispatch("model.set_active", { provider: "ollama", model: name, tool: "ai" });
        setNotice(`AI Chat is now using ${name}.`);
        return;
      }

      setPendingDownload(name);
      setNotice(`Press Enter again to download ${name}.`);
    },
    [pendingDownload],
  );

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
    setNotice(`AI Chat is now using ${apiPrompt.label}.`);
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

  async function activateInputModel() {
    const model = modelInput.trim();
    if (!model || downloading) return;
    await activateLocal(model).catch((err) => setError(String(err)));
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (apiPrompt) {
        setApiPrompt(null);
        rootRef.current?.focus();
      } else {
        onClose();
      }
      return;
    }
    if (apiPrompt) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((i) => Math.min(i + 1, options.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      void activateOption(options[selected]);
    }
  }

  return (
    <div
      ref={rootRef}
      tabIndex={-1}
      onKeyDown={handleKeyDown}
      className="kn-panel-shell flex max-h-[460px] flex-col rounded-t-none border-t-0 outline-none"
    >
      <div className="kn-panel-header">
        <div>
          <div className="kn-panel-title">Model Download</div>
          <div className="kn-panel-subtitle">Pick a local or hosted model for AI Chat</div>
        </div>
        <div className="flex items-center gap-2">
          <span className="kn-chip">RAM {formatMb(hardware?.ram_mb ?? 0)}</span>
          <span className="kn-chip">VRAM {formatMb(hardware?.vram_mb ?? 0)}</span>
        </div>
      </div>

      <div className="border-b border-[color:var(--kn-border)] px-4 py-3">
        <div className="kn-section-label mb-2">Custom Model</div>
        <div className="flex gap-2">
          <input
            value={modelInput}
            onChange={(e) => setModelInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                void activateInputModel();
              }
            }}
            placeholder="qwen2.5:1.5b or an Ollama library URL"
            className="kn-field min-w-0 flex-1 text-sm"
          />
          <button
            type="button"
            onClick={() => void activateInputModel()}
            disabled={!modelInput.trim() || Boolean(downloading)}
            className="kn-button kn-button-primary px-3 disabled:opacity-40"
          >
            Use
          </button>
        </div>
      </div>

      <div className="kn-scroll flex-1 overflow-y-auto px-2 py-2">
        <div className="space-y-1">
          {options.map((option, index) => {
            const isSelected = index === selected;
            const isPending = option.kind === "local" && pendingDownload === option.name;
            const isDownloading = option.kind === "local" && downloading === option.name;
            const sourceText = option.kind === "local" ? sourceLabel(option.source) : "API";

            return (
              <button
                key={option.kind === "local" ? option.name : option.provider}
                ref={(el) => {
                  if (index === selected && el) el.scrollIntoView({ block: "nearest" });
                }}
                type="button"
                onMouseEnter={() => setSelected(index)}
                onMouseDown={() => {
                  setSelected(index);
                  void activateOption(option);
                }}
                className="kn-result-row flex w-full items-center gap-3 px-3 py-2.5 text-left"
                data-selected={isSelected}
              >
                <span className={`kn-chip ${isPending ? "kn-chip-active" : ""}`}>{sourceText}</span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm font-medium text-[color:var(--kn-text)]">
                    {option.kind === "local" ? option.name : option.label}
                  </span>
                  <span className="block truncate text-xs text-[color:var(--kn-text-faint)]">
                    {option.kind === "local" ? option.rating : option.description}
                  </span>
                </span>
                <span className="shrink-0 text-xs text-[color:var(--kn-text-muted)]">
                  {option.kind === "local"
                    ? option.size_gb > 0
                      ? `${option.size_gb.toFixed(1)} GB`
                      : "Unknown size"
                    : option.model}
                </span>
                {isPending && (
                  <span className="shrink-0 text-xs text-[color:var(--kn-warm)]">Press Enter</span>
                )}
                {isDownloading && (
                  <span className="shrink-0 text-xs text-[color:var(--kn-success)]">
                    {progressText(progress)}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      </div>

      {apiPrompt && (
        <div className="border-t border-[color:var(--kn-border)] px-4 py-3">
          <div className="kn-section-label mb-2">{apiPrompt.label} Key</div>
          <div className="flex gap-2">
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
              placeholder="Paste API key"
              className="kn-field w-full text-sm"
            />
            <button
              type="button"
              onClick={() => void activateApi().catch((err) => setError(String(err)))}
              disabled={!apiKey.trim()}
              className="kn-button kn-button-primary px-3 disabled:opacity-40"
            >
              Save
            </button>
          </div>
        </div>
      )}

      {progress && typeof progress.percent === "number" && (
        <div className="h-1 bg-white/[0.04]">
          <div
            className="h-full bg-[color:var(--kn-success)] transition-all"
            style={{ width: `${progress.percent}%` }}
          />
        </div>
      )}

      <div className="kn-panel-footer">
        <span>Use arrows and Enter</span>
        <span className={error ? "text-red-300" : ""}>{error || notice || "Esc closes"}</span>
      </div>
    </div>
  );
}
