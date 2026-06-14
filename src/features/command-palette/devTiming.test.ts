import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  __resetPerfForTest,
  confirmInputReady,
  markPaletteOpen,
  summarizePerf,
  timingEnabled,
} from "./devTiming";

// Drive the requestAnimationFrame the util schedules by hand so the paint-time
// confirmation is deterministic.
function captureRaf(): { run: (now?: number) => void } {
  const cbs: FrameRequestCallback[] = [];
  vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
    cbs.push(cb);
    return cbs.length;
  });
  return {
    run: (now = performance.now()) => {
      const pending = cbs.splice(0);
      pending.forEach((cb) => cb(now));
    },
  };
}

function focusedInputRef(): {
  ref: React.RefObject<HTMLInputElement | null>;
  cleanup: () => void;
} {
  const input = document.createElement("input");
  document.body.appendChild(input);
  input.focus();
  return {
    ref: { current: input },
    cleanup: () => input.remove(),
  };
}

describe("devTiming", () => {
  beforeEach(() => {
    __resetPerfForTest();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = "";
  });

  it("is enabled under the test/dev build", () => {
    // Vitest runs with mode !== production, so import.meta.env.DEV is true.
    expect(timingEnabled()).toBe(true);
  });

  it("records a non-negative delta when the input is active at paint time", () => {
    const raf = captureRaf();
    const { ref, cleanup } = focusedInputRef();

    markPaletteOpen("window-focused");
    confirmInputReady("window-focused", ref);
    raf.run();

    const samples = summarizePerf();
    expect(samples).toHaveLength(1);
    expect(samples[0].reason).toBe("window-focused");
    expect(samples[0].dt).toBeGreaterThanOrEqual(0);

    cleanup();
  });

  it("exposes the ring buffer on window for devtools", () => {
    const raf = captureRaf();
    const { ref, cleanup } = focusedInputRef();

    markPaletteOpen("cold-mount");
    confirmInputReady("cold-mount", ref);
    raf.run();

    const buffer = (window as unknown as { __keynovaPerf?: unknown[] }).__keynovaPerf;
    expect(Array.isArray(buffer)).toBe(true);
    expect(buffer).toHaveLength(1);

    cleanup();
  });

  it("does not record when the input never became active", () => {
    const raf = captureRaf();
    const input = document.createElement("input");
    document.body.appendChild(input);
    // Deliberately do NOT focus it -> activeElement !== input.
    const ref: React.RefObject<HTMLInputElement | null> = { current: input };

    markPaletteOpen("window-focused");
    confirmInputReady("window-focused", ref);
    raf.run();

    expect(summarizePerf()).toHaveLength(0);
  });

  it("confirmInputReady is a safe no-op with no pending open", () => {
    const raf = captureRaf();
    const { ref, cleanup } = focusedInputRef();

    confirmInputReady("window-focused", ref);
    raf.run();

    expect(summarizePerf()).toHaveLength(0);
    cleanup();
  });

  it("a newer open supersedes a stale pending one (last-write-wins)", () => {
    const raf = captureRaf();
    const { ref, cleanup } = focusedInputRef();

    markPaletteOpen("cold-mount");
    // Second open arrives before the first confirms.
    markPaletteOpen("window-focused");
    confirmInputReady("window-focused", ref);
    raf.run();

    const samples = summarizePerf();
    expect(samples).toHaveLength(1);
    expect(samples[0].reason).toBe("window-focused");

    cleanup();
  });

  it("caps the ring buffer", () => {
    const raf = captureRaf();
    const { ref, cleanup } = focusedInputRef();

    for (let i = 0; i < 60; i += 1) {
      markPaletteOpen("window-focused");
      confirmInputReady("window-focused", ref);
      raf.run();
    }

    const buffer = (window as unknown as { __keynovaPerf?: unknown[] }).__keynovaPerf;
    expect(buffer?.length).toBeLessThanOrEqual(50);

    cleanup();
  });
});
