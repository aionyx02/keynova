// REF.6.B — focused regression for the capability-mode Enter routing.
//
// The hook exposes a single `onKeyDown` handler. Wide deps stubbed to no-ops;
// only the capability-mode branch is exercised. If this test fails the
// frontend ↔ capability submit wire is broken.

import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { useKeyboardNav, type UseKeyboardNavDeps } from "./useKeyboardNav";

function makeDeps(overrides: Partial<UseKeyboardNavDeps> = {}): UseKeyboardNavDeps {
  const noop = vi.fn();
  const setNoop = vi.fn() as unknown as React.Dispatch<React.SetStateAction<unknown>>;
  return {
    mode: "search",
    query: "explain foo",
    cmdResult: null,
    visibleResults: [],
    safeSelected: 0,
    setSelected: setNoop as never,
    secondaryMenuOpen: false,
    setSecondaryMenuOpen: setNoop as never,
    menuFocusedIndex: 0,
    setMenuFocusedIndex: setNoop as never,
    closeSecondaryMenu: noop,
    setCheatsheetOpen: setNoop as never,
    cmdName: "",
    cmdArgs: "",
    cmdSuggestions: [],
    selectedCmd: 0,
    setSelectedCmd: setNoop as never,
    isArgsPhase: false,
    argSuggestions: [],
    selectedArg: 0,
    setSelectedArg: setNoop as never,
    setQuery: noop,
    copyResultLocation: vi.fn(async () => undefined),
    handleSecondaryAction: vi.fn(async () => undefined),
    launchResult: vi.fn(async () => undefined),
    runFirstSecondary: vi.fn(async () => undefined),
    runPipeline: vi.fn(async () => undefined),
    execCommand: vi.fn(async () => undefined),
    capabilityMode: false,
    onCapabilitySubmit: vi.fn(),
    keepLauncherOpen: vi.fn(),
    ...overrides,
  };
}

function fakeEnter(opts: { shiftKey?: boolean; isComposing?: boolean } = {}): React.KeyboardEvent<HTMLInputElement> {
  const preventDefault = vi.fn();
  return {
    key: "Enter",
    shiftKey: opts.shiftKey ?? false,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    preventDefault,
    nativeEvent: { isComposing: opts.isComposing ?? false } as KeyboardEvent,
  } as unknown as React.KeyboardEvent<HTMLInputElement>;
}

describe("useKeyboardNav — capability Enter routing (REF.6.B)", () => {
  it("fires onCapabilitySubmit on Enter when capabilityMode is true", () => {
    const onCapabilitySubmit = vi.fn();
    const launchResult = vi.fn(async () => undefined);
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          capabilityMode: true,
          onCapabilitySubmit,
          launchResult,
        }),
      ),
    );
    const e = fakeEnter();
    result.current.onKeyDown(e);
    expect(onCapabilitySubmit).toHaveBeenCalledTimes(1);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    // Capability branch returns before search launch fires.
    expect(launchResult).not.toHaveBeenCalled();
  });

  it("does NOT fire submit when capabilityMode is false", () => {
    const onCapabilitySubmit = vi.fn();
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          capabilityMode: false,
          onCapabilitySubmit,
        }),
      ),
    );
    result.current.onKeyDown(fakeEnter());
    expect(onCapabilitySubmit).not.toHaveBeenCalled();
  });

  it("does NOT fire submit when IME composition is active", () => {
    const onCapabilitySubmit = vi.fn();
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          capabilityMode: true,
          onCapabilitySubmit,
        }),
      ),
    );
    result.current.onKeyDown(fakeEnter({ isComposing: true }));
    // Lets the IME commit its candidate; subsequent Enter (isComposing=false)
    // would fire submit.
    expect(onCapabilitySubmit).not.toHaveBeenCalled();
  });

  it("does NOT fire submit on Shift+Enter", () => {
    const onCapabilitySubmit = vi.fn();
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          capabilityMode: true,
          onCapabilitySubmit,
        }),
      ),
    );
    result.current.onKeyDown(fakeEnter({ shiftKey: true }));
    expect(onCapabilitySubmit).not.toHaveBeenCalled();
  });
});
