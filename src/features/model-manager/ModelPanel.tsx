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
import { useI18n } from "../../i18n/useI18n";
import { fmt } from "../../i18n/format";
import type { I18nKeys } from "../../i18n/zh-TW";

type ModelT = I18nKeys["model"];

// ---------------------------------------------------------------------------
// Shared
// ---------------------------------------------------------------------------

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

type TabId = "installed" | "browse" | "remove";

const TAB_IDS: TabId[] = ["installed", "browse", "remove"];

function tabLabel(id: TabId, m: ModelT): string {
  return id === "installed" ? m.tabInstalled : id === "browse" ? m.tabBrowse : m.tabRemove;
}

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

function modelSize(model: ModelRow, m: ModelT) {
  if (model.kind === "api") return "API";
  return typeof model.size_gb === "number" ? `${model.size_gb.toFixed(1)} GB` : m.sizeLocal;
}

function InstalledView({ onClose }: ViewProps) {
  const m = useI18n().model;
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
      setNotice(fmt(m.configureInBrowse, { label: row.label }));
      return;
    }
    const payload =
      row.kind === "local"
        ? { provider: "ollama", model: row.name, tool: "ai" }
        : { provider: row.provider, model: row.model, tool: "ai" };
    await ipcDispatch("model.set_active", payload);
    setNotice(fmt(m.activeNow, { name: row.kind === "local" ? row.name : row.label }));
    await load();
  }

  async function deleteRow(row: ModelRow) {
    if (row.kind !== "local") return;
    setError("");
    await ipcDispatch("model.delete", { name: row.name });
    setNotice(fmt(m.removed, { name: row.name }));
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
      className="flex min-h-0 flex-1 flex-col outline-none"
    >
      <div className="kn-panel-subtitle px-4 py-2">
        {data
          ? fmt(m.activeModel, { provider: data.active_provider, model: data.active_model })
          : loading
            ? m.loadingAvailable
            : m.inspectActive}
      </div>

      <div className="kn-scroll min-h-0 flex-1 overflow-y-auto px-2 py-2">
        {rows.length === 0 ? (
          <div className="flex h-full min-h-[200px] items-center justify-center px-4 text-center text-xs text-[color:var(--kn-text-faint)]">
            {loading ? m.loadingAvailable : m.noneAvailable}
          </div>
        ) : (
          <div className="space-y-1">
            {rows.map((row, index) => {
              const isSelected = index === selected;
              const status = row.active
                ? m.statusActive
                : row.kind === "api" && !row.configured
                  ? m.statusNeedsKey
                  : m.statusAvailable;
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
                  <span className="text-xs text-[color:var(--kn-text-muted)]">
                    {modelSize(row, m)}
                  </span>
                  <span
                    className={`text-xs ${
                      row.active
                        ? "text-[color:var(--kn-success)]"
                        : "text-[color:var(--kn-text-muted)]"
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
        <span>{m.installedFooter}</span>
        <span
          role="status"
          aria-live="polite"
          className={error ? "text-red-300" : ""}
        >
          {error || notice || m.tabSwitches}
        </span>
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
  },
  {
    kind: "api",
    provider: "openai",
    label: "OpenAI-compatible",
    model: "gpt-4o-mini",
  },
];

function apiOptionDescription(option: ApiOption, m: ModelT): string {
  return option.provider === "claude" ? m.claudeDesc : m.openaiDesc;
}

function formatMb(mb: number, m: ModelT) {
  if (!mb) return m.unknown;
  return `${Math.round(mb / 1024)} GB`;
}

function progressText(payload: ModelEventPayload | null, m: ModelT) {
  if (!payload) return "";
  if (typeof payload.percent === "number") return `${Math.round(payload.percent)}%`;
  return payload.status ?? m.progressPulling;
}

function sourceLabel(source: string, m: ModelT) {
  switch (source) {
    case "recommended":
      return m.sourceRecommended;
    case "catalog":
      return m.sourceCatalog;
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
      size_gb: model.size_gb > 0 ? model.size_gb : (previous?.size_gb ?? 0),
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
  m: ModelT,
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
      setNotice(fmt(m.ollamaOffline, { url: snapshot.model.ollama_url }));
      return;
    }
    if (payload.running) {
      setNotice(m.refreshingBootstrap);
      return;
    }
    if (snapshot.warnings.length > 0) {
      setNotice(snapshot.warnings[0]);
      return;
    }
    setNotice(m.bootstrapReady);
    setError("");
    return;
  }

  if (payload.running) {
    setNotice(m.preparingBootstrap);
    setError("");
    return;
  }

  setNotice(m.bootstrapUnavailable);
  setError("");
}

function BrowseView({ onClose }: ViewProps) {
  const t = useI18n();
  const m = t.model;
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
        applyBootstrapState(bootstrap, setHardware, setCandidates, setNotice, setError, m);
      } catch (err) {
        if (!cancelled) setError(String(err));
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [m]);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    const unlistenPreflight = listen<BootstrapStatusPayload>(
      "startup-preflight-updated",
      (event) => {
        applyBootstrapState(event.payload, setHardware, setCandidates, setNotice, setError, m);
      },
    );
    const unlistenPreflightError = listen<BootstrapStatusPayload>(
      "startup-preflight-failed",
      (event) => {
        applyBootstrapState(event.payload, setHardware, setCandidates, setNotice, setError, m);
      },
    );
    const unlistenCatalog = listen<CatalogUpdatedPayload>("model-catalog-updated", (event) => {
      setCandidates((current) => mergeCatalog(current, event.payload.models));
    });
    const unlistenProgress = listen<ModelEventPayload>("model-pull-progress", (event) => {
      if (event.payload.tool && event.payload.tool !== "ai") return;
      setProgress(event.payload);
      setNotice(`${event.payload.name} ${progressText(event.payload, m)}`);
    });
    const unlistenDone = listen<ModelEventPayload>("model-pull-done", (event) => {
      if (event.payload.tool && event.payload.tool !== "ai") return;
      setDownloading(null);
      setPendingDownload(null);
      setProgress(null);
      setError("");
      setNotice(fmt(m.activeNow, { name: event.payload.name }));
    });
    const unlistenError = listen<ModelEventPayload>("model-pull-error", (event) => {
      if (event.payload.tool && event.payload.tool !== "ai") return;
      setDownloading(null);
      setProgress(null);
      setError(event.payload.error ?? m.downloadFailed);
    });
    return () => {
      unlistenPreflight.then((fn) => fn());
      unlistenPreflightError.then((fn) => fn());
      unlistenCatalog.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
      unlistenDone.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, [m]);

  useEffect(() => {
    apiInputRef.current?.focus();
  }, [apiPrompt]);

  const activateLocal = useCallback(
    async (name: string) => {
      setError("");
      if (pendingDownload === name) {
        setDownloading(name);
        setNotice(fmt(m.downloading, { name }));
        await ipcDispatch("model.pull", { name, tool: "ai" });
        return;
      }

      const check = await ipcDispatch<CheckResponse>("model.check", { name });
      if (check.exists) {
        await ipcDispatch("model.set_active", { provider: "ollama", model: name, tool: "ai" });
        setNotice(fmt(m.activeNow, { name }));
        return;
      }

      setPendingDownload(name);
      setNotice(fmt(m.pressEnterToDownload, { name }));
    },
    [pendingDownload, m],
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
    setNotice(fmt(m.activeNow, { name: apiPrompt.label }));
  }, [apiKey, apiPrompt, m]);

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
      className="flex min-h-0 flex-1 flex-col outline-none"
    >
      <div className="flex items-center justify-between gap-2 px-4 py-2">
        <span className="kn-panel-subtitle">{m.browseSubtitle}</span>
        <span className="flex items-center gap-2">
          <span className="kn-chip">RAM {formatMb(hardware?.ram_mb ?? 0, m)}</span>
          <span className="kn-chip">VRAM {formatMb(hardware?.vram_mb ?? 0, m)}</span>
        </span>
      </div>

      <div className="border-b border-[color:var(--kn-border)] px-4 py-3">
        <div className="kn-section-label mb-2">{m.customModel}</div>
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
            placeholder={m.customPlaceholder}
            className="kn-field min-w-0 flex-1 text-sm"
          />
          <button
            type="button"
            onClick={() => void activateInputModel()}
            disabled={!modelInput.trim() || Boolean(downloading)}
            className="kn-button kn-button-primary px-3 disabled:opacity-40"
          >
            {m.use}
          </button>
        </div>
      </div>

      <div className="kn-scroll min-h-0 flex-1 overflow-y-auto px-2 py-2">
        <div className="space-y-1">
          {options.map((option, index) => {
            const isSelected = index === selected;
            const isPending = option.kind === "local" && pendingDownload === option.name;
            const isDownloading = option.kind === "local" && downloading === option.name;
            const sourceText = option.kind === "local" ? sourceLabel(option.source, m) : "API";

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
                    {option.kind === "local" ? option.rating : apiOptionDescription(option, m)}
                  </span>
                </span>
                <span className="shrink-0 text-xs text-[color:var(--kn-text-muted)]">
                  {option.kind === "local"
                    ? option.size_gb > 0
                      ? `${option.size_gb.toFixed(1)} GB`
                      : m.unknownSize
                    : option.model}
                </span>
                {isPending && (
                  <span className="shrink-0 text-xs text-[color:var(--kn-warm)]">{m.pressEnter}</span>
                )}
                {isDownloading && (
                  <span className="shrink-0 text-xs text-[color:var(--kn-success)]">
                    {progressText(progress, m)}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      </div>

      {apiPrompt && (
        <div className="border-t border-[color:var(--kn-border)] px-4 py-3">
          <div className="kn-section-label mb-2">{fmt(m.apiKeyLabel, { label: apiPrompt.label })}</div>
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
              placeholder={m.pasteApiKey}
              className="kn-field w-full text-sm"
            />
            <button
              type="button"
              onClick={() => void activateApi().catch((err) => setError(String(err)))}
              disabled={!apiKey.trim()}
              className="kn-button kn-button-primary px-3 disabled:opacity-40"
            >
              {t.note.save}
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
        <span>{m.browseFooter}</span>
        <span role="status" aria-live="polite" className={error ? "text-red-300" : ""}>
          {error || notice || m.escCloses}
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Remove — delete local Ollama models with a confirm step
// ---------------------------------------------------------------------------

function RemoveView({ onClose }: ViewProps) {
  const t = useI18n();
  const m = t.model;
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
      setNotice(fmt(m.removed, { name }));
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
      className="flex min-h-0 flex-1 flex-col outline-none"
    >
      <div className="kn-panel-subtitle px-4 py-2">
        {models.length > 0
          ? fmt(m.localModelsAvailable, { count: models.length })
          : loading
            ? m.loadingLocal
            : m.removeSubtitle}
      </div>

      <div className="kn-scroll min-h-0 flex-1 overflow-y-auto px-2 py-2">
        {models.length === 0 ? (
          <div className="flex h-full min-h-[200px] items-center justify-center px-4 text-center text-xs text-[color:var(--kn-text-faint)]">
            {loading ? m.loadingLocal : m.noneToRemove}
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
                        {model.active ? m.currentlyActive : m.localModel}
                      </span>
                    </span>
                    <span className="shrink-0 text-xs text-[color:var(--kn-text-muted)]">
                      {typeof model.size_gb === "number"
                        ? `${model.size_gb.toFixed(1)} GB`
                        : m.sizeLocal}
                    </span>
                    {isDeleting && (
                      <span className="shrink-0 text-xs text-red-300">{m.removing}</span>
                    )}
                    {isConfirming && !isDeleting && (
                      <span className="shrink-0 text-xs text-[color:var(--kn-warm)]">
                        {m.pressEnter}
                      </span>
                    )}
                  </button>

                  {isConfirming && !isDeleting && (
                    <div className="kn-muted-surface flex items-center gap-3 border-red-400/20 bg-[color:var(--kn-danger-wash)] px-3 py-2">
                      <span className="flex-1 text-xs text-red-100">
                        {fmt(m.removeConfirm, { name: model.name })}
                      </span>
                      <button
                        type="button"
                        onMouseDown={() => void deleteModel(model.name)}
                        className="kn-button kn-button-danger px-3 py-1"
                      >
                        {m.delete}
                      </button>
                      <button
                        type="button"
                        onMouseDown={() => setConfirming(null)}
                        className="kn-button px-3 py-1"
                      >
                        {t.common.cancel}
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
        <span>{m.removeFooter}</span>
        <span role="status" aria-live="polite" className={error ? "text-red-300" : ""}>
          {error || notice || m.escBacksOut}
        </span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shell — tab bar that hosts the three views
// ---------------------------------------------------------------------------

export function ModelPanel({ onClose }: PanelProps) {
  const m = useI18n().model;
  const [activeTab, setActiveTab] = useState<TabId>("installed");

  const cycleTab = useCallback((direction: 1 | -1) => {
    setActiveTab((current) => {
      const index = TAB_IDS.indexOf(current);
      const next = (index + direction + TAB_IDS.length) % TAB_IDS.length;
      return TAB_IDS[next];
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
        <div className="kn-panel-title">{m.title}</div>
        <div className="flex items-center gap-1">
          {TAB_IDS.map((id) => (
            <button
              key={id}
              type="button"
              onMouseDown={() => setActiveTab(id)}
              className={`kn-chip ${activeTab === id ? "kn-chip-active" : ""}`}
              data-selected={activeTab === id}
            >
              {tabLabel(id, m)}
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
