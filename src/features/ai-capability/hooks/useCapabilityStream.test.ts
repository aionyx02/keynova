import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../../context/IPCContext";
import { IPC } from "../../../ipc/routes";
import { useCapabilityStream } from "./useCapabilityStream";

type EventHandler = (event: { payload: unknown }) => void;

const eventListeners = vi.hoisted(() => new Map<string, EventHandler[]>());
const listenMock = vi.hoisted(() =>
  vi.fn((event: string, handler: EventHandler) => {
    const handlers = eventListeners.get(event) ?? [];
    handlers.push(handler);
    eventListeners.set(event, handlers);
    return Promise.resolve(() => {
      const current = eventListeners.get(event) ?? [];
      eventListeners.set(
        event,
        current.filter((item) => item !== handler),
      );
    });
  }),
);

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

// jsdom path: capability.response events never fire, so the hook stays in
// `pending` until cancelled or unmounted. We assert: idle until submit,
// cancel-on-rerun, args-change resets, unmount cancel, status transitions.

function emitMockEvent(event: string, payload: unknown) {
  for (const handler of eventListeners.get(event) ?? []) {
    handler({ payload });
  }
}

async function withTauriShim(run: () => Promise<void>): Promise<void> {
  const win = window as unknown as { __TAURI_INTERNALS__?: unknown };
  const original = win.__TAURI_INTERNALS__;
  win.__TAURI_INTERNALS__ = {};
  try {
    await run();
  } finally {
    if (original === undefined) {
      delete win.__TAURI_INTERNALS__;
    } else {
      win.__TAURI_INTERNALS__ = original;
    }
  }
}

