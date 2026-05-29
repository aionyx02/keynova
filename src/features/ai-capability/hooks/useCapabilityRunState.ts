import { useCallback, useEffect, useRef, useState } from "react";

export type CapabilityRunStatus =
  | "idle"
  | "pending"
  | "complete"
  | "cancelled"
  | "error";

interface Deps {
  active: boolean;
  argsKey: string | null;
  isLoading: boolean;
  error: string | null;
  autoSubmit?: boolean;
  run: () => Promise<void>;
  cancelInner: () => Promise<void> | void;
}

export interface UseCapabilityRunState {
  status: CapabilityRunStatus;
  startedAtMs: number | null;
  completedAtMs: number | null;
  cancel: () => void;
  submit: () => void;
}

function shouldReset(activeKey: string | null, submittedKey: string | null): boolean {
  return submittedKey !== null && activeKey !== submittedKey;
}

export function useCapabilityRunState(deps: Deps): UseCapabilityRunState {
  const { active, argsKey, isLoading, error, autoSubmit = false, run, cancelInner } = deps;
  const [startedAtMs, setStartedAtMs] = useState<number | null>(null);
  const [completedAtMs, setCompletedAtMs] = useState<number | null>(null);
  const [cancelled, setCancelled] = useState(false);
  const [hasRunOnce, setHasRunOnce] = useState(false);

  const prevLoadingRef = useRef(false);
  const lastSubmittedKeyRef = useRef<string | null>(null);

  const activeKey = active ? argsKey : null;

  useEffect(() => {
    if (shouldReset(activeKey, lastSubmittedKeyRef.current)) {
      void cancelInner();
      // eslint-disable-next-line react-hooks/set-state-in-effect -- bounded reset on capability key change
      setStartedAtMs(null);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCompletedAtMs(null);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCancelled(false);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setHasRunOnce(false);
      lastSubmittedKeyRef.current = null;
    }
  }, [activeKey, cancelInner]);

  useEffect(() => {
    if (
      prevLoadingRef.current &&
      !isLoading &&
      completedAtMs === null &&
      startedAtMs !== null
    ) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- bounded transition observation
      setCompletedAtMs(Date.now());
    }
    prevLoadingRef.current = isLoading;
  }, [isLoading, completedAtMs, startedAtMs]);

  const cancel = useCallback(() => {
    setCancelled(true);
    setCompletedAtMs((prev) => prev ?? Date.now());
    void cancelInner();
  }, [cancelInner]);

  const submit = useCallback(() => {
    if (activeKey === null) return;
    if (activeKey.trim() === "") return;
    void cancelInner();
    setStartedAtMs(Date.now());
    setCompletedAtMs(null);
    setCancelled(false);
    setHasRunOnce(true);
    lastSubmittedKeyRef.current = activeKey;
    void run();
  }, [activeKey, cancelInner, run]);

  useEffect(() => {
    if (!autoSubmit || activeKey === null) return;
    if (lastSubmittedKeyRef.current === activeKey) return;
    submit();
  }, [autoSubmit, activeKey, submit]);

  let status: CapabilityRunStatus;
  if (!hasRunOnce) {
    status = "idle";
  } else if (cancelled) {
    status = "cancelled";
  } else if (error) {
    status = "error";
  } else if (isLoading) {
    status = "pending";
  } else {
    status = "complete";
  }

  return {
    status,
    startedAtMs,
    completedAtMs,
    cancel,
    submit,
  };
}
