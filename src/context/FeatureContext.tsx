import { createContext, useCallback, useContext, useState } from "react";
import type { ReactNode } from "react";

export type FeatureKey = "terminal" | "agent" | "ai" | "notes" | "nvim" | "system_monitor";

interface FeatureContextValue {
  isActive: (key: FeatureKey) => boolean;
  activate: (key: FeatureKey) => void;
}

const FeatureContext = createContext<FeatureContextValue | null>(null);

export function FeatureProvider({ children }: { children: ReactNode }) {
  const [activeFeatures, setActiveFeatures] = useState<ReadonlySet<FeatureKey>>(new Set());

  const isActive = useCallback(
    (key: FeatureKey) => activeFeatures.has(key),
    [activeFeatures],
  );

  const activate = useCallback((key: FeatureKey) => {
    setActiveFeatures((prev) => {
      if (prev.has(key)) return prev;
      const next = new Set(prev);
      next.add(key);
      return next;
    });
  }, []);

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