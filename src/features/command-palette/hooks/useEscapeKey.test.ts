import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useEscapeKey, type UseEscapeKeyDeps } from "./useEscapeKey";

interface MutableRef<T> {
  current: T;
}

function makeDeps(overrides: Partial<UseEscapeKeyDeps> = {}): {
  deps: UseEscapeKeyDeps;
  spies: Record<string, ReturnType<typeof vi.fn>>;
  refs: {
    modeRef: MutableRef<string>;
    cmdResultRef: MutableRef<unknown>;
    queryRef: MutableRef<string>;
    secondaryMenuOpenRef: MutableRef<boolean>;
    expandedMetadataRef: MutableRef<boolean>;
    inputRef: MutableRef<HTMLInputElement | null>;
    containerRef: MutableRef<HTMLDivElement | null>;
  };
} {
  const refs = {
    modeRef: { current: "search" },
    cmdResultRef: { current: null as unknown },
    queryRef: { current: "" },
    secondaryMenuOpenRef: { current: false },
    expandedMetadataRef: { current: false },
    inputRef: { current: null as HTMLInputElement | null },
    containerRef: { current: null as HTMLDivElement | null },
  };
  const spies = {
    shouldIgnoreEscape: vi.fn(() => false),
    closeSecondaryMenu: vi.fn(),
    setExpandedMetadata: vi.fn(),
    setCmdResult: vi.fn(),
    setQuery: vi.fn(),
    clearPipeline: vi.fn(),
    clearSearchResults: vi.fn(),
    cancelSearch: vi.fn(),
    hideWindow: vi.fn(),
    keepLauncherOpen: vi.fn(),
  };
  const deps: UseEscapeKeyDeps = {
    shouldIgnoreEscape: spies.shouldIgnoreEscape,
    modeRef: refs.modeRef,
    cmdResultRef: refs.cmdResultRef,
    queryRef: refs.queryRef,
    secondaryMenuOpenRef: refs.secondaryMenuOpenRef,
    expandedMetadataRef: refs.expandedMetadataRef,
    inputRef: refs.inputRef,
    containerRef: refs.containerRef,
    closeSecondaryMenu: spies.closeSecondaryMenu,
    setExpandedMetadata: spies.setExpandedMetadata,
    setCmdResult: spies.setCmdResult,
    setQuery: spies.setQuery,
    clearPipeline: spies.clearPipeline,
    clearSearchResults: spies.clearSearchResults,
    cancelSearch: spies.cancelSearch,
    hideWindow: spies.hideWindow,
    keepLauncherOpen: spies.keepLauncherOpen,
    ...overrides,
  };
  return { deps, spies, refs };
}

function fireEscape() {
  const event = new KeyboardEvent("keydown", { key: "Escape" });
  window.dispatchEvent(event);
}

describe("useEscapeKey", () => {
  beforeEach(() => {
    // Replace requestAnimationFrame with synchronous timer so we can assert
    // post-ESC focus / clear callbacks deterministically.
    vi.stubGlobal(
      "requestAnimationFrame",
      (cb: FrameRequestCallback) => setTimeout(() => cb(Date.now()), 0) as unknown as number,
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("ignores non-Escape keys", () => {
    const { deps, spies } = makeDeps();
    renderHook(() => useEscapeKey(deps));
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    expect(spies.hideWindow).not.toHaveBeenCalled();
    expect(spies.setQuery).not.toHaveBeenCalled();
  });

  it("shouldIgnoreEscape=true short-circuits", () => {
    const { deps, spies } = makeDeps();
    spies.shouldIgnoreEscape.mockReturnValue(true);
    renderHook(() => useEscapeKey(deps));
    fireEscape();
    expect(spies.hideWindow).not.toHaveBeenCalled();
    expect(spies.closeSecondaryMenu).not.toHaveBeenCalled();
  });

  it("priority 1: secondary menu open → closeSecondaryMenu", () => {
    const { deps, spies, refs } = makeDeps();
    refs.secondaryMenuOpenRef.current = true;
    renderHook(() => useEscapeKey(deps));
    fireEscape();
    expect(spies.closeSecondaryMenu).toHaveBeenCalledTimes(1);
    expect(spies.setExpandedMetadata).not.toHaveBeenCalled();
    expect(spies.hideWindow).not.toHaveBeenCalled();
  });

  it("priority 2: expanded metadata → collapse only", () => {
    const { deps, spies, refs } = makeDeps();
    refs.expandedMetadataRef.current = true;
    renderHook(() => useEscapeKey(deps));
    fireEscape();
    expect(spies.setExpandedMetadata).toHaveBeenCalledWith(false);
    expect(spies.closeSecondaryMenu).not.toHaveBeenCalled();
    expect(spies.hideWindow).not.toHaveBeenCalled();
  });

  it("priority 3: terminal mode → keepLauncherOpen + clear query", () => {
    const { deps, spies, refs } = makeDeps();
    refs.modeRef.current = "terminal";
    renderHook(() => useEscapeKey(deps));
    fireEscape();
    expect(spies.setQuery).toHaveBeenCalledWith("");
    expect(spies.keepLauncherOpen).toHaveBeenCalledTimes(1);
    expect(spies.cancelSearch).not.toHaveBeenCalled();
    expect(spies.hideWindow).not.toHaveBeenCalled();
  });

  it("priority 4: cmdResult set → unwind cmd + pipeline + search", () => {
    const { deps, spies, refs } = makeDeps();
    refs.cmdResultRef.current = { text: "x", ui_type: { type: "Inline" } };
    renderHook(() => useEscapeKey(deps));
    fireEscape();
    expect(spies.cancelSearch).toHaveBeenCalled();
    expect(spies.setCmdResult).toHaveBeenCalledWith(null);
    expect(spies.clearPipeline).toHaveBeenCalled();
    expect(spies.setQuery).toHaveBeenCalledWith("");
    expect(spies.clearSearchResults).toHaveBeenCalled();
    expect(spies.hideWindow).not.toHaveBeenCalled();
  });

  it("priority 5: non-empty query → clear query only", () => {
    const { deps, spies, refs } = makeDeps();
    refs.queryRef.current = "typed something";
    renderHook(() => useEscapeKey(deps));
    fireEscape();
    expect(spies.cancelSearch).toHaveBeenCalled();
    expect(spies.setQuery).toHaveBeenCalledWith("");
    expect(spies.clearSearchResults).toHaveBeenCalled();
    expect(spies.clearPipeline).toHaveBeenCalled();
    expect(spies.setCmdResult).not.toHaveBeenCalled();
    expect(spies.hideWindow).not.toHaveBeenCalled();
  });

  it("priority 6: empty state → hideWindow", () => {
    const { deps, spies } = makeDeps();
    renderHook(() => useEscapeKey(deps));
    fireEscape();
    expect(spies.hideWindow).toHaveBeenCalledTimes(1);
    expect(spies.setQuery).not.toHaveBeenCalled();
    expect(spies.cancelSearch).not.toHaveBeenCalled();
  });

  it("unregisters the listener on unmount", () => {
    const { deps, spies } = makeDeps();
    const { unmount } = renderHook(() => useEscapeKey(deps));
    unmount();
    fireEscape();
    expect(spies.hideWindow).not.toHaveBeenCalled();
  });
});
