import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useQueryChange } from "./useQueryChange";

type QueryChangeDeps = Parameters<typeof useQueryChange>[0];

function makeDeps(overrides: Partial<QueryChangeDeps> = {}) {
  const spies = {
    setQuery: vi.fn(),
    setCmdResult: vi.fn(),
    clearCopyHint: vi.fn(),
    clearPipeline: vi.fn(),
    setSelectedCmd: vi.fn(),
    setSelectedArg: vi.fn(),
    setArgSuggestions: vi.fn(),
    clearRecentlyDeleted: vi.fn(),
    setTerminalMounted: vi.fn(),
    clearSearchResults: vi.fn(),
    cancelSearch: vi.fn(),
    triggerSearch: vi.fn(),
  };
  const deps: QueryChangeDeps = {
    ...spies,
    ...overrides,
  };
  return { deps, spies };
}

describe("useQueryChange", () => {
  it("lets slashless utility queries reach search providers", () => {
    const { deps, spies } = makeDeps();
    const { result } = renderHook(() => useQueryChange(deps));

    result.current('json {"a":1}');

    expect(spies.setQuery).toHaveBeenCalledWith('json {"a":1}');
    expect(spies.triggerSearch).toHaveBeenCalledWith('json {"a":1}');
    expect(spies.clearSearchResults).not.toHaveBeenCalled();
    expect(spies.cancelSearch).not.toHaveBeenCalled();
  });

  it("suppresses backend search for capability prefixes", () => {
    const { deps, spies } = makeDeps();
    const { result } = renderHook(() => useQueryChange(deps));

    result.current("explain rust ownership");

    expect(spies.setQuery).toHaveBeenCalledWith("explain rust ownership");
    expect(spies.clearSearchResults).toHaveBeenCalledTimes(1);
    expect(spies.cancelSearch).toHaveBeenCalledTimes(1);
    expect(spies.triggerSearch).not.toHaveBeenCalled();
  });

  it("keeps direct terminal input disabled in query-change handling", () => {
    const { deps, spies } = makeDeps();
    const { result } = renderHook(() => useQueryChange(deps));

    result.current("> npm run dev");

    expect(spies.clearSearchResults).toHaveBeenCalledTimes(1);
    expect(spies.cancelSearch).toHaveBeenCalledTimes(1);
    expect(spies.triggerSearch).not.toHaveBeenCalled();
    expect(spies.setTerminalMounted).not.toHaveBeenCalled();
  });
});