describe("useCapabilityStream (submit-on-Enter)", () => {
  beforeEach(() => {
    eventListeners.clear();
    listenMock.mockClear();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
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

  it("does NOT auto-dispatch when args change", async () => {
    const dispatch = makeDispatch();
    const { rerender } = renderHook(
      ({ text }) =>
        useCapabilityStream({
          dispatch: dispatch as DispatchFn,
          id: "explain",
          args: { text },
        }),
      { initialProps: { text: "a" } },
    );
    rerender({ text: "ab" });
    rerender({ text: "abc" });
    // No dispatch fires from typing — Enter is required.
    expect(dispatch).not.toHaveBeenCalled();
  });

  it("submit() dispatches capability.call with current args", async () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "rust hashmap" },
      }),
    );
    await act(async () => {
      result.current.submit();
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

  it("submit() is a no-op when args is null", async () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: null,
      }),
    );
    await act(async () => {
      result.current.submit();
    });
    expect(dispatch).not.toHaveBeenCalled();
    expect(result.current.status).toBe("idle");
  });

  it("submit() ignores whitespace-only args", async () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "   " },
      }),
    );
    await act(async () => {
      result.current.submit();
    });
    expect(dispatch).not.toHaveBeenCalled();
  });

  it("status flips to pending after submit", async () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "rust" },
      }),
    );
    expect(result.current.status).toBe("idle");
    await act(async () => {
      result.current.submit();
    });
    expect(result.current.status).toBe("pending");
    expect(result.current.startedAtMs).not.toBeNull();
  });

  it("submitting again with different args cancels previous + dispatches new", async () => {
    const dispatch = makeDispatch();
    const { result, rerender } = renderHook(
      ({ text }) =>
        useCapabilityStream({
          dispatch: dispatch as DispatchFn,
          id: "explain",
          args: { text },
        }),
      { initialProps: { text: "first" } },
    );
    await act(async () => {
      result.current.submit();
    });
    expect(dispatch.mock.calls.length).toBe(1);
    rerender({ text: "second" });
    // Args change after a submit → state resets to idle.
    expect(result.current.status).toBe("idle");
    await act(async () => {
      result.current.submit();
    });
    const routes = dispatch.mock.calls.map((c) => c[0]);
    expect(routes.filter((r) => r === IPC.CAPABILITY_CALL).length).toBe(2);
    expect(routes).toContain(IPC.CAPABILITY_CANCEL);
  });

  it("cancel() flips status to cancelled and dispatches capability.cancel", async () => {
    const dispatch = makeDispatch();
    const { result } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "rust" },
      }),
    );
    await act(async () => {
      result.current.submit();
    });
    dispatch.mockClear();
    act(() => {
      result.current.cancel();
    });
    expect(result.current.status).toBe("cancelled");
    await act(async () => {
      await Promise.resolve();
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
      }),
    );
    await act(async () => {
      result.current.submit();
    });
    expect(result.current.completedAtMs).toBeNull();
    act(() => {
      result.current.cancel();
    });
    expect(result.current.completedAtMs).not.toBeNull();
  });

  it("cancels in-flight request on unmount", async () => {
    const dispatch = makeDispatch();
    const { result, unmount } = renderHook(() =>
      useCapabilityStream({
        dispatch: dispatch as DispatchFn,
        id: "explain",
        args: { text: "rust" },
      }),
    );
    await act(async () => {
      result.current.submit();
    });
    dispatch.mockClear();
    unmount();
    await act(async () => {
      await Promise.resolve();
    });
    expect(dispatch).toHaveBeenCalledWith(IPC.CAPABILITY_CANCEL, expect.objectContaining({}));
  });

  it("uses Tauri alias events and falls back to final response text", async () => {
    await withTauriShim(async () => {
      const dispatch = makeDispatch();
      const { result } = renderHook(() =>
        useCapabilityStream({
          dispatch: dispatch as DispatchFn,
          id: "explain",
          args: { text: "rust ownership" },
        }),
      );

      await act(async () => {
        result.current.submit();
      });

      expect(listenMock).toHaveBeenCalledWith("capability-response", expect.any(Function));
      expect(listenMock).toHaveBeenCalledWith("capability-stream-chunk", expect.any(Function));

      const payload = dispatch.mock.calls.find(([route]) => route === IPC.CAPABILITY_CALL)?.[1] as
        | { request_id: string }
        | undefined;
      expect(payload?.request_id).toBeTruthy();

      act(() => {
        emitMockEvent("capability-response", {
          request_id: payload?.request_id,
          ok: true,
          id: "explain",
          output: { kind: "text", text: "final-only reply" },
          risk_tag: { requires_confirmation: false, reason: "" },
          sources: [
            {
              source_id: "workspace:1",
              source_type: "workspace",
              title: "Keynova",
            },
          ],
        });
      });

      expect(result.current.status).toBe("complete");
      expect(result.current.text).toBe("final-only reply");
      expect(result.current.sources).toEqual([
        {
          source_id: "workspace:1",
          source_type: "workspace",
          title: "Keynova",
          uri: null,
        },
      ]);
    });
  });

  it("projects fix_error structured output into explanation and command fields", async () => {
    await withTauriShim(async () => {
      const dispatch = makeDispatch();
      const { result } = renderHook(() =>
        useCapabilityStream({
          dispatch: dispatch as DispatchFn,
          id: "fix_error",
          args: { text: "error[E0308]: mismatched types" },
        }),
      );

      await act(async () => {
        result.current.submit();
      });
      const payload = dispatch.mock.calls.find(([route]) => route === IPC.CAPABILITY_CALL)?.[1] as
        | { request_id: string }
        | undefined;

      act(() => {
        emitMockEvent("capability-response", {
          request_id: payload?.request_id,
          ok: true,
          id: "fix_error",
          output: {
            kind: "structured",
            value: {
              explanation: "The value has the wrong type.",
              suggested_command: {
                command: "cargo check",
                confidence: 0.6,
                rationale: "Re-run the compiler.",
              },
            },
          },
          risk_tag: { requires_confirmation: false, reason: "" },
          sources: [],
        });
      });

      expect(result.current.status).toBe("complete");
      expect(result.current.text).toBe("The value has the wrong type.");
      expect(result.current.suggestedCommand?.command).toBe("cargo check");
      expect(result.current.riskRequiresConfirmation).toBe(false);
    });
  });
});
