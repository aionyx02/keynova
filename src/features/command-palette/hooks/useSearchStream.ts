// Search stream state, chunk listener, and debounced trigger.
//
// Owns the four pieces of state that move together during a streaming
// `search.query` exchange:
//   - `results`            — the merged result list rendered by the palette
//   - `selected`           — UI cursor index, reset to 0 on new responses
//   - `timedOutProviders`  — provider names whose 800ms slice missed
//   - `fileDiagnostics`    — backend-supplied search diagnostics for this batch
//
// And the refs that keep streaming chunks aligned with the request that
// produced them:
//   - `searchIdRef`              — monotonic request counter
//   - `activeSearchRequestRef`   — `search-<id>` of the request currently
//                                   considered live; chunks with another id
//                                   are dropped (prevents Bug A stale-result
//                                   races when the user keeps typing).
//   - `searchLimitRef`           — `launcher.max_results`; receives updates
//                                   from outside via `setSearchLimit()` so the
//                                   chunk listener always sees the latest cap.
//
// `triggerSearch(rawInput)` is the 200ms debounced dispatcher; `cancelSearch()`
// is the immediate teardown used by ESC / workspace switch / mode switch.

import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import type {
  SearchChunkDiagnostics,
  SearchChunkPayload,
  SearchErrorPayload,
} from "../../../types/search";
import type { UnifiedResult } from "../../../types/unified-result";
import { applySourceQuotas, mergeUnifiedResults, sortUnifiedResults } from "../../../utils/search";

const DEFAULT_SEARCH_LIMIT = 30;
const SEARCH_DEBOUNCE_MS = 200;
const FIRST_BATCH_CAP = 20;

export interface UseSearchStreamDeps {
  dispatch: DispatchFn;
  setLoading: (loading: boolean) => void;
}

export interface UseSearchStream {
  results: UnifiedResult[];
  setResults: React.Dispatch<React.SetStateAction<UnifiedResult[]>>;
  selected: number;
  setSelected: React.Dispatch<React.SetStateAction<number>>;
  timedOutProviders: string[];
  setTimedOutProviders: React.Dispatch<React.SetStateAction<string[]>>;
  fileDiagnostics: SearchChunkDiagnostics | null;
  setFileDiagnostics: React.Dispatch<React.SetStateAction<SearchChunkDiagnostics | null>>;
  /** Trigger a 200ms-debounced search. Subsequent calls supersede pending ones. */
  triggerSearch: (rawInput: string) => void;
  /** Mark current request stale + ask backend to cancel + clear pending debounce. */
  cancelSearch: () => void;
  /** Update the hard cap; affects subsequent triggerSearch payloads + chunk merges. */
  setSearchLimit: (limit: number) => void;
  /** Wipe results + selected + timedOut + fileDiagnostics, and mark request stale. */
  clearResults: () => void;
}

export function useSearchStream({ dispatch, setLoading }: UseSearchStreamDeps): UseSearchStream {
  const [results, setResults] = useState<UnifiedResult[]>([]);
  const [selected, setSelected] = useState(0);
  const [timedOutProviders, setTimedOutProviders] = useState<string[]>([]);
  const [fileDiagnostics, setFileDiagnostics] = useState<SearchChunkDiagnostics | null>(null);

  const searchIdRef = useRef(0);
  const activeSearchRequestRef = useRef("");
  const searchLimitRef = useRef(DEFAULT_SEARCH_LIMIT);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Chunk + error listener. Registered once (setLoading is a stable Zustand
  // setter). Reads searchLimitRef + activeSearchRequestRef from closure so the
  // listener always sees the latest values without re-registering.
  useEffect(() => {
    if (!window.__TAURI_INTERNALS__) return;

    const chunkListener = listen<SearchChunkPayload>("search-results-chunk", (event) => {
      const payload = event.payload;
      if (payload.request_id !== activeSearchRequestRef.current) return;
      if (payload.timed_out_providers?.length) {
        setTimedOutProviders(payload.timed_out_providers);
      }
      if (payload.diagnostics) {
        setFileDiagnostics(payload.diagnostics);
      }
      // Guard the array before dereferencing: this is the one IPC-return path
      // that skips the field-by-field validation the capability parsers use, so
      // a malformed chunk (items missing/non-array) would throw inside the
      // listener callback (unhandled rejection, stuck spinner) (L8).
      if (!Array.isArray(payload.items)) {
        // ignore malformed items; still honor done/diagnostics below
      } else if (payload.replace) {
        // Final balanced batch from backend — replace results entirely.
        // Deleted paths are filtered at render time via
        // `visibleResults`, so the kill set doesn't need to live in this
        // event handler.
        setResults(applySourceQuotas(sortUnifiedResults(payload.items), searchLimitRef.current));
      } else if (payload.items.length > 0) {
        setResults((current) =>
          mergeUnifiedResults(current, payload.items, searchLimitRef.current),
        );
      }
      if (payload.done) {
        setLoading(false);
      }
    });
    const errorListener = listen<SearchErrorPayload>("search-results-error", (event) => {
      if (event.payload.request_id !== activeSearchRequestRef.current) return;
      setTimedOutProviders([]);
      setLoading(false);
    });

    return () => {
      chunkListener.then((fn) => fn());
      errorListener.then((fn) => fn());
    };
  }, [setLoading]);

  const setSearchLimit = useCallback((limit: number) => {
    if (Number.isFinite(limit) && limit > 0) {
      searchLimitRef.current = limit;
    }
  }, []);

  const cancelSearch = useCallback(() => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
      debounceRef.current = null;
    }
    activeSearchRequestRef.current = "";
    void dispatch(IPC.SEARCH_CANCEL).catch(() => {});
    // dispatch is intentionally omitted from deps — see hook contract.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const clearResults = useCallback(() => {
    setResults([]);
    setSelected(0);
    setTimedOutProviders([]);
    setFileDiagnostics(null);
    activeSearchRequestRef.current = "";
  }, []);

  const triggerSearch = useCallback(
    (rawInput: string) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(async () => {
        const reqId = ++searchIdRef.current;
        const requestId = `search-${reqId}`;
        activeSearchRequestRef.current = requestId;
        setLoading(true);
        setTimedOutProviders([]);
        setFileDiagnostics(null);
        try {
          const data = await dispatch<UnifiedResult[]>(IPC.SEARCH_QUERY, {
            query: rawInput,
            limit: searchLimitRef.current,
            stream: true,
            request_id: requestId,
            first_batch_limit: Math.min(FIRST_BATCH_CAP, searchLimitRef.current),
          });
          if (reqId !== searchIdRef.current) return;
          setResults(sortUnifiedResults(data).slice(0, searchLimitRef.current));
          setSelected(0);
          if (data.length >= searchLimitRef.current) {
            setLoading(false);
          }
        } catch {
          if (reqId === searchIdRef.current) {
            setResults([]);
            setLoading(false);
          }
        }
      }, SEARCH_DEBOUNCE_MS);
    },
    // dispatch / setLoading intentionally omitted — see hook contract.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  return {
    results,
    setResults,
    selected,
    setSelected,
    timedOutProviders,
    setTimedOutProviders,
    fileDiagnostics,
    setFileDiagnostics,
    triggerSearch,
    cancelSearch,
    setSearchLimit,
    clearResults,
  };
}
