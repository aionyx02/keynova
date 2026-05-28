import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useCopyHint } from "./useCopyHint";

describe("useCopyHint", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("initial state is nulls", () => {
    const { result } = renderHook(() => useCopyHint());
    expect(result.current.copiedPath).toBeNull();
    expect(result.current.copyHint).toBeNull();
  });

  it("flashCopiedPath sets copiedPath and auto-clears at default TTL", () => {
    const { result } = renderHook(() => useCopyHint());
    act(() => result.current.flashCopiedPath("/tmp/foo"));
    expect(result.current.copiedPath).toBe("/tmp/foo");
    expect(result.current.copyHint).toBeNull();
    act(() => {
      vi.advanceTimersByTime(1200);
    });
    expect(result.current.copiedPath).toBeNull();
  });

  it("flashCopyHint sets copyHint and auto-clears at default TTL", () => {
    const { result } = renderHook(() => useCopyHint());
    act(() => result.current.flashCopyHint("done"));
    expect(result.current.copyHint).toBe("done");
    expect(result.current.copiedPath).toBeNull();
    act(() => {
      vi.advanceTimersByTime(1200);
    });
    expect(result.current.copyHint).toBeNull();
  });

  it("flashCopiedPath clears any prior copyHint", () => {
    const { result } = renderHook(() => useCopyHint());
    act(() => result.current.flashCopyHint("first"));
    expect(result.current.copyHint).toBe("first");
    act(() => result.current.flashCopiedPath("/p"));
    expect(result.current.copyHint).toBeNull();
    expect(result.current.copiedPath).toBe("/p");
  });

  it("flashCopyHint clears any prior copiedPath", () => {
    const { result } = renderHook(() => useCopyHint());
    act(() => result.current.flashCopiedPath("/x"));
    expect(result.current.copiedPath).toBe("/x");
    act(() => result.current.flashCopyHint("note"));
    expect(result.current.copiedPath).toBeNull();
    expect(result.current.copyHint).toBe("note");
  });

  it("flashCopyHint accepts a custom duration", () => {
    const { result } = renderHook(() => useCopyHint());
    act(() => result.current.flashCopyHint("hi", 3000));
    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(result.current.copyHint).toBe("hi");
    act(() => {
      vi.advanceTimersByTime(1500);
    });
    expect(result.current.copyHint).toBeNull();
  });

  it("clear() resets immediately + cancels pending timeout", () => {
    const { result } = renderHook(() => useCopyHint());
    act(() => result.current.flashCopyHint("ephemeral", 5000));
    expect(result.current.copyHint).toBe("ephemeral");
    act(() => result.current.clear());
    expect(result.current.copyHint).toBeNull();
    // advancing past the original TTL should not re-fire anything
    act(() => {
      vi.advanceTimersByTime(10000);
    });
    expect(result.current.copyHint).toBeNull();
    expect(result.current.copiedPath).toBeNull();
  });

  it("a second flash supersedes the first timer", () => {
    const { result } = renderHook(() => useCopyHint());
    act(() => result.current.flashCopyHint("first"));
    act(() => {
      vi.advanceTimersByTime(800);
    });
    act(() => result.current.flashCopyHint("second"));
    // 800ms passed but first timer was replaced; original deadline shouldn't fire
    act(() => {
      vi.advanceTimersByTime(800);
    });
    expect(result.current.copyHint).toBe("second");
    act(() => {
      vi.advanceTimersByTime(400);
    });
    expect(result.current.copyHint).toBeNull();
  });
});
