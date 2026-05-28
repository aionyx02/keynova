// REF.6.B — Submit-on-Enter wrapper around `useCapability` for prefix mode.
//
// Replaces the original debounced auto-fire design. Reasons:
//   - Capability calls cost real backend work (Ollama load, token gen);
//     auto-firing on every typing pause wastes them.
//   - Enter is the keyboard-first conventional "commit" gesture.
//   - Predictable status flow: idle → pending → streaming → complete.
//
// State machine:
//   args === null      → idle, no-op submit
//   args !== null      → idle, awaits caller to invoke `submit()`
//   submit() called    → dispatch run + flip to pending; previous in-flight
//                        request is cancelled before the new one fires
//   args change after a submit → reset everything to idle (the previous
//                        answer is for a different question; require a
//                        fresh Enter)
//
// Cancel on unmount stays the same.

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
  /** Null = no prefix active. Setting a value enables `submit()`. */
  args: { text: string } | null;
}

export interface UseCapabilityStream {
  status: CapabilityStreamStatus;
  text: string;
  error: string | null;
  startedAtMs: number | null;
  firstChunkAtMs: number | null;
  completedAtMs: number | null;
  /** Cancel any in-flight stream + flip status to cancelled. */
  cancel: () => void;
  /** Dispatch a capability run for the current args. No-op when args is null. */
  submit: () => void;
}

export function useCapabilityStream({
  dispatch,
  id,
  args,
}: UseCapabilityStreamDeps): UseCapabilityStream {
  const inner = useCapability({ dispatch, id });
  const { run, cancel: innerCancel, data, streamText, isLoading, error } = inner;

  const [startedAtMs, setStartedAtMs] = useState<number | null>(null);
  const [firstChunkAtMs, setFirstChunkAtMs] = useState<number | null>(null);
  const [completedAtMs, setCompletedAtMs] = useState<number | null>(null);
  const [cancelled, setCancelled] = useState(false);
  const [hasRunOnce, setHasRunOnce] = useState(false);

  const argsKey = args ? args.text : null;
  /** Set when a `submit()` dispatches a run; cleared on args change. */
  const lastSubmittedKeyRef = useRef<string | null>(null);

  // Args change after a submit → discard the stale response and return to
  // idle. The previous answer was for a different question, and the user
  // explicitly types out a new one + presses Enter to ask again.
  useEffect(() => {
    if (
      lastSubmittedKeyRef.current !== null &&
      argsKey !== lastSubmittedKeyRef.current
    ) {
      void innerCancel();
      // eslint-disable-next-line react-hooks/set-state-in-effect -- bounded one-shot reset on args-change after submit
      setStartedAtMs(null);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setFirstChunkAtMs(null);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCompletedAtMs(null);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setCancelled(false);
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setHasRunOnce(false);
      lastSubmittedKeyRef.current = null;
    }
  }, [argsKey, innerCancel]);

  const resolvedText =
    streamText || (data?.kind === "text" ? data.text : "");

  // First-chunk timing: resolvedText goes from empty to non-empty after a run.
  // The final response text is a fallback for providers or event channels that
  // complete without incremental chunks.
  useEffect(() => {
    if (
      resolvedText &&
      firstChunkAtMs === null &&
      startedAtMs !== null &&
      !cancelled
    ) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- bounded one-shot transition observation
      setFirstChunkAtMs(Date.now());
    }
  }, [resolvedText, firstChunkAtMs, startedAtMs, cancelled]);

  // Completion timing: isLoading drops after a run was dispatched.
  const prevLoadingRef = useRef(false);
  useEffect(() => {
    if (
      prevLoadingRef.current &&
      !isLoading &&
      completedAtMs === null &&
      startedAtMs !== null
    ) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
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

  const submit = useCallback(() => {
    if (argsKey === null || argsKey.trim() === "") return;
    void innerCancel();
    setStartedAtMs(Date.now());
    setFirstChunkAtMs(null);
    setCompletedAtMs(null);
    setCancelled(false);
    setHasRunOnce(true);
    lastSubmittedKeyRef.current = argsKey;
    void run({ text: argsKey }, { stream: true });
  }, [argsKey, run, innerCancel]);

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
    text: resolvedText,
    error,
    startedAtMs,
    firstChunkAtMs,
    completedAtMs,
    cancel,
    submit,
  };
}
