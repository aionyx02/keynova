import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../../context/IPCContext";
import { useSearchBackend } from "./useSearchBackend";

// useSearchBackend is a thin wrapper around a Tauri `listen`-driven effect.
// In jsdom without `window.__TAURI_INTERNALS__`, the hook short-circuits
// and stays at its initial state — that is what we assert here.
//
// Full event flow (config-reloaded → refresh → setSearchBackend) requires
// Tauri context and is covered by manual REF.2.P6 regression.

describe("useSearchBackend", () => {
  it("starts with no backend info and does not call dispatch outside Tauri", () => {
    const dispatch = vi.fn() as unknown as DispatchFn;
    const { result } = renderHook(() => useSearchBackend({ dispatch }));
    expect(result.current.searchBackend).toBeNull();
    expect(dispatch).not.toHaveBeenCalled();
  });
});
