import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { RECENTLY_DELETED_TTL_MS, useRecentlyDeleted } from "./useRecentlyDeleted";

describe("useRecentlyDeleted", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-05-20T00:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("isDeleted returns false for unknown paths", () => {
    const { result } = renderHook(() => useRecentlyDeleted());
    expect(result.current.isDeleted("/never-touched")).toBe(false);
  });

  it("markDeleted then isDeleted returns true within TTL", () => {
    const { result } = renderHook(() => useRecentlyDeleted());
    act(() => result.current.markDeleted("/tmp/trash.txt"));
    expect(result.current.isDeleted("/tmp/trash.txt")).toBe(true);
  });

  it("isDeleted returns false once TTL elapses", () => {
    const { result } = renderHook(() => useRecentlyDeleted());
    act(() => result.current.markDeleted("/tmp/trash.txt"));
    vi.advanceTimersByTime(RECENTLY_DELETED_TTL_MS + 1);
    expect(result.current.isDeleted("/tmp/trash.txt")).toBe(false);
  });

  it("isDeleted still true at exact TTL boundary", () => {
    const { result } = renderHook(() => useRecentlyDeleted());
    act(() => result.current.markDeleted("/tmp/trash.txt"));
    vi.advanceTimersByTime(RECENTLY_DELETED_TTL_MS - 1);
    expect(result.current.isDeleted("/tmp/trash.txt")).toBe(true);
  });

  it("clear() drops the kill set", () => {
    const { result } = renderHook(() => useRecentlyDeleted());
    act(() => {
      result.current.markDeleted("/a");
      result.current.markDeleted("/b");
    });
    expect(result.current.isDeleted("/a")).toBe(true);
    act(() => result.current.clear());
    expect(result.current.isDeleted("/a")).toBe(false);
    expect(result.current.isDeleted("/b")).toBe(false);
  });

  it("supports independent paths with independent TTLs", () => {
    const { result } = renderHook(() => useRecentlyDeleted());
    act(() => result.current.markDeleted("/old"));
    vi.advanceTimersByTime(20_000);
    act(() => result.current.markDeleted("/new"));
    // /old has aged 20s; /new is fresh
    expect(result.current.isDeleted("/old")).toBe(true);
    expect(result.current.isDeleted("/new")).toBe(true);
    vi.advanceTimersByTime(11_000);
    // /old is now 31s in, past TTL; /new is 11s in, still alive
    expect(result.current.isDeleted("/old")).toBe(false);
    expect(result.current.isDeleted("/new")).toBe(true);
  });
});
