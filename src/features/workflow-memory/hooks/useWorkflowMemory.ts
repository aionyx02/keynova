// `useRecentWorkflows` / `useSuggestedWorkflows` thin wrappers around the
// `workflow.*` IPC namespace.
//
// Outside Tauri (`window.__TAURI_INTERNALS__` absent) the hook short-
// circuits to an empty state, matching the jsdom pattern in
// `useSearchBackend` and `useCapability`.

import { useCallback, useEffect, useState } from "react";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import type { WorkflowHistoryRow, WorkflowSuggestResponse } from "../types";

export interface UseWorkflowMemoryDeps {
  dispatch: DispatchFn;
  /** Default 5; clamped by the backend to [1, 50]. */
  limit?: number;
}

export interface UseWorkflowMemory {
  rows: WorkflowHistoryRow[];
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

function makeWorkflowQueryHook(route: typeof IPC.WORKFLOW_RECENT | typeof IPC.WORKFLOW_SUGGEST) {
  return function useWorkflowQueryImpl({
    dispatch,
    limit,
  }: UseWorkflowMemoryDeps): UseWorkflowMemory {
    const [rows, setRows] = useState<WorkflowHistoryRow[]>([]);
    const [isLoading, setIsLoading] = useState<boolean>(false);
    const [error, setError] = useState<string | null>(null);

    const refresh = useCallback(async () => {
      if (typeof window === "undefined" || !window.__TAURI_INTERNALS__) {
        setRows([]);
        return;
      }
      setIsLoading(true);
      setError(null);
      try {
        const resp = await dispatch<WorkflowSuggestResponse>(route, {
          limit,
        });
        setRows(resp?.rows ?? []);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        setRows([]);
      } finally {
        setIsLoading(false);
      }
    }, [dispatch, limit]);

    useEffect(() => {
      void refresh();
      // dispatch comes back as a fresh wrapper per render — same caveat
      // as useSearchBackend.
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [limit]);

    return { rows, isLoading, error, refresh };
  };
}

export const useRecentWorkflows = makeWorkflowQueryHook(IPC.WORKFLOW_RECENT);
export const useSuggestedWorkflows = makeWorkflowQueryHook(IPC.WORKFLOW_SUGGEST);
