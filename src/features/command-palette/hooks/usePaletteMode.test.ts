import { describe, it, expect } from "vitest";
import { renderHook } from "@testing-library/react";

import { usePaletteMode } from "./usePaletteMode";

describe("usePaletteMode", () => {
  it("returns search kind for empty query", () => {
    const { result } = renderHook(() => usePaletteMode(""));
    expect(result.current).toEqual({ kind: "search" });
  });

  it("returns search kind for a plain search query", () => {
    const { result } = renderHook(() => usePaletteMode("hashmap remove"));
    expect(result.current).toEqual({ kind: "search" });
  });

  it("returns capability kind on explain prefix", () => {
    const { result } = renderHook(() => usePaletteMode("explain rust hashmap"));
    expect(result.current).toEqual({
      kind: "capability",
      id: "explain",
      args: { text: "rust hashmap" },
    });
  });

  it("returns capability kind on summarize prefix", () => {
    const { result } = renderHook(() => usePaletteMode("summarize hello world"));
    expect(result.current).toEqual({
      kind: "capability",
      id: "summarize",
      args: { text: "hello world" },
    });
  });

  it("returns search kind when query is terminal sigil (parseInputMode pre-empts)", () => {
    const { result } = renderHook(() => usePaletteMode("> explain rust"));
    expect(result.current).toEqual({ kind: "search" });
  });

  it("returns search kind when query is command sigil", () => {
    const { result } = renderHook(() => usePaletteMode("/help"));
    expect(result.current).toEqual({ kind: "search" });
  });

  it("preserves object identity across equivalent rerenders", () => {
    const { result, rerender } = renderHook(({ q }) => usePaletteMode(q), {
      initialProps: { q: "explain rust" },
    });
    const first = result.current;
    rerender({ q: "explain rust" });
    expect(result.current).toBe(first);
  });

  it("returns a new object when the query changes meaningfully", () => {
    const { result, rerender } = renderHook(({ q }) => usePaletteMode(q), {
      initialProps: { q: "explain rust" },
    });
    const first = result.current;
    rerender({ q: "explain hashmap" });
    expect(result.current).not.toBe(first);
    expect(result.current).toMatchObject({
      kind: "capability",
      id: "explain",
      args: { text: "hashmap" },
    });
  });

  it("transitions from capability back to search when prefix dies", () => {
    const { result, rerender } = renderHook(({ q }) => usePaletteMode(q), {
      initialProps: { q: "explain rust" },
    });
    expect(result.current.kind).toBe("capability");
    rerender({ q: "explain" });
    expect(result.current.kind).toBe("search");
  });

  it("transitions between capabilities", () => {
    const { result, rerender } = renderHook(({ q }) => usePaletteMode(q), {
      initialProps: { q: "explain foo" },
    });
    expect(result.current).toMatchObject({ kind: "capability", id: "explain" });
    rerender({ q: "summarize foo" });
    expect(result.current).toMatchObject({ kind: "capability", id: "summarize" });
  });
});
