// Focused regression for the capability-mode Enter routing.
//
// The hook exposes a single `onKeyDown` handler. Wide deps stubbed to no-ops;
// only the capability-mode branch is exercised. If this test fails the
// frontend ↔ capability submit wire is broken.

import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { SearchResult } from "../../../types/search";
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
    copyCommandResult: vi.fn(async () => undefined),
    copyResultLocation: vi.fn(async () => undefined),
    handleSecondaryAction: vi.fn(async () => undefined),
    launchResult: vi.fn(async () => undefined),
    runFirstSecondary: vi.fn(async () => undefined),
    runPipeline: vi.fn(async () => undefined),
    execCommand: vi.fn(async () => undefined),
    capabilityMode: false,
    capabilityListMode: false,
    capabilityListCount: 0,
    capabilityListSelected: 0,
    setCapabilityListSelected: setNoop as never,
    onCapabilitySubmit: vi.fn(),
    onCapabilityRunSelected: vi.fn(),
    capabilityPinMode: false,
    onCapabilityTogglePin: vi.fn(),
    keepLauncherOpen: vi.fn(),
    ...overrides,
  };
}

function fileResult(overrides: Partial<SearchResult> = {}): SearchResult {
  return {
    kind: "file",
    name: "README.md",
    path: "C:\\repo\\README.md",
    score: 100,
    ...overrides,
  };
}

function fakeKey(
  key: string,
  opts: {
    shiftKey?: boolean;
    ctrlKey?: boolean;
    metaKey?: boolean;
    altKey?: boolean;
    isComposing?: boolean;
    value?: string;
    selectionStart?: number;
    selectionEnd?: number;
  } = {},
): React.KeyboardEvent<HTMLInputElement> {
  const preventDefault = vi.fn();
  const value = opts.value ?? "";
  const selectionStart = opts.selectionStart ?? value.length;
  const selectionEnd = opts.selectionEnd ?? selectionStart;
  return {
    key,
    shiftKey: opts.shiftKey ?? false,
    ctrlKey: opts.ctrlKey ?? false,
    metaKey: opts.metaKey ?? false,
    altKey: opts.altKey ?? false,
    preventDefault,
    currentTarget: {
      value,
      selectionStart,
      selectionEnd,
    },
    nativeEvent: { isComposing: opts.isComposing ?? false } as KeyboardEvent,
  } as unknown as React.KeyboardEvent<HTMLInputElement>;
}

function fakeEnter(
  opts: { shiftKey?: boolean; isComposing?: boolean } = {},
): React.KeyboardEvent<HTMLInputElement> {
  return fakeKey("Enter", opts);
}

function fakeArrow(key: "ArrowDown" | "ArrowUp"): React.KeyboardEvent<HTMLInputElement> {
  return fakeKey(key);
}

describe("useKeyboardNav - capability Enter routing", () => {
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

  it("routes next-mode ArrowDown to list selection", () => {
    const setCapabilityListSelected = vi.fn();
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          capabilityMode: true,
          capabilityListMode: true,
          capabilityListCount: 3,
          setCapabilityListSelected: setCapabilityListSelected as never,
        }),
      ),
    );
    const e = fakeArrow("ArrowDown");
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(setCapabilityListSelected).toHaveBeenCalledTimes(1);
  });

  it("runs the selected next suggestion on Enter", () => {
    const onCapabilityRunSelected = vi.fn();
    const onCapabilitySubmit = vi.fn();
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          capabilityMode: true,
          capabilityListMode: true,
          capabilityListCount: 2,
          capabilityListSelected: 1,
          onCapabilityRunSelected,
          onCapabilitySubmit,
        }),
      ),
    );
    const e = fakeEnter();
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(onCapabilityRunSelected).toHaveBeenCalledTimes(1);
    expect(onCapabilitySubmit).not.toHaveBeenCalled();
  });

  it("toggles the pin on Ctrl+P in profile (pin) mode (PROFILE.2)", () => {
    const onCapabilityTogglePin = vi.fn();
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          capabilityMode: true,
          capabilityListMode: true,
          capabilityPinMode: true,
          capabilityListCount: 2,
          onCapabilityTogglePin,
        }),
      ),
    );
    const e = fakeKey("p", { ctrlKey: true });
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(onCapabilityTogglePin).toHaveBeenCalledTimes(1);
  });

  it("does NOT toggle pin on Ctrl+P when not in pin mode (idle next)", () => {
    const onCapabilityTogglePin = vi.fn();
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          capabilityMode: true,
          capabilityListMode: true,
          capabilityPinMode: false,
          capabilityListCount: 2,
          onCapabilityTogglePin,
        }),
      ),
    );
    result.current.onKeyDown(fakeKey("p", { ctrlKey: true }));
    expect(onCapabilityTogglePin).not.toHaveBeenCalled();
  });
});

