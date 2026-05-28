import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../../context/IPCContext";
import { useLauncherSettings } from "./useLauncherSettings";

// Same notes as useSearchBackend: Tauri listener flow is not exercised in
// jsdom; only the initial state + non-Tauri short-circuit are unit-tested.

describe("useLauncherSettings", () => {
  it("defaults previewEnabled / showRankBreakdown to true", () => {
    const dispatch = vi.fn() as unknown as DispatchFn;
    const { result } = renderHook(() =>
      useLauncherSettings({ dispatch, onMaxResultsChange: () => {} }),
    );
    expect(result.current.previewEnabled).toBe(true);
    expect(result.current.showRankBreakdown).toBe(true);
  });

  it("does not call dispatch outside Tauri", () => {
    const dispatch = vi.fn() as unknown as DispatchFn;
    const onMaxResultsChange = vi.fn();
    renderHook(() => useLauncherSettings({ dispatch, onMaxResultsChange }));
    expect(dispatch).not.toHaveBeenCalled();
    expect(onMaxResultsChange).not.toHaveBeenCalled();
  });
});
