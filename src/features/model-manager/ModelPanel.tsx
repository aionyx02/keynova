import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { PanelProps } from "../../types/panel";

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

type TabId = "installed" | "browse" | "remove";

const TABS: { id: TabId; label: string }[] = [
  { id: "installed", label: "Installed" },
  { id: "browse", label: "Browse" },
  { id: "remove", label: "Remove" },
];

interface ViewProps {
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// Installed — list + switch active model (Del removes a local model)
// ---------------------------------------------------------------------------

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

function modelSize(model: ModelRow) {
  if (model.kind === "api") return "API";
  return typeof model.size_gb === "number" ? `${model.size_gb.toFixed(1)} GB` : "Local";
}

function InstalledView({ onClose }: ViewProps) {
  const [data, setData] = useState<ModelListResponse | null>(null);
  const [selected, setSelected] = useState(0);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  const rows = useMemo<ModelRow[]>(() => {
    if (!data) return [];
    return [
      ...data.local_models.map((model) => ({ ...model, kind: "local" as const, label: model.name })),
      ...data.api_models.map((model) => ({ ...model, kind: "api" as const, label: model.name })),
    ];
  }, [data]);

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const next = await ipcDispatch<ModelListResponse>("model.list_available", { tool: "ai" });
      setData(next);
      setSelected((i) =>
        Math.min(i, Math.max(next.local_models.length + next.api_models.length - 1, 0)),
      );
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
      setNotice(`Use the Browse tab to configure ${row.label}.`);
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
    <div ref={rootRef} tabIndex={-1} onKeyDown={handleKeyDown} className="flex min-h-0 flex-1 flex-col outline-none">
      <div className="kn-panel-subtitle px-4 py-2">
        {data
          ? `${data.tool_label}: ${data.active_provider}:${data.active_model}`
          : loading
            ? "Loading available models..."
            : "Inspect the active AI Chat model"}
      </div>

      <div className="kn-scroll min-h-0 flex-1 overflow-y-auto px-2 py-2">
        {rows.length === 0 ? (
          <div className="flex h-full min-h-[200px] items-center justify-center px-4 text-center text-xs text-[color:var(--kn-text-faint)]">
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
        <span>Enter activates · Delete removes a local model</span>
        <span className={error ? "text-red-300" : ""}>{error || notice || "Tab switches view"}</span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Browse — download a local model or configure a hosted API
// ---------------------------------------------------------------------------

interface HardwareInfo {
  ram_mb: number;
  vram_mb: number;
}

interface BootstrapHardwareInfo extends HardwareInfo {
  cpu_cores: number;
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

interface BootstrapSnapshot {
  status: string;
  hardware: BootstrapHardwareInfo;
  model: {
    ollama_url: string;
    ollama_reachable: boolean;
    recommended_models: ModelCandidate[];
  };
  warnings: string[];
  errors: string[];
}

interface BootstrapStatusPayload {
  running: boolean;
  stale: boolean;
  snapshot: BootstrapSnapshot | null;
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

function applyBootstrapState(
  payload: BootstrapStatusPayload,
  setHardware: Dispatch<SetStateAction<HardwareInfo | null>>,
  setCandidates: Dispatch<SetStateAction<ModelCandidate[]>>,
  setNotice: Dispatch<SetStateAction<string>>,
  setError: Dispatch<SetStateAction<string>>,
) {
  const snapshot = payload.snapshot;
  if (snapshot) {
    setHardware(snapshot.hardware);
    if (snapshot.model.recommended_models.length > 0) {
      setCandidates(snapshot.model.recommended_models);
    }
    if (snapshot.errors.length > 0) {
      setError(snapshot.errors[0]);
      return;
    }
    setError("");
    if (!snapshot.model.ollama_reachable) {
      setNotice(`Ollama is offline at ${snapshot.model.ollama_url}. Recommendations are cached.`);
      return;
    }
    if (payload.running) {
      setNotice("Refreshing local model bootstrap…");
      return;
    }
    if (snapshot.warnings.length > 0) {
      setNotice(snapshot.warnings[0]);
      return;
    }
    setNotice("Local model bootstrap ready.");
    setError("");
    return;
  }

  if (payload.running) {
    setNotice("Preparing local model bootstrap…");
    setError("");
    return;
  }

  setNotice("Bootstrap snapshot unavailable. Press Esc and reopen if it stays empty.");
  setError("");
}

function BrowseView({ onClose }: ViewProps) {
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
        const bootstrap = await ipcDispatch<BootstrapStatusPayload>("model.bootstrap_snapshot");
        if (cancelled) return;
        applyBootstrapState(bootstrap, setHardware, setCandidates, setNotice, setError);
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
    const unlistenPreflight = listen<BootstrapStatusPayload>("startup-preflight-updated", (event) => {
      applyBootstrapState(event.payload, setHardware, setCandidates, setNotice, setError);
    });
    const unlistenPreflightError = listen<BootstrapStatusPayload>("startup-preflight-failed", (event) => {
      applyBootstrapState(event.payload, setHardware, setCandidates, setNotice, setError);
    });
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
      unlistenPreflight.then((fn) => fn());
      unlistenPreflightError.then((fn) => fn());
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
    <div ref={rootRef} tabIndex={-1} onKeyDown={handleKeyDown} className="flex min-h-0 flex-1 flex-col outline-none">
      <div className="flex items-center justify-between gap-2 px-4 py-2">
        <span className="kn-panel-subtitle">Pick a local or hosted model for AI Chat</span>
        <span className="flex items-center gap-2">
          <span className="kn-chip">RAM {formatMb(hardware?.ram_mb ?? 0)}</span>
          <span className="kn-chip">VRAM {formatMb(hardware?.vram_mb ?? 0)}</span>
        </span>
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

      <div className="kn-scroll min-h-0 flex-1 overflow-y-auto px-2 py-2">
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
        <span>Arrows and Enter · Tab switches view</span>
        <span className={error ? "text-red-300" : ""}>{error || notice || "Esc closes"}</span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Remove — delete local Ollama models with a confirm step
// ---------------------------------------------------------------------------

function RemoveView({ onClose }: ViewProps) {
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
    <div ref={rootRef} tabIndex={-1} onKeyDown={handleKeyDown} className="flex min-h-0 flex-1 flex-col outline-none">
      <div className="kn-panel-subtitle px-4 py-2">
        {models.length > 0
          ? `${models.length} local model${models.length === 1 ? "" : "s"} available`
          : loading
            ? "Loading local models..."
            : "Delete local Ollama models"}
      </div>

      <div className="kn-scroll min-h-0 flex-1 overflow-y-auto px-2 py-2">
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
                    {isDeleting && <span className="shrink-0 text-xs text-red-300">Removing...</span>}
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
        <span>Enter or Delete confirms · Tab switches view</span>
        <span className={error ? "text-red-300" : ""}>{error || notice || "Esc backs out"}</span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shell — tab bar that hosts the three views
// ---------------------------------------------------------------------------

export function ModelPanel({ onClose }: PanelProps) {
  const [activeTab, setActiveTab] = useState<TabId>("installed");

  const cycleTab = useCallback((direction: 1 | -1) => {
    setActiveTab((current) => {
      const index = TABS.findIndex((tab) => tab.id === current);
      const next = (index + direction + TABS.length) % TABS.length;
      return TABS[next].id;
    });
  }, []);

  function handleKeyDown(e: React.KeyboardEvent<HTMLDivElement>) {
    if (e.key !== "Tab") return;
    const tag = (e.target as HTMLElement).tagName;
    if (tag === "INPUT" || tag === "TEXTAREA") return;
    e.preventDefault();
    cycleTab(e.shiftKey ? -1 : 1);
  }

  return (
    <div
      onKeyDown={handleKeyDown}
      className="kn-panel-shell flex max-h-[460px] min-h-[380px] flex-col rounded-t-none border-t-0 outline-none"
    >
      <div className="kn-panel-header">
        <div className="kn-panel-title">Models</div>
        <div className="flex items-center gap-1">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onMouseDown={() => setActiveTab(tab.id)}
              className={`kn-chip ${activeTab === tab.id ? "kn-chip-active" : ""}`}
              data-selected={activeTab === tab.id}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {activeTab === "installed" && <InstalledView key="installed" onClose={onClose} />}
      {activeTab === "browse" && <BrowseView key="browse" onClose={onClose} />}
      {activeTab === "remove" && <RemoveView key="remove" onClose={onClose} />}
    </div>
  );
}
