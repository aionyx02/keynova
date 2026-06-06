import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CommandResultArea } from "./CommandResultArea";

function renderInlineResult(text: string) {
  return render(
    <CommandResultArea
      exactCmd={null}
      isArgsPhase={false}
      cmdResult={{ text, ui_type: { type: "Inline" } }}
      terminalLaunchSpec={null}
      PanelComponent={null}
      panelKey="test"
      panelInitialArgs=""
      onTerminalCommandExit={vi.fn()}
      onPanelClose={vi.fn()}
      onPanelCommandResult={vi.fn()}
    />,
  );
}

describe("CommandResultArea", () => {
  it("renders long inline results in a keyboard-scrollable region", () => {
    renderInlineResult(Array.from({ length: 40 }, (_, index) => `/${index}`).join("\n"));

    const result = screen.getByRole("region", { name: "Command result" });
    expect(result.getAttribute("tabindex")).toBe("0");
    expect(result.classList.contains("max-h-[300px]")).toBe(true);
    expect(result.classList.contains("overflow-y-auto")).toBe(true);
    expect(result.classList.contains("overscroll-contain")).toBe(true);
  });

  it("copies inline command result text", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    renderInlineResult('{\n  "a": 1\n}');
    fireEvent.click(screen.getByLabelText("Copy result"));

    expect(writeText).toHaveBeenCalledWith('{\n  "a": 1\n}');
    await waitFor(() => expect(screen.getByLabelText("Copied result")).not.toBeNull());
  });
});
