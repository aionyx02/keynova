import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../i18n/useI18n";
import type { PanelProps } from "../types/panel";

interface VolumeInfo { level: number; muted: boolean; }
interface WifiInfo { ssid: string; signal: number; connected: boolean; }

async function ipcDispatch<T>(route: string, payload?: Record<string, unknown>): Promise<T> {
  return invoke<T>("cmd_dispatch", { route, payload: payload ?? null });
}

export function SystemPanel({ onClose }: PanelProps) {
  const t = useI18n();
  const [volume, setVolume] = useState<VolumeInfo | null>(null);
  const [brightness, setBrightness] = useState<number | null>(null);
  const [wifi, setWifi] = useState<WifiInfo[]>([]);
  const [error, setError] = useState("");

  const load = useCallback(async () => {
    if (!window.__TAURI_INTERNALS__) return;
    const results = await Promise.allSettled([
      ipcDispatch<VolumeInfo>("system.volume.get"),
      ipcDispatch<{ level: number }>("system.brightness.get"),
      ipcDispatch<WifiInfo[]>("system.wifi.info"),
    ]);
    if (results[0].status === "fulfilled") setVolume(results[0].value);
    if (results[1].status === "fulfilled") setBrightness(results[1].value.level);
    if (results[2].status === "fulfilled") setWifi(results[2].value);
    const firstErr = results.find((r) => r.status === "rejected");
    if (firstErr && firstErr.status === "rejected") setError(String(firstErr.reason));
  }, []);

  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;
    Promise.allSettled([
      ipcDispatch<VolumeInfo>("system.volume.get"),
      ipcDispatch<{ level: number }>("system.brightness.get"),
      ipcDispatch<WifiInfo[]>("system.wifi.info"),
    ]).then((results) => {
      if (results[0].status === "fulfilled") setVolume(results[0].value);
      if (results[1].status === "fulfilled") setBrightness(results[1].value.level);
      if (results[2].status === "fulfilled") setWifi(results[2].value);
      const firstErr = results.find((r) => r.status === "rejected");
      if (firstErr && firstErr.status === "rejected") setError(String(firstErr.reason));
    });
  }, []);

  async function setVolumeLevel(level: number) {
    await ipcDispatch("system.volume.set", { level: level / 100 });
    setVolume((v) => v ? { ...v, level: level / 100 } : v);
  }

  async function toggleMute() {
    if (!volume) return;
    await ipcDispatch("system.volume.mute", { muted: !volume.muted });
    setVolume((v) => v ? { ...v, muted: !v.muted } : v);
  }

  async function setBrightnessLevel(level: number) {
    await ipcDispatch("system.brightness.set", { level });
    setBrightness(level);
  }

  return (
    <div
      className="bg-gray-900/95 backdrop-blur-md rounded-b-xl shadow-2xl flex flex-col p-4 gap-4"
      onKeyDown={(e) => { if (e.key === "Escape") { e.preventDefault(); onClose(); } }}
    >
      <div className="flex items-center gap-2">
        <span className="text-xs font-semibold text-blue-400 uppercase tracking-wide">{t.system.title}</span>
        <button onClick={() => void load()} className="text-[10px] text-gray-600 hover:text-gray-400 ml-auto">⟳ 重整</button>
      </div>

      {error && <p className="text-xs text-amber-400">{t.system.notSupported}: {error}</p>}

      {/* Volume */}
      {volume !== null && (
        <div className="flex items-center gap-3">
          <span className="text-xs text-gray-400 w-10 shrink-0">{t.system.volume}</span>
          <input
            type="range"
            min={0}
            max={100}
            value={Math.round(volume.level * 100)}
            onChange={(e) => void setVolumeLevel(Number(e.target.value))}
            className="flex-1 accent-blue-500"
          />
          <span className="text-xs text-gray-300 w-8 text-right">{Math.round(volume.level * 100)}%</span>
          <button
            onClick={() => void toggleMute()}
            className={`text-xs px-2 py-0.5 rounded transition-colors ${
              volume.muted ? "bg-red-600/50 text-red-300" : "bg-gray-700/60 text-gray-400 hover:text-gray-200"
            }`}
          >
            {volume.muted ? t.system.unmute : t.system.mute}
          </button>
        </div>
      )}

      {/* Brightness */}
      {brightness !== null && (
        <div className="flex items-center gap-3">
          <span className="text-xs text-gray-400 w-10 shrink-0">{t.system.brightness}</span>
          <input
            type="range"
            min={0}
            max={100}
            value={brightness}
            onChange={(e) => void setBrightnessLevel(Number(e.target.value))}
            className="flex-1 accent-yellow-400"
          />
          <span className="text-xs text-gray-300 w-8 text-right">{brightness}%</span>
        </div>
      )}

      {/* WiFi */}
      {wifi.length > 0 && (
        <div className="flex items-center gap-3">
          <span className="text-xs text-gray-400 w-10 shrink-0">{t.system.wifi}</span>
          <div className="flex-1">
            {wifi.map((w) => (
              <div key={w.ssid} className="flex items-center gap-2">
                <span className={`text-xs font-medium ${w.connected ? "text-green-400" : "text-gray-500"}`}>
                  {w.ssid}
                </span>
                <span className="text-[10px] text-gray-600">{w.signal}%</span>
                {w.connected && <span className="text-[9px] text-green-500">● 已連線</span>}
              </div>
            ))}
          </div>
        </div>
      )}

      {volume === null && brightness === null && wifi.length === 0 && !error && (
        <p className="text-xs text-gray-600 text-center">{t.common.loading}</p>
      )}
    </div>
  );
}