import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import { useCapability } from "./useCapability";

// jsdom path: `window.__TAURI_INTERNALS__` is absent, so the hook short-
// circuits listener setup. We assert the hook still dispatches `capability.call`
// with the expected shape, tracks isLoading transitions, and surfaces dispatch
// errors with a fail-safe risk tag (requires_confirmation === true per
// ADR-0030 §4).

describe("useCapability", () => {
  it("dispatches capability.call with id and payload", async () => {
    const dispatch = vi.fn().mockResolvedValue({ status: "pending", request_id: "rid-1" });
    const { result } = renderHook(() => useCapability({ dispatch: dispatch as DispatchFn, id: "explain" }));

    await act(async () => {
      await result.current.run({ text: "what is rg?" });
    });

    expect(dispatch).toHaveBeenCalledTimes(1);
    const [route, payload] = dispatch.mock.calls[0]!;
    expect(route).toBe(IPC.CAPABILITY_CALL);
    expect(payload).toMatchObject({ id: "explain", payload: { text: "what is rg?" }, stream: false });
    expect(typeof (payload as { request_id: string }).request_id).toBe("string");
  });

  it("isLoading flips true while the dispatch promise is pending", async () => {
    let resolveDispatch: ((v: unknown) => void) | null = null;
    const dispatch = vi.fn().mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveDispatch = resolve;
        }),
    );
    const { result } = renderHook(() => useCapability({ dispatch: dispatch as DispatchFn, id: "summarize" }));

    // Start the call but do not await — we want to observe the in-flight state.
    let runPromise!: Promise<void>;
    act(() => {
      runPromise = result.current.run({ text: "hello" });
    });
    await waitFor(() => expect(result.current.isLoading).toBe(true));

    await act(async () => {
      resolveDispatch?.({ status: "pending", request_id: "x" });
      await runPromise;
    });
    // Outside Tauri we never receive `capability.response`, so isLoading
    // stays true until a future response event. The hook's behavior under
    // jsdom is to remain in-flight pending event-channel completion.
    expect(result.current.isLoading).toBe(true);
  });

  it("dispatch failure sets error and fail-safe risk tag", async () => {
    const dispatch = vi.fn().mockRejectedValue(new Error("backend offline"));
    const { result } = renderHook(() => useCapability({ dispatch: dispatch as DispatchFn, id: "fix_error" }));

    await act(async () => {
      await result.current.run({ raw_output: "error: oops" });
    });

    expect(result.current.error).toBe("backend offline");
    expect(result.current.isLoading).toBe(false);
    // ADR-0030 §4 fail-safe: missing/invalid risk tag → requires_confirmation true.
    expect(result.current.risk?.requires_confirmation).toBe(true);
  });

  it("cancel dispatches capability.cancel for the active request id", async () => {
    const dispatch = vi.fn().mockResolvedValue({ status: "pending", request_id: "rid-c" });
    const { result } = renderHook(() => useCapability({ dispatch: dispatch as DispatchFn, id: "explain" }));

    await act(async () => {
      await result.current.run({ text: "x" });
    });
    dispatch.mockClear();
    await act(async () => {
      await result.current.cancel();
    });

    expect(dispatch).toHaveBeenCalledTimes(1);
    const [route, payload] = dispatch.mock.calls[0]!;
    expect(route).toBe(IPC.CAPABILITY_CANCEL);
    expect(payload).toEqual({ request_id: result.current.requestId });
  });

  it("cancel is a no-op when there is no active request", async () => {
    const dispatch = vi.fn();
    const { result } = renderHook(() => useCapability({ dispatch: dispatch as DispatchFn, id: "explain" }));
    await act(async () => {
      await result.current.cancel();
    });
    expect(dispatch).not.toHaveBeenCalled();
  });
});
