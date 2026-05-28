import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { useSecondaryMenu } from "./useSecondaryMenu";

describe("useSecondaryMenu", () => {
  it("initial state is closed / index 0 / no confirm / no input", () => {
    const { result } = renderHook(() => useSecondaryMenu());
    expect(result.current.secondaryMenuOpen).toBe(false);
    expect(result.current.menuFocusedIndex).toBe(0);
    expect(result.current.expandedMetadata).toBe(false);
    expect(result.current.pendingConfirm).toBeNull();
    expect(result.current.inlineInput).toBeNull();
  });

  it("closeSecondaryMenu resets menu / index / pendingConfirm / inlineInput", () => {
    const { result } = renderHook(() => useSecondaryMenu());
    act(() => {
      result.current.setSecondaryMenuOpen(true);
      result.current.setMenuFocusedIndex(3);
      result.current.setPendingConfirm("delete");
      result.current.setInlineInput({ for: "rename", value: "x" });
    });
    expect(result.current.secondaryMenuOpen).toBe(true);
    expect(result.current.menuFocusedIndex).toBe(3);
    expect(result.current.pendingConfirm).toBe("delete");
    expect(result.current.inlineInput?.value).toBe("x");

    act(() => result.current.closeSecondaryMenu());
    expect(result.current.secondaryMenuOpen).toBe(false);
    expect(result.current.menuFocusedIndex).toBe(0);
    expect(result.current.pendingConfirm).toBeNull();
    expect(result.current.inlineInput).toBeNull();
  });

  it("closeSecondaryMenu leaves expandedMetadata alone", () => {
    const { result } = renderHook(() => useSecondaryMenu());
    act(() => result.current.setExpandedMetadata(true));
    act(() => result.current.closeSecondaryMenu());
    expect(result.current.expandedMetadata).toBe(true);
  });
});
