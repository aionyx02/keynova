import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";

import { RankTooltip } from "./RankTooltip";
import type { ScoreBreakdown } from "../../types/search";

function anchor(): DOMRect {
  return {
    top: 100,
    left: 100,
    right: 200,
    bottom: 120,
    width: 100,
    height: 20,
    x: 100,
    y: 100,
    toJSON: () => ({}),
  } as DOMRect;
}

const zero: ScoreBreakdown = {
  base: 80,
  workspace_boost: 0,
  config_boost: 0,
  recency_boost: 0,
  frequency_boost: 0,
};

describe("RankTooltip (PRODUCT.1.A workspace term)", () => {
  it("renders the workspace row and folds the boost into the total", () => {
    render(<RankTooltip breakdown={{ ...zero, workspace_boost: 20 }} anchorRect={anchor()} visible />);
    expect(screen.getByText("workspace")).toBeTruthy();
    expect(screen.getByText("+20")).toBeTruthy();
    expect(screen.getByText("100")).toBeTruthy(); // 80 base + 20 workspace
  });

  it("hides the workspace/config rows when their boost is zero", () => {
    render(<RankTooltip breakdown={zero} anchorRect={anchor()} visible />);
    expect(screen.queryByText("workspace")).toBeNull();
    expect(screen.queryByText("config")).toBeNull();
  });
});
