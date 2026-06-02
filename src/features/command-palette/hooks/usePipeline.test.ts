import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../../context/IPCContext";
import { usePipeline, type PipelineReport } from "./usePipeline";

function happyDispatch<T>(value: T): DispatchFn {
  return (async () => value as unknown) as DispatchFn;
}

function failingDispatch(message: string): DispatchFn {
  return (async () => {
    throw new Error(message);
  }) as DispatchFn;
}

const sampleReport: PipelineReport = {
  log: { workflow_name: "test", status: "ok", action_count: 1 },
  actions: [{ index: 0, route: "echo", status: "ok", output: "hi" }],
};

describe("usePipeline", () => {
  it("initial state has no result and not running", () => {
    const { result } = renderHook(() => usePipeline({ dispatch: happyDispatch(sampleReport) }));
    expect(result.current.pipelineResult).toBeNull();
    expect(result.current.pipelineRunning).toBe(false);
  });

  it("runPipeline sets running then writes result on success", async () => {
    const dispatch = happyDispatch(sampleReport);
    const { result } = renderHook(() => usePipeline({ dispatch }));
    await act(async () => {
      await result.current.runPipeline("a | b");
    });
    expect(result.current.pipelineRunning).toBe(false);
    expect(result.current.pipelineResult).toEqual(sampleReport);
  });

  it("runPipeline writes a failed log when dispatch throws", async () => {
    const dispatch = failingDispatch("kaboom");
    const { result } = renderHook(() => usePipeline({ dispatch }));
    await act(async () => {
      await result.current.runPipeline("a | b");
    });
    expect(result.current.pipelineRunning).toBe(false);
    const report = result.current.pipelineResult;
    expect(report?.log.status).toBe("failed");
    expect(report?.log.error).toContain("kaboom");
    expect(report?.actions).toEqual([]);
  });

  it("clear resets result and running flags", async () => {
    const dispatch = happyDispatch(sampleReport);
    const { result } = renderHook(() => usePipeline({ dispatch }));
    await act(async () => {
      await result.current.runPipeline("a | b");
    });
    expect(result.current.pipelineResult).not.toBeNull();
    act(() => result.current.clear());
    expect(result.current.pipelineResult).toBeNull();
    expect(result.current.pipelineRunning).toBe(false);
  });

  it("calling runPipeline forwards the supplied text into dispatch payload", async () => {
    const spy = vi.fn(async () => sampleReport);
    const dispatch = spy as unknown as DispatchFn;
    const { result } = renderHook(() => usePipeline({ dispatch }));
    await act(async () => {
      await result.current.runPipeline("foo | bar");
    });
    await waitFor(() =>
      expect(spy.mock.calls[0]).toEqual(["automation.execute_pipeline", { text: "foo | bar" }]),
    );
  });
});
