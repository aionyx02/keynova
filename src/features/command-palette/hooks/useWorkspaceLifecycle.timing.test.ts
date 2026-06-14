import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { __resetPerfForTest, summarizePerf } from "../devTiming";
import { useWorkspaceLifecycle } from "./useWorkspaceLifecycle";

// Integration-level check that the dev timing instrument is actually wired into
// the real window-focused reopen path (the primary Ctrl+K path), not just the
// devTiming util in isolation. We mock the Tauri `listen` binding to capture and
// fire the event handler the hook registers.

const listeners = new Map<string, (event: unknown) => void>();

vi.mock("@tauri-apps/api/event", () => ({
  listen: (name: string, cb: (event: unknown) => void) => {
    listeners.set(name, cb);
    return Promise.resolve(() => listeners.delete(name));
  },
}));

function captureRaf(): { run: () => void } {
  const cbs: FrameRequestCallback[] = [];
  vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
    cbs.push(cb);
    return cbs.length;
  });
  return { run: () => cbs.splice(0).forEach((cb) => cb(performance.now())) };
}

function baseDeps(overrides: Partial<Parameters<typeof useWorkspaceLifecycle>[0]>) {
  return {
    modeRef: { current: "search" } as React.RefObject<string>,
    inputRef: { current: null } as React.RefObject<HTMLInputElement | null>,
    setQuery: vi.fn(),
    setCmdResult: vi.fn(),
    clearSearchResults: vi.fn(),
    cancelSearch: vi.fn(),
    clearRecentlyDeleted: vi.fn(),
    ...overrides,
  };
}

describe("useWorkspaceLifecycle timing wiring", () => {
  beforeEach(() => {
    __resetPerfForTest();
    listeners.clear();
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    document.body.innerHTML = "";
  });

  it("records a window-focused sample when the reopen path fires", () => {
    const raf = captureRaf();
    const input = document.createElement("input");
    document.body.appendChild(input);

    renderHook(() => useWorkspaceLifecycle(baseDeps({ inputRef: { current: input } })));

    const onFocused = listeners.get("window-focused");
    expect(onFocused).toBeTypeOf("function");

    onFocused!(undefined); // simulate Ctrl+K -> Focused(true) -> "window-focused"
    raf.run(); // flush the rAF the instrument scheduled

    const samples = summarizePerf();
    expect(samples).toHaveLength(1);
    expect(samples[0].reason).toBe("window-focused");
    expect(samples[0].dt).toBeGreaterThanOrEqual(0);
  });

  it("records nothing while in terminal mode (handler early-returns)", () => {
    const raf = captureRaf();
    const input = document.createElement("input");
    document.body.appendChild(input);

    renderHook(() =>
      useWorkspaceLifecycle(
        baseDeps({ modeRef: { current: "terminal" }, inputRef: { current: input } }),
      ),
    );

    listeners.get("window-focused")!(undefined);
    raf.run();

    expect(summarizePerf()).toHaveLength(0);
  });
});
