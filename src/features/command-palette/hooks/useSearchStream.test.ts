import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import type { SearchResult } from "../../../types/search";
import { useSearchStream } from "./useSearchStream";

function makeResult(name: string, score = 100): SearchResult {
  return {
    kind: "file",
    name,
    path: `/tmp/${name}`,
    score,
  };
}

interface DispatchCall {
  route: string;
  payload: unknown;
}

function recordingDispatch(impl?: (route: string, payload: unknown) => unknown): {
  dispatch: DispatchFn;
  calls: DispatchCall[];
} {
  const calls: DispatchCall[] = [];
  const dispatch = (async (route: string, payload?: unknown) => {
    calls.push({ route, payload });
    return impl ? impl(route, payload) : ([] as SearchResult[]);
  }) as unknown as DispatchFn;
  return { dispatch, calls };
}

describe("useSearchStream", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("initial state is empty", () => {
    const { dispatch } = recordingDispatch();
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    expect(result.current.results).toEqual([]);
    expect(result.current.selected).toBe(0);
    expect(result.current.timedOutProviders).toEqual([]);
    expect(result.current.fileDiagnostics).toBeNull();
  });

  it("triggerSearch debounces by 200ms before dispatching", async () => {
    const { dispatch, calls } = recordingDispatch();
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    act(() => result.current.triggerSearch("foo"));
    expect(calls).toHaveLength(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(199);
    });
    expect(calls).toHaveLength(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    expect(calls).toHaveLength(1);
    expect(calls[0].route).toBe(IPC.SEARCH_QUERY);
  });

  it("triggerSearch payload carries the latest searchLimit and stream flag", async () => {
    const { dispatch, calls } = recordingDispatch();
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    act(() => result.current.setSearchLimit(50));
    act(() => result.current.triggerSearch("query text"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(calls).toHaveLength(1);
    expect(calls[0].payload).toEqual({
      query: "query text",
      limit: 50,
      stream: true,
      request_id: "search-1",
      first_batch_limit: 20,
    });
  });

  it("setSearchLimit clamps non-positive values (no-op for 0)", async () => {
    const { dispatch, calls } = recordingDispatch();
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    act(() => result.current.setSearchLimit(0));
    act(() => result.current.triggerSearch("foo"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(calls[0].payload).toMatchObject({ limit: 30 }); // default preserved
  });

  it("a second triggerSearch within debounce supersedes the first", async () => {
    const { dispatch, calls } = recordingDispatch();
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    act(() => result.current.triggerSearch("first"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });
    act(() => result.current.triggerSearch("second"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    // Only the second triggers a dispatch.
    expect(calls).toHaveLength(1);
    expect((calls[0].payload as { query: string }).query).toBe("second");
  });

  it("cancelSearch clears pending debounce and dispatches SEARCH_CANCEL", async () => {
    const { dispatch, calls } = recordingDispatch();
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    act(() => result.current.triggerSearch("foo"));
    act(() => result.current.cancelSearch());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });
    // The pending triggerSearch was cleared — no SEARCH_QUERY dispatched.
    const search = calls.find((c) => c.route === IPC.SEARCH_QUERY);
    expect(search).toBeUndefined();
    const cancel = calls.find((c) => c.route === IPC.SEARCH_CANCEL);
    expect(cancel).toBeDefined();
  });

  it("clearResults resets results / selected / timedOut / diagnostics", () => {
    const { dispatch } = recordingDispatch();
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    act(() => {
      result.current.setResults([makeResult("a"), makeResult("b")]);
      result.current.setSelected(1);
      result.current.setTimedOutProviders(["tantivy"]);
    });
    expect(result.current.results.length).toBe(2);
    act(() => result.current.clearResults());
    expect(result.current.results).toEqual([]);
    expect(result.current.selected).toBe(0);
    expect(result.current.timedOutProviders).toEqual([]);
    expect(result.current.fileDiagnostics).toBeNull();
  });

  it("dispatch success writes results sorted and trimmed to limit", async () => {
    const fakeData: SearchResult[] = [
      makeResult("low", 10),
      makeResult("high", 90),
      makeResult("mid", 50),
    ];
    const { dispatch } = recordingDispatch(() => fakeData);
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    act(() => result.current.setSearchLimit(2));
    act(() => result.current.triggerSearch("anything"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(result.current.results.length).toBe(2);
    // Highest scores first after sortSearchResults.
    expect(result.current.results[0].score).toBeGreaterThanOrEqual(
      result.current.results[1].score,
    );
  });

  it("dispatch rejection clears results when reqId still current", async () => {
    const { dispatch } = recordingDispatch(() => {
      throw new Error("boom");
    });
    const setLoading = vi.fn();
    const { result } = renderHook(() => useSearchStream({ dispatch, setLoading }));
    act(() => {
      result.current.setResults([makeResult("ghost")]);
    });
    expect(result.current.results.length).toBe(1);
    act(() => result.current.triggerSearch("anything"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    expect(result.current.results).toEqual([]);
    expect(setLoading).toHaveBeenCalledWith(false);
  });
});
