import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import { useRecentWorkflows, useSuggestedWorkflows } from "./useWorkflowMemory";

// jsdom does not set `window.__TAURI_INTERNALS__`, so the hook
// short-circuits — we don't actually issue IPC. To exercise the dispatch
// path here, we install a stub flag on `window` before each test.

async function withTauriShim(run: () => Promise<void>): Promise<void> {
  const original = (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  try {
    await run();
  } finally {
    if (original === undefined) {
      delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    } else {
      (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = original;
    }
  }
}

describe("useRecentWorkflows", () => {
  it("short-circuits to empty rows outside Tauri", async () => {
    const dispatch = vi.fn();
    const { result } = renderHook(() =>
      useRecentWorkflows({ dispatch: dispatch as DispatchFn }),
    );
    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.rows).toEqual([]);
    expect(dispatch).not.toHaveBeenCalled();
  });

  it("dispatches workflow.recent with limit when inside Tauri", async () => {
    await withTauriShim(async () => {
      const dispatch = vi.fn().mockResolvedValue({
        rows: [
          {
            id: 1,
            context_hash: "abc",
            route: "cmd.run",
            action_label: "help",
            executed_at: 1700000000,
          },
        ],
      });
      const { result } = renderHook(() =>
        useRecentWorkflows({ dispatch: dispatch as DispatchFn, limit: 3 }),
      );
      await waitFor(() => expect(result.current.isLoading).toBe(false));
      expect(dispatch).toHaveBeenCalledWith(IPC.WORKFLOW_RECENT, { limit: 3 });
      expect(result.current.rows).toHaveLength(1);
      expect(result.current.rows[0]!.action_label).toBe("help");
    });
  });

  it("surfaces dispatch errors and clears rows", async () => {
    await withTauriShim(async () => {
      const dispatch = vi.fn().mockRejectedValue(new Error("store offline"));
      const { result } = renderHook(() =>
        useRecentWorkflows({ dispatch: dispatch as DispatchFn }),
      );
      await waitFor(() => expect(result.current.isLoading).toBe(false));
      expect(result.current.error).toBe("store offline");
      expect(result.current.rows).toEqual([]);
    });
  });
});

describe("useSuggestedWorkflows", () => {
  it("dispatches workflow.suggest route", async () => {
    await withTauriShim(async () => {
      const dispatch = vi.fn().mockResolvedValue({ rows: [] });
      const { result } = renderHook(() =>
        useSuggestedWorkflows({ dispatch: dispatch as DispatchFn }),
      );
      await waitFor(() => expect(result.current.isLoading).toBe(false));
      expect(dispatch).toHaveBeenCalledWith(IPC.WORKFLOW_SUGGEST, { limit: undefined });
    });
  });

  it("refresh re-issues the IPC call", async () => {
    await withTauriShim(async () => {
      const dispatch = vi.fn().mockResolvedValue({ rows: [] });
      const { result } = renderHook(() =>
        useSuggestedWorkflows({ dispatch: dispatch as DispatchFn }),
      );
      await waitFor(() => expect(result.current.isLoading).toBe(false));
      dispatch.mockClear();
      await act(async () => {
        await result.current.refresh();
      });
      expect(dispatch).toHaveBeenCalledTimes(1);
    });
  });
});
