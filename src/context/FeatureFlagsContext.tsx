// Persisted `features.*` flags as an app-wide single source of truth.
//
// Distinct from FeatureContext (which tracks in-memory session lazy-activation):
// this provider reads the persisted `features.*` config so the UI can hide a
// feature's surfaces when its flag is off. Mirrors the `useLauncherSettings`
// pattern: one `setting.list_all` read on mount + refresh on `config-reloaded`.

import { createContext, useCallback, useContext, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";

import { useIPC } from "../hooks/useIPC";
import { IPC } from "../ipc/routes";
import type { SettingEntry } from "../ipc/types";

export type GateKey = "ai" | "translation" | "notes" | "history" | "calculator" | "system";

// Defaults match `default_config.toml`: AI surfaces are opt-in, the rest opt-out.
// Seeding them avoids a flash of feature UI before the first fetch resolves.
const DEFAULT_FLAGS: Record<GateKey, boolean> = {
  ai: false,
  translation: true,
  notes: true,
  history: true,
  calculator: true,
  system: true,
};

const GATE_KEYS = Object.keys(DEFAULT_FLAGS) as GateKey[];
const configKey = (key: GateKey) => `features.${key}`;
const WATCHED_KEYS: ReadonlySet<string> = new Set(GATE_KEYS.map(configKey));

interface ConfigReloadedPayload {
  changed_keys: string[];
}

interface FeatureFlagsValue {
  isEnabled: (key: GateKey) => boolean;
}

const FeatureFlagsContext = createContext<FeatureFlagsValue | null>(null);

export function FeatureFlagsProvider({ children }: { children: ReactNode }) {
  const { dispatch } = useIPC();
  const [flags, setFlags] = useState<Record<GateKey, boolean>>(DEFAULT_FLAGS);

  useEffect(() => {
    // Browser preview: no backend to read, the seeded defaults stand.
    if (!window.__TAURI_INTERNALS__) return;

    async function refresh() {
      try {
        const entries = await dispatch<SettingEntry[]>(IPC.SETTING_LIST_ALL);
        setFlags((prev) => {
          const next = { ...prev };
          for (const key of GATE_KEYS) {
            const value = entries.find((e) => e.key === configKey(key))?.value;
            // Missing/empty ⇒ keep the default (matches the backend
            // `unwrap_or(true)` idiom for opt-out features).
            if (value !== undefined) next[key] = value !== "false";
          }
          return next;
        });
      } catch {
        // keep current values
      }
    }

    void refresh();
    const unlisten = listen<ConfigReloadedPayload>("config-reloaded", (event) => {
      const keys = event.payload.changed_keys;
      if (keys.length === 0 || keys.some((k) => WATCHED_KEYS.has(k))) {
        void refresh();
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
    // dispatch intentionally omitted — useIPC returns a fresh wrapper each render
    // (same rationale as the other Tauri-listener hooks).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const isEnabled = useCallback((key: GateKey) => flags[key], [flags]);

  return (
    <FeatureFlagsContext.Provider value={{ isEnabled }}>{children}</FeatureFlagsContext.Provider>
  );
}

export function useFeatureFlags(): FeatureFlagsValue {
  const ctx = useContext(FeatureFlagsContext);
  if (!ctx) throw new Error("[useFeatureFlags] must be used inside <FeatureFlagsProvider>");
  return ctx;
}
