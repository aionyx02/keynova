import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../i18n/useI18n";
import type { PanelProps } from "../types/panel";

interface VolumeInfo {
  level: number;
  muted: boolean;
}

interface WifiInfo {
  ssid: string;
  signal: number;
  connected: boolean;
}

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function SystemPanel({ onClose }: PanelProps) {
  const t = useI18n();
  const [volume, setVolume] = useState<VolumeInfo | null>(null);
  const [brightness, setBrightness] = useState<number | null>(null);
  const [wifi, setWifi] = useState<WifiInfo[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const rootRef = useRef<HTMLDivElement>(null);

  const load = useCallback(async () => {
    if (!window.__TAURI_INTERNALS__) return;
    setLoading(true);
    setError("");
    const results = await Promise.allSettled([
      ipcDispatch<VolumeInfo>("system.volume.get"),
      ipcDispatch<{ level: number }>("system.brightness.get"),
      ipcDispatch<WifiInfo[]>("system.wifi.info"),
    ]);
    if (results[0].status === "fulfilled") setVolume(results[0].value);
    if (results[1].status === "fulfilled") setBrightness(results[1].value.level);
    if (results[2].status === "fulfilled") setWifi(results[2].value);
    const firstErr = results.find((result) => result.status === "rejected");
    if (firstErr && firstErr.status === "rejected") setError(String(firstErr.reason));
    setLoading(false);
  }, []);

  useEffect(() => {
    rootRef.current?.focus();
    void load();
  }, [load]);

  async function setVolumeLevel(level: number) {
    await ipcDispatch("system.volume.set", { level: level / 100 });
    setVolume((current) => (current ? { ...current, level: level / 100 } : current));
  }

  async function toggleMute() {
    if (!volume) return;
    await ipcDispatch("system.volume.mute", { muted: !volume.muted });
    setVolume((current) => (current ? { ...current, muted: !current.muted } : current));
  }

  async function setBrightnessLevel(level: number) {
    await ipcDispatch("system.brightness.set", { level });
    setBrightness(level);
  }

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
      className="kn-panel-shell flex min-h-[320px] flex-col rounded-t-none border-t-0 outline-none"
    >
      <div className="kn-panel-header">
        <div>
          <div className="kn-panel-title">{t.system.title}</div>
          <div className="kn-panel-subtitle">Quick access to system controls and wireless state</div>
        </div>
        <button type="button" onClick={() => void load()} className="kn-button py-1 text-[10px]">
          Refresh
        </button>
      </div>

      <div className="flex flex-1 flex-col gap-3 px-4 py-3">
        {error && (
          <div className="kn-muted-surface border-amber-400/20 bg-amber-400/10 px-3 py-2 text-xs text-amber-200">
            {t.system.notSupported}: {error}
          </div>
        )}

        {volume !== null && (
          <div className="kn-muted-surface px-3 py-3">
            <div className="mb-2 flex items-center justify-between">
              <span className="kn-section-label">{t.system.volume}</span>
              <button type="button" onClick={() => void toggleMute()} className="kn-button px-2 py-1 text-[10px]">
                {volume.muted ? t.system.unmute : t.system.mute}
              </button>
            </div>
            <div className="flex items-center gap-3">
              <input
                type="range"
                min={0}
                max={100}
                value={Math.round(volume.level * 100)}
                onChange={(e) => void setVolumeLevel(Number(e.target.value))}
                className="flex-1 accent-[color:var(--kn-accent)]"
              />
              <span className="w-10 text-right text-xs text-[color:var(--kn-text-soft)]">
                {Math.round(volume.level * 100)}%
              </span>
            </div>
          </div>
        )}

        {brightness !== null && (
          <div className="kn-muted-surface px-3 py-3">
            <div className="mb-2 flex items-center justify-between">
              <span className="kn-section-label">{t.system.brightness}</span>
              <span className="text-xs text-[color:var(--kn-text-soft)]">{brightness}%</span>
            </div>
            <input
              type="range"
              min={0}
              max={100}
              value={brightness}
              onChange={(e) => void setBrightnessLevel(Number(e.target.value))}
              className="w-full accent-[color:var(--kn-warm)]"
            />
          </div>
        )}

        {wifi.length > 0 && (
          <div className="kn-muted-surface px-3 py-3">
            <div className="kn-section-label mb-2">{t.system.wifi}</div>
            <div className="space-y-2">
              {wifi.map((network) => (
                <div key={network.ssid} className="flex items-center justify-between gap-3">
                  <div className="min-w-0">
                    <div
                      className={`truncate text-sm ${
                        network.connected ? "text-[color:var(--kn-text)]" : "text-[color:var(--kn-text-soft)]"
                      }`}
                    >
                      {network.ssid}
                    </div>
                    <div className="text-xs text-[color:var(--kn-text-faint)]">
                      {network.connected ? "Connected" : "Nearby network"}
                    </div>
                  </div>
                  <div className="shrink-0 text-xs text-[color:var(--kn-text-muted)]">
                    {network.signal}%
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {volume === null && brightness === null && wifi.length === 0 && !error && loading && (
          <div className="flex flex-1 items-center justify-center text-xs text-[color:var(--kn-text-faint)]">
            {t.common.loading}
          </div>
        )}
      </div>

      <div className="kn-panel-footer">
        <span>Esc closes</span>
        <span>{loading ? "Refreshing status..." : "Controls update the current machine immediately"}</span>
      </div>
    </div>
  );
}
