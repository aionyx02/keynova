// REF.6.B — Streaming wrapper around `useCapability` for prefix-mode UI.
//
// Adds three things on top of the bare capability hook:
//   1. Debounced auto-run on `args` change (300 ms). Prevents per-keystroke
//      backend spam when the user is mid-typing a prefix body.
//   2. Cancel-on-rerun. Each new run cancels the previous request_id before
//      dispatching the next, so we never have two in-flight requests for
//      the same hook instance.
//   3. Projected timing + status. Surfaces `status`, `startedAtMs`,
//      `firstChunkAtMs`, `completedAtMs` for the answer card header.
//
// The hook owns its lifecycle: cancel on unmount, cancel on `args` becoming
// null (caller exited capability mode but kept the hook mounted).

import { useCallback, useEffect, useRef, useState } from "react";

import type { DispatchFn } from "../../../context/IPCContext";
import { useCapability } from "./useCapability";
import type { CapabilityId } from "../types";

export type CapabilityStreamStatus =
  | "idle"
  | "pending"
  | "streaming"
  | "complete"
  | "cancelled"
  | "error";

export interface UseCapabilityStreamDeps {
  dispatch: DispatchFn;
  id: CapabilityId;
  /** Null = idle (no run). Setting non-null fires a debounced run. */
  args: { text: string } | null;
  /** Override debounce window in tests; defaults to 300 ms. */
  debounceMs?: number;
}

export interface UseCapabilityStream {
  status: CapabilityStreamStatus;
  text: string;
  error: string | null;
  startedAtMs: number | null;
  firstChunkAtMs: number | null;
  completedAtMs: number | null;
  cancel: () => void;
}

export function useCapabilityStream({
  dispatch,
  id,
  args,
  debounceMs = 300,
}: UseCapabilityStreamDeps): UseCapabilityStream {
  const inner = useCapability({ dispatch, id });
  const { run, cancel: innerCancel, streamText, isLoading, error } = inner;

  const [startedAtMs, setStartedAtMs] = useState<number | null>(null);
  const [firstChunkAtMs, setFirstChunkAtMs] = useState<number | null>(null);
  const [completedAtMs, setCompletedAtMs] = useState<number | null>(null);
  const [cancelled, setCancelled] = useState(false);
  const [hasRunOnce, setHasRunOnce] = useState(false);

  const argsKey = args ? args.text : null;

  // Debounced auto-run on args change.
  useEffect(() => {
    if (argsKey === null) return;
    const timer = window.setTimeout(() => {
      // Cancel any prior in-flight request before dispatching a new one.
      void innerCancel();
      setStartedAtMs(Date.now());
      setFirstChunkAtMs(null);
      setCompletedAtMs(null);
      setCancelled(false);
      setHasRunOnce(true);
      void run({ text: argsKey }, { stream: true });
    }, debounceMs);
    return () => window.clearTimeout(timer);
  }, [argsKey, run, innerCancel, debounceMs]);

  // First-chunk timing: streamText goes from empty to non-empty after a run.
  // The setState-in-effect here is a one-shot observation (guarded by
  // `firstChunkAtMs === null`), not a render loop; it can't cascade because
  // the next render sees firstChunkAtMs !== null and the effect early-returns.
  useEffect(() => {
    if (
      streamText &&
      firstChunkAtMs === null &&
      startedAtMs !== null &&
      !cancelled
    ) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- bounded one-shot transition observation; reference: useCapability subscribes to Tauri events outside React's tree
      setFirstChunkAtMs(Date.now());
    }
  }, [streamText, firstChunkAtMs, startedAtMs, cancelled]);

  // Completion timing: isLoading drops after a run was dispatched. Same
  // bounded-observation pattern as above.
  const prevLoadingRef = useRef(false);
  useEffect(() => {
    if (
      prevLoadingRef.current &&
      !isLoading &&
      completedAtMs === null &&
      startedAtMs !== null
    ) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- bounded one-shot transition observation
      setCompletedAtMs(Date.now());
    }
    prevLoadingRef.current = isLoading;
  }, [isLoading, completedAtMs, startedAtMs]);

  // Cancel on unmount.
  useEffect(() => {
    return () => {
      void innerCancel();
    };
  }, [innerCancel]);

  const cancel = useCallback(() => {
    setCancelled(true);
    setCompletedAtMs((prev) => prev ?? Date.now());
    void innerCancel();
  }, [innerCancel]);

  // Derived status.
  let status: CapabilityStreamStatus;
  if (!hasRunOnce) {
    status = "idle";
  } else if (cancelled) {
    status = "cancelled";
  } else if (error) {
    status = "error";
  } else if (isLoading) {
    status = streamText ? "streaming" : "pending";
  } else {
    status = "complete";
  }

  return {
    status,
    text: streamText,
    error,
    startedAtMs,
    firstChunkAtMs,
    completedAtMs,
    cancel,
  };
}
