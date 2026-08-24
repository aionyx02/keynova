import { act, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { __resetPerfForTest, confirmInputReady, markPaletteOpen } from "./devTiming";
import { PerfBadge } from "./PerfBadge";

// Drive the rAF the timing util schedules by hand.
function captureRaf(): { run: () => void } {
  const cbs: FrameRequestCallback[] = [];
  vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
    cbs.push(cb);
    return cbs.length;
  });
  return { run: () => cbs.splice(0).forEach((cb) => cb(performance.now())) };
}

describe("PerfBadge", () => {
  beforeEach(() => __resetPerfForTest());

  afterEach(() => {
    vi.unstubAllGlobals();
    document.body.innerHTML = "";
  });

  it("shows the no-samples placeholder before any open", () => {
    const { getByText } = render(<PerfBadge />);
    expect(getByText("input-ready")).not.toBeNull();
    expect(getByText("—")).not.toBeNull();
    expect(getByText("no samples")).not.toBeNull();
  });

  it("re-renders with the latest delta when a sample lands", () => {
    const raf = captureRaf();
    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();

    const { getByText } = render(<PerfBadge />);

    act(() => {
      markPaletteOpen("window-focused");
      confirmInputReady("window-focused", { current: input });
      raf.run();
    });

    // last delta is rendered as "<n>ms" and the stat line shows the count.
    expect(getByText(/ms$/)).not.toBeNull();
    expect(getByText(/^med .* n=1$/)).not.toBeNull();
  });
});
