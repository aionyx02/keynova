import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import { useCapabilityStream } from "./useCapabilityStream";

// jsdom path: capability.response events never fire, so the hook stays in
// `pending` until cancelled or unmounted. We assert: debounce, cancel-on-
// rerun, cancel() callback, unmount cancel, status transitions reachable
// purely from `args` and `cancel`.

describe("useCapabilityStream", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  function makeDispatch() {
    return vi.fn().mockResolvedValue({ status: "pending", request_id: "rid-1" });
  }

  it("starts idle when args is null", () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: null,
      }),
    );
    expect(result.current.status).toBe("idle");
    expect(dispatch).not.toHaveBeenCalled();
  });

  it("dispatches after debounce window when args are set", async () => {
    const dispatch = makeDispatch();
    renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "rust hashmap" },
        debounceMs: 100,
      }),
    );
    expect(dispatch).not.toHaveBeenCalled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(120);
    });
    expect(dispatch).toHaveBeenCalledTimes(1);
    const [route, payload] = dispatch.mock.calls[0]!;
    expect(route).toBe(IPC.CAPABILITY_CALL);
    expect(payload).toMatchObject({
      id: "explain",
      payload: { text: "rust hashmap" },
      stream: true,
    });
  });

  it("debounces rapid args changes to a single dispatch", async () => {
    const dispatch = makeDispatch();
    const { rerender } = renderHook(
      ({ text }) =>
        useCapabilityStream({
          dispatch: dispatch as DispatchFn,
          id: "explain",
          args: { text },
          debounceMs: 100,
        }),
      { initialProps: { text: "a" } },
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(40);
    });
    rerender({ text: "ab" });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(40);
    });
    rerender({ text: "abc" });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(40);
    });
    expect(dispatch).not.toHaveBeenCalled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(120);
    });
    expect(dispatch).toHaveBeenCalledTimes(1);
    const [, payload] = dispatch.mock.calls[0]!;
    expect((payload as { payload: { text: string } }).payload.text).toBe("abc");
  });

  it("cancels the previous in-flight request before dispatching the next", async () => {
    const dispatch = makeDispatch();
    const { rerender } = renderHook(
      ({ text }) =>
        useCapabilityStream({
          dispatch: dispatch as DispatchFn,
          id: "explain",
          args: { text },
          debounceMs: 50,
        }),
      { initialProps: { text: "first" } },
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60);
    });
    // Wait for the dispatch promise to resolve so `activeIdRef` is set.
    expect(dispatch).toHaveBeenCalledTimes(1);
    expect(dispatch.mock.calls[0]![0]).toBe(IPC.CAPABILITY_CALL);

    rerender({ text: "second" });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60);
    });
    expect(dispatch.mock.calls.length).toBeGreaterThanOrEqual(3);

    const routes = dispatch.mock.calls.map((c) => c[0]);
    // Expect at least one cancel between the two calls.
    expect(routes).toContain(IPC.CAPABILITY_CANCEL);
    expect(routes.filter((r) => r === IPC.CAPABILITY_CALL).length).toBe(2);
  });

  it("cancel() sets status to cancelled and dispatches capability.cancel", async () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "rust" },
        debounceMs: 50,
      }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60);
    });
    expect(dispatch).toHaveBeenCalledTimes(1);
    dispatch.mockClear();

    act(() => {
      result.current.cancel();
    });

    expect(result.current.status).toBe("cancelled");
    // Drain the cancel dispatch promise.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(dispatch).toHaveBeenCalledWith(IPC.CAPABILITY_CANCEL, expect.objectContaining({}));
  });

  it("populates completedAtMs on cancel", async () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "x" },
        debounceMs: 10,
      }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20);
    });
    expect(result.current.startedAtMs).not.toBeNull();
    expect(result.current.completedAtMs).toBeNull();
    act(() => {
      result.current.cancel();
    });
    expect(result.current.completedAtMs).not.toBeNull();
  });

  it("cancels in-flight request on unmount", async () => {
    const dispatch = makeDispatch();
    const { unmount } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "rust" },
        debounceMs: 10,
      }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20);
    });
    expect(dispatch).toHaveBeenCalledTimes(1);
    dispatch.mockClear();

    unmount();
    // Drain microtasks: inner.cancel is async.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(dispatch).toHaveBeenCalledWith(IPC.CAPABILITY_CANCEL, expect.objectContaining({}));
  });

  it("status is pending after run before any stream chunk arrives", async () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "rust" },
        debounceMs: 10,
      }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20);
    });
    expect(dispatch).toHaveBeenCalledTimes(1);
    expect(result.current.status).toBe("pending");
    expect(result.current.text).toBe("");
  });
});
