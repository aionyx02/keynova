// Generic single-call hook for the stateless AI capability layer.
//
// Per ADR-0029 §4 the in-flight disable is state-based (not time-based
// debounce). `isLoading` reflects whether a call is in flight for *this*
// hook instance; the consuming UI is responsible for disabling its chip
// while `isLoading === true`.
//
// In jsdom without `window.__TAURI_INTERNALS__`, the hook short-circuits
// listener registration and lets the unit test verify state transitions
// without a live Tauri channel.

import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import { parseRiskTagFailSafe, type RiskTag } from "../../../types/unified-result";
import { parseCapabilitySources } from "../types";
import type {
  CapabilityId,
  CapabilityOutput,
  CapabilityResponseEvent,
  CapabilitySource,
  CapabilityStreamChunkEvent,
} from "../types";

const CAPABILITY_RESPONSE_EVENT = "capability-response";
const CAPABILITY_STREAM_CHUNK_EVENT = "capability-stream-chunk";

export interface UseCapabilityDeps {
  dispatch: DispatchFn;
  id: CapabilityId;
}

export interface UseCapabilityRunOptions {
  stream?: boolean;
  context_hash?: string;
  request_id?: string;
}

export interface UseCapability {
  data: CapabilityOutput | null;
  /** Streaming-only accumulated text. Empty unless stream=true was requested. */
  streamText: string;
  risk: RiskTag | null;
  sources: CapabilitySource[];
  error: string | null;
  isLoading: boolean;
  /** Last dispatched request_id (or null if none has been started). */
  requestId: string | null;
  run: (payload: unknown, opts?: UseCapabilityRunOptions) => Promise<void>;
  cancel: () => Promise<void>;
}

function genRequestId(id: CapabilityId): string {
  // Browser/Tauri WebView both expose crypto.randomUUID; jsdom does too.
  const uuid =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;
  return `cap-${id}-${uuid}`;
}

export function useCapability({ dispatch, id }: UseCapabilityDeps): UseCapability {
  const [data, setData] = useState<CapabilityOutput | null>(null);
  const [streamText, setStreamText] = useState<string>("");
  const [risk, setRisk] = useState<RiskTag | null>(null);
  const [sources, setSources] = useState<CapabilitySource[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [requestId, setRequestId] = useState<string | null>(null);

  const unlistensRef = useRef<UnlistenFn[]>([]);
  const activeIdRef = useRef<string | null>(null);

  const teardown = useCallback(() => {
    for (const fn of unlistensRef.current) {
      try {
        fn();
      } catch {
        // Ignore — Tauri unlisten can throw if already torn down.
      }
    }
    unlistensRef.current = [];
  }, []);

  useEffect(() => {
    return () => teardown();
  }, [teardown]);

  const cancel = useCallback(async () => {
    const active = activeIdRef.current;
    if (!active) return;
    try {
      await dispatch(IPC.CAPABILITY_CANCEL, { request_id: active });
    } catch {
      // Backend may have already torn down; we still flip local state below.
    }
  }, [dispatch]);

  const run = useCallback(
    async (payload: unknown, opts: UseCapabilityRunOptions = {}) => {
      teardown();

      setData(null);
      setStreamText("");
      setRisk(null);
      setSources([]);
      setError(null);
      setIsLoading(true);

      const rid = opts.request_id ?? genRequestId(id);
      setRequestId(rid);
      activeIdRef.current = rid;

      // Outside a Tauri WebView (e.g. jsdom unit tests) we cannot subscribe
      // to events. Tests assert state transitions via `run` directly without
      // a live channel.
      if (typeof window !== "undefined" && window.__TAURI_INTERNALS__) {
        const responseUnlistenP = listen<CapabilityResponseEvent>(
          CAPABILITY_RESPONSE_EVENT,
          (event) => {
            if (event.payload.request_id !== rid) return;
            if (event.payload.ok) {
              if (event.payload.output) {
                setData(event.payload.output);
              }
              setRisk(parseRiskTagFailSafe(event.payload.risk_tag));
              setSources(parseCapabilitySources(event.payload.sources));
              setError(null);
            } else {
              setError(event.payload.error ?? "unknown capability error");
              setRisk(parseRiskTagFailSafe(undefined)); // fail-safe to confirm
              setSources([]);
            }
            setIsLoading(false);
            activeIdRef.current = null;
            teardown();
          },
        );

        const chunkUnlistenP = opts.stream
          ? listen<CapabilityStreamChunkEvent>(CAPABILITY_STREAM_CHUNK_EVENT, (event) => {
              if (event.payload.request_id !== rid) return;
              setStreamText((prev) => prev + event.payload.delta);
            })
          : null;

        const [responseUnlisten, chunkUnlisten] = await Promise.all([
          responseUnlistenP,
          chunkUnlistenP ?? Promise.resolve<UnlistenFn>(() => {}),
        ]);
        // A newer run() may have started (and run teardown() on an empty ref)
        // while we awaited subscription. If so, these two subscriptions are
        // orphaned — unlisten them now instead of overwriting the ref and
        // leaking them until unmount (M6).
        if (activeIdRef.current !== rid) {
          responseUnlisten();
          chunkUnlisten();
        } else {
          unlistensRef.current = [responseUnlisten, chunkUnlisten];
        }
      }

      try {
        await dispatch(IPC.CAPABILITY_CALL, {
          request_id: rid,
          id,
          payload,
          context_hash: opts.context_hash,
          stream: opts.stream ?? false,
        });
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        setRisk(parseRiskTagFailSafe(undefined));
        setIsLoading(false);
        activeIdRef.current = null;
        teardown();
      }
    },
    [dispatch, id, teardown],
  );

  return { data, streamText, risk, sources, error, isLoading, requestId, run, cancel };
}
