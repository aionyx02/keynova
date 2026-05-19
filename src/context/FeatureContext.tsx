import { createContext, useCallback, useContext, useState } from "react";
import type { ReactNode } from "react";
import { useIPC } from "../hooks/useIPC";
import { IPC } from "../ipc/routes";

export type FeatureKey = "terminal" | "agent" | "ai" | "notes" | "nvim" | "system_monitor";

interface FeatureContextValue {
  isActive: (key: FeatureKey) => boolean;
  activate: (key: FeatureKey) => void;
}

const FeatureContext = createContext<FeatureContextValue | null>(null);

export function FeatureProvider({ children }: { children: ReactNode }) {
  const { dispatch } = useIPC();
  const [activeFeatures, setActiveFeatures] = useState<ReadonlySet<FeatureKey>>(new Set());

  const isActive = useCallback(
    (key: FeatureKey) => activeFeatures.has(key),
    [activeFeatures],
  );

  const activate = useCallback(
    (key: FeatureKey) => {
      setActiveFeatures((prev) => {
        if (prev.has(key)) return prev;
        // Notify backend once per key — idempotent at the service level.
        dispatch(IPC.FEATURE_ACTIVATE, { key }).catch(() => {});
        const next = new Set(prev);
        next.add(key);
        return next;
      });
    },
    [dispatch],
  );

  return (
    <FeatureContext.Provider value={{ isActive, activate }}>
      {children}
    </FeatureContext.Provider>
  );
}

export function useFeature(): FeatureContextValue {
  const ctx = useContext(FeatureContext);
  if (!ctx) throw new Error("[useFeature] must be used inside <FeatureProvider>");
  return ctx;
}