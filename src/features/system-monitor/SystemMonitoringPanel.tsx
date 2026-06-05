import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { fmt } from "../../i18n/format";
import { useI18n } from "../../i18n/useI18n";
import type { PanelProps } from "../../types/panel";

interface DiskInfo {
  mount: string;
  used_gb: number;
  total_gb: number;
  pct: number;
}

interface NetworkInfo {
  name: string;
  rx_kbps: number;
  tx_kbps: number;
}

interface ProcessInfo {
  name: string;
  pid: number;
  mem_mb: number;
  cpu_pct: number;
}

interface Snapshot {
  cpu_pct: number;
  ram_used_mb: number;
  ram_total_mb: number;
  disks: DiskInfo[];
  networks: NetworkInfo[];
  processes: ProcessInfo[];
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

function UsageBar({ pct, tone }: { pct: number; tone: "accent" | "warm" | "danger" }) {
  const colorClass =
    tone === "danger"
      ? "bg-[color:var(--kn-danger)]"
      : tone === "warm"
        ? "bg-[color:var(--kn-warm)]"
        : "bg-[color:var(--kn-accent)]";

  return (
    <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/[0.05]">
      <div
        className={`h-full rounded-full transition-all duration-500 ${colorClass}`}
        style={{ width: `${Math.min(100, pct)}%` }}
      />
    </div>
  );
}

export function SystemMonitoringPanel({ onClose }: PanelProps) {
  const t = useI18n().systemMonitor;
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState("");
  const [sortBy, setSortBy] = useState<"mem" | "cpu">("mem");
  const rootRef = useRef<HTMLDivElement>(null);

  const stopStream = useCallback(async () => {
    if (!window.__TAURI_INTERNALS__) return;
    await ipcDispatch("system_monitoring.stream_stop").catch(() => {});
    setStreaming(false);
  }, []);

  useEffect(() => {
    rootRef.current?.focus();
    if (!window.__TAURI_INTERNALS__) return;

    void ipcDispatch<Snapshot>("system_monitoring.snapshot")
      .then(setSnap)
      .catch((e: unknown) => setError(String(e)));

    void ipcDispatch("system_monitoring.stream_start", { interval_ms: 2000 })
      .then(() => setStreaming(true))
      .catch((e: unknown) => setError(String(e)));

    let unlisten: (() => void) | undefined;
    void listen<Snapshot>("system-monitoring-tick", (event) => {
      setSnap(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
      void stopStream();
    };
  }, [stopStream]);

  const ramPct = snap ? (snap.ram_used_mb / snap.ram_total_mb) * 100 : 0;
  const activeNetworks = snap?.networks.filter((net) => net.rx_kbps > 0 || net.tx_kbps > 0) ?? [];

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
      className="kn-panel-shell flex max-h-[580px] flex-col rounded-t-none border-t-0 outline-none"
    >
      <div className="kn-panel-header">
        <div>
          <div className="kn-panel-title">{t.title}</div>
          <div className="kn-panel-subtitle">{t.subtitle}</div>
        </div>
        <span className={`kn-chip ${streaming ? "kn-chip-active" : ""}`}>
          {streaming ? t.live : t.stopped}
        </span>
      </div>

      <div className="kn-scroll flex-1 overflow-y-auto px-4 py-3">
        {error && (
          <div className="kn-muted-surface mb-3 border-red-400/20 bg-[color:var(--kn-danger-wash)] px-3 py-2 text-xs text-red-100">
            {error}
          </div>
        )}

        {!snap ? (
          <div className="flex min-h-[260px] items-center justify-center text-xs text-[color:var(--kn-text-faint)]">
            {t.loading}
          </div>
        ) : (
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-3">
              <div className="kn-muted-surface p-3">
                <div className="mb-2 flex items-center justify-between">
                  <span className="kn-section-label">CPU</span>
                  <span className="font-mono text-xs text-[color:var(--kn-text-soft)]">
                    {snap.cpu_pct.toFixed(1)}%
                  </span>
                </div>
                <UsageBar pct={snap.cpu_pct} tone={snap.cpu_pct >= 85 ? "danger" : "accent"} />
              </div>
              <div className="kn-muted-surface p-3">
                <div className="mb-2 flex items-center justify-between">
                  <span className="kn-section-label">{t.memory}</span>
                  <span className="font-mono text-xs text-[color:var(--kn-text-soft)]">
                    {snap.ram_used_mb.toLocaleString()} / {snap.ram_total_mb.toLocaleString()} MB
                  </span>
                </div>
                <UsageBar pct={ramPct} tone={ramPct >= 85 ? "danger" : "warm"} />
              </div>
            </div>

            {snap.disks.length > 0 && (
              <div>
                <div className="kn-section-label mb-2">{t.disks}</div>
                <div className="space-y-2">
                  {snap.disks.map((disk) => (
                    <div key={disk.mount} className="kn-muted-surface px-3 py-2.5">
                      <div className="mb-1.5 flex items-center justify-between gap-3 text-xs">
                        <span className="truncate font-mono text-[color:var(--kn-text-soft)]">
                          {disk.mount}
                        </span>
                        <span className="text-[color:var(--kn-text-muted)]">
                          {disk.used_gb.toFixed(1)} / {disk.total_gb.toFixed(1)} GB
                        </span>
                      </div>
                      <UsageBar pct={disk.pct} tone={disk.pct >= 90 ? "danger" : "accent"} />
                    </div>
                  ))}
                </div>
              </div>
            )}

            {activeNetworks.length > 0 && (
              <div>
                <div className="kn-section-label mb-2">{t.network}</div>
                <div className="space-y-1">
                  {activeNetworks.map((network) => (
                    <div
                      key={network.name}
                      className="kn-muted-surface flex items-center gap-3 px-3 py-2 text-xs"
                    >
                      <span className="min-w-0 flex-1 truncate font-mono text-[color:var(--kn-text-soft)]">
                        {network.name}
                      </span>
                      <span className="shrink-0 text-[color:var(--kn-accent)]">
                        {fmt(t.down, { value: network.rx_kbps.toFixed(0) })}
                      </span>
                      <span className="shrink-0 text-[color:var(--kn-success)]">
                        {fmt(t.up, { value: network.tx_kbps.toFixed(0) })}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div>
              <div className="mb-2 flex items-center justify-between">
                <div className="kn-section-label">{t.topProcesses}</div>
                <div className="flex gap-1.5">
                  <button
                    type="button"
                    onClick={() => setSortBy("mem")}
                    className={`kn-button px-2 py-1 text-[10px] ${sortBy === "mem" ? "kn-button-primary" : ""}`}
                  >
                    RAM
                  </button>
                  <button
                    type="button"
                    onClick={() => setSortBy("cpu")}
                    className={`kn-button px-2 py-1 text-[10px] ${sortBy === "cpu" ? "kn-button-primary" : ""}`}
                  >
                    CPU
                  </button>
                </div>
              </div>
              <div className="space-y-1">
                {[...snap.processes]
                  .sort((a, b) => (sortBy === "cpu" ? b.cpu_pct - a.cpu_pct : b.mem_mb - a.mem_mb))
                  .slice(0, 15)
                  .map((process) => (
                    <div
                      key={process.pid}
                      className="kn-muted-surface grid grid-cols-[minmax(0,1fr)_68px_90px_70px] items-center gap-3 px-3 py-2 text-xs"
                    >
                      <span className="truncate text-[color:var(--kn-text-soft)]">
                        {process.name}
                      </span>
                      <span className="font-mono text-[color:var(--kn-text-faint)]">
                        {process.pid}
                      </span>
                      <span className="font-mono text-[color:var(--kn-warm)]">
                        {process.mem_mb} MB
                      </span>
                      <span className="font-mono text-[color:var(--kn-accent)]">
                        {process.cpu_pct.toFixed(1)}%
                      </span>
                    </div>
                  ))}
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="kn-panel-footer">
        <span>{t.escCloses}</span>
        <span>{streaming ? t.updatesEveryTwoSeconds : t.streamPaused}</span>
      </div>
    </div>
  );
}
