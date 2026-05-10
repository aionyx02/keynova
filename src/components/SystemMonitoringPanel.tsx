import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PanelProps } from "../types/panel";

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

function UsageBar({ pct }: { pct: number }) {
  const color =
    pct >= 90 ? "bg-red-500" : pct >= 70 ? "bg-amber-500" : "bg-blue-500";
  return (
    <div className="h-1 w-full bg-gray-700 rounded-full overflow-hidden">
      <div
        className={`h-full rounded-full transition-all duration-500 ${color}`}
        style={{ width: `${Math.min(100, pct)}%` }}
      />
    </div>
  );
}

export function SystemMonitoringPanel({ onClose }: PanelProps) {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState("");
  const [sortBy, setSortBy] = useState<"mem" | "cpu">("mem");

  const stopStream = useCallback(async () => {
    if (!window.__TAURI_INTERNALS__) return;
    await ipcDispatch("system_monitoring.stream_stop").catch(() => {});
    setStreaming(false);
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    ipcDispatch<Snapshot>("system_monitoring.snapshot")
      .then(setSnap)
      .catch((e: unknown) => setError(String(e)));

    ipcDispatch("system_monitoring.stream_start", { interval_ms: 2000 })
      .then(() => setStreaming(true))
      .catch((e: unknown) => setError(String(e)));

    let unlisten: (() => void) | undefined;
    listen<Snapshot>("system-monitoring-tick", (e) => {
      setSnap(e.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
      void stopStream();
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const ramPct = snap ? (snap.ram_used_mb / snap.ram_total_mb) * 100 : 0;
  const activeNets = snap?.networks.filter((n) => n.rx_kbps > 0 || n.tx_kbps > 0) ?? [];

  return (
    <div
      className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col p-4 gap-3 max-h-[580px] overflow-y-auto"
      onKeyDown={(e) => { if (e.key === "Escape") { e.preventDefault(); onClose(); } }}
    >
      <div className="flex items-center gap-2">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">System Monitor</span>
        <span className={`ml-auto text-[10px] ${streaming ? "text-green-500" : "text-gray-600"}`}>
          {streaming ? "● Live" : "○ Stopped"}
        </span>
      </div>

      {error && <p className="text-xs text-red-400">{error}</p>}

      {!snap ? (
        <p className="text-xs text-gray-600 text-center py-8">Loading...</p>
      ) : (
        <>
          {/* CPU + RAM */}
          <div className="grid grid-cols-2 gap-3">
            <div className="bg-gray-800/50 rounded-lg p-3">
              <div className="flex justify-between text-[10px] text-gray-500 mb-1.5">
                <span>CPU</span>
                <span className="text-gray-300 font-mono">{snap.cpu_pct.toFixed(1)}%</span>
              </div>
              <UsageBar pct={snap.cpu_pct} />
            </div>
            <div className="bg-gray-800/50 rounded-lg p-3">
              <div className="flex justify-between text-[10px] text-gray-500 mb-1.5">
                <span>RAM</span>
                <span className="text-gray-300 font-mono">
                  {snap.ram_used_mb.toLocaleString()} / {snap.ram_total_mb.toLocaleString()} MB
                </span>
              </div>
              <UsageBar pct={ramPct} />
            </div>
          </div>

          {/* Disks */}
          {snap.disks.length > 0 && (
            <div>
              <div className="text-[10px] text-gray-600 mb-1.5 uppercase tracking-wide">Disks</div>
              <div className="space-y-2">
                {snap.disks.map((d) => (
                  <div key={d.mount}>
                    <div className="flex justify-between text-[10px] mb-0.5">
                      <span className="text-gray-500 font-mono truncate max-w-[100px]">{d.mount}</span>
                      <span className="text-gray-400">
                        {d.used_gb.toFixed(1)}&thinsp;/&thinsp;{d.total_gb.toFixed(1)} GB
                        <span className="ml-1 text-gray-600">({d.pct.toFixed(0)}%)</span>
                      </span>
                    </div>
                    <UsageBar pct={d.pct} />
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Network */}
          {activeNets.length > 0 && (
            <div>
              <div className="text-[10px] text-gray-600 mb-1.5 uppercase tracking-wide">Network</div>
              <div className="space-y-0.5">
                {activeNets.map((n) => (
                  <div key={n.name} className="flex items-center gap-2 text-[10px]">
                    <span className="text-gray-500 font-mono truncate flex-1">{n.name}</span>
                    <span className="text-blue-400 shrink-0">↓&nbsp;{n.rx_kbps.toFixed(0)}&thinsp;KB/s</span>
                    <span className="text-green-400 shrink-0">↑&nbsp;{n.tx_kbps.toFixed(0)}&thinsp;KB/s</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Processes */}
          <div>
            <div className="flex items-center mb-1.5">
              <span className="text-[10px] text-gray-600 uppercase tracking-wide flex-1">Top Processes</span>
              <div className="flex gap-1">
                <button
                  onClick={() => setSortBy("mem")}
                  className={`text-[9px] px-1.5 py-0.5 rounded transition-colors ${sortBy === "mem" ? "bg-amber-500/20 text-amber-400" : "text-gray-600 hover:text-gray-400"}`}
                >
                  RAM
                </button>
                <button
                  onClick={() => setSortBy("cpu")}
                  className={`text-[9px] px-1.5 py-0.5 rounded transition-colors ${sortBy === "cpu" ? "bg-blue-500/20 text-blue-400" : "text-gray-600 hover:text-gray-400"}`}
                >
                  CPU
                </button>
              </div>
            </div>
            <div className="space-y-0.5">
              {[...snap.processes]
                .sort((a, b) => sortBy === "cpu" ? b.cpu_pct - a.cpu_pct : b.mem_mb - a.mem_mb)
                .slice(0, 15)
                .map((p) => (
                  <div key={p.pid} className="flex items-center gap-2 text-[10px]">
                    <span className="text-gray-400 truncate flex-1">{p.name}</span>
                    <span className="text-gray-600 font-mono shrink-0">{p.pid}</span>
                    <span className="text-amber-400 font-mono shrink-0 w-16 text-right">{p.mem_mb}&thinsp;MB</span>
                    <span className="text-blue-400 font-mono shrink-0 w-14 text-right">{p.cpu_pct.toFixed(1)}%</span>
                  </div>
                ))}
            </div>
          </div>
        </>
      )}
    </div>
  );
}