describe("useKeyboardNav - search result keyboard paths", () => {
  it("launches the selected search result on Enter", () => {
    const row = fileResult();
    const launchResult = vi.fn(async () => undefined);
    const runFirstSecondary = vi.fn(async () => undefined);
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          query: "readme",
          visibleResults: [row],
          safeSelected: 0,
          launchResult,
          runFirstSecondary,
        }),
      ),
    );
    const e = fakeEnter();
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(launchResult).toHaveBeenCalledWith(row);
    expect(runFirstSecondary).not.toHaveBeenCalled();
  });

  it("runs the first secondary action on Shift+Enter", () => {
    const row = fileResult();
    const launchResult = vi.fn(async () => undefined);
    const runFirstSecondary = vi.fn(async () => undefined);
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          query: "readme",
          visibleResults: [row],
          safeSelected: 0,
          launchResult,
          runFirstSecondary,
        }),
      ),
    );
    const e = fakeEnter({ shiftKey: true });
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(runFirstSecondary).toHaveBeenCalledWith(row);
    expect(launchResult).not.toHaveBeenCalled();
  });

  it("opens the secondary menu with Tab at the end of the input", () => {
    const row = fileResult();
    const setSecondaryMenuOpen = vi.fn();
    const setMenuFocusedIndex = vi.fn();
    const launchResult = vi.fn(async () => undefined);
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          query: "readme",
          visibleResults: [row],
          safeSelected: 0,
          setSecondaryMenuOpen: setSecondaryMenuOpen as never,
          setMenuFocusedIndex: setMenuFocusedIndex as never,
          launchResult,
        }),
      ),
    );
    const e = fakeKey("Tab", { value: "readme", selectionStart: 6, selectionEnd: 6 });
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(setSecondaryMenuOpen).toHaveBeenCalledWith(true);
    expect(setMenuFocusedIndex).toHaveBeenCalledWith(0);
    expect(launchResult).not.toHaveBeenCalled();
  });

  it("copies a selected file location with Ctrl+C when no text is selected", () => {
    const row = fileResult();
    const copyResultLocation = vi.fn(async () => undefined);
    const launchResult = vi.fn(async () => undefined);
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          query: "readme",
          visibleResults: [row],
          safeSelected: 0,
          copyResultLocation,
          launchResult,
        }),
      ),
    );
    const e = fakeKey("c", { ctrlKey: true, value: "readme", selectionStart: 6, selectionEnd: 6 });
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(copyResultLocation).toHaveBeenCalledWith(row);
    expect(launchResult).not.toHaveBeenCalled();
  });

  it("copies an inline command result with Ctrl+C before selected row paths", () => {
    const row = fileResult();
    const copyCommandResult = vi.fn(async () => undefined);
    const copyResultLocation = vi.fn(async () => undefined);
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          query: "json",
          cmdResult: { text: '{\n  "a": 1\n}', ui_type: { type: "Inline" } },
          visibleResults: [row],
          safeSelected: 0,
          copyCommandResult,
          copyResultLocation,
        }),
      ),
    );
    const e = fakeKey("c", { ctrlKey: true, value: "json", selectionStart: 4, selectionEnd: 4 });
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(copyCommandResult).toHaveBeenCalledWith('{\n  "a": 1\n}');
    expect(copyResultLocation).not.toHaveBeenCalled();
  });

  it("runs the focused secondary menu item on Enter", () => {
    const row = fileResult();
    const handleSecondaryAction = vi.fn(async () => undefined);
    const launchResult = vi.fn(async () => undefined);
    const { result } = renderHook(() =>
      useKeyboardNav(
        makeDeps({
          query: "readme",
          visibleResults: [row],
          safeSelected: 0,
          secondaryMenuOpen: true,
          menuFocusedIndex: 1,
          handleSecondaryAction,
          launchResult,
        }),
      ),
    );
    const e = fakeEnter();
    result.current.onKeyDown(e);
    expect(e.preventDefault).toHaveBeenCalledTimes(1);
    expect(handleSecondaryAction).toHaveBeenCalledWith("copy_path", row);
    expect(launchResult).not.toHaveBeenCalled();
  });
});
