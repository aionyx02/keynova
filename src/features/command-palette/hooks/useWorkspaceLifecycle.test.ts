import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useWorkspaceLifecycle } from "./useWorkspaceLifecycle";

// Side-effect-only hook (3 Tauri listen() bindings). In jsdom we can only
// assert that mounting the hook does not crash and does not invoke its
// reset callbacks. The actual event flow (workspace-switched / cycled /
// window-focused -> reset chain) is exercised by manual regression.

describe("useWorkspaceLifecycle", () => {
  it("mounts without invoking callbacks outside Tauri", () => {
    const modeRef = { current: "search" };
    const inputRef = { current: null as HTMLInputElement | null };
    const setQuery = vi.fn();
    const setCmdResult = vi.fn();
    const clearSearchResults = vi.fn();
    const cancelSearch = vi.fn();
    const clearRecentlyDeleted = vi.fn();
    renderHook(() =>
      useWorkspaceLifecycle({
        modeRef,
        inputRef,
        setQuery,
        setCmdResult,
        clearSearchResults,
        cancelSearch,
        clearRecentlyDeleted,
      }),
    );
    expect(setQuery).not.toHaveBeenCalled();
    expect(setCmdResult).not.toHaveBeenCalled();
    expect(clearSearchResults).not.toHaveBeenCalled();
    expect(cancelSearch).not.toHaveBeenCalled();
    expect(clearRecentlyDeleted).not.toHaveBeenCalled();
  });
});
