import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CapabilityCommandCard } from "./CapabilityCommandCard";

const baseProps = {
  status: "complete" as const,
  data: {
    command: "git push origin HEAD",
    confidence: 0.82,
    rationale: "Push the current branch to its origin remote.",
  },
  error: null,
  startedAtMs: 1000,
  completedAtMs: 1400,
  intent: "push current branch to origin",
  riskRequiresConfirmation: true,
  onCancel: vi.fn(),
  onClose: vi.fn(),
  onSubmit: vi.fn(),
  onRun: vi.fn(),
  onEditBefore: vi.fn(),
};

describe("CapabilityCommandCard", () => {
  it("renders command, rationale, and risk guidance", () => {
    render(<CapabilityCommandCard {...baseProps} />);
    expect(screen.getByText("git push origin HEAD")).not.toBeNull();
    expect(screen.getByText(/Push the current branch/i)).not.toBeNull();
    expect(screen.getByText(/may change local system state/i)).not.toBeNull();
  });

  it("shows idle guidance and generate button before submission", () => {
    render(<CapabilityCommandCard {...baseProps} status="idle" data={null} completedAtMs={null} />);
    expect(screen.getByText(/Ready to turn your intent/i)).not.toBeNull();
    expect(screen.getByRole("button", { name: /Generate/i })).not.toBeNull();
  });

  it("routes Run and Edit before buttons", () => {
    const onRun = vi.fn();
    const onEditBefore = vi.fn();
    render(<CapabilityCommandCard {...baseProps} onRun={onRun} onEditBefore={onEditBefore} />);
    fireEvent.mouseDown(screen.getByRole("button", { name: /Run/i }));
    fireEvent.mouseDown(screen.getByRole("button", { name: /Edit before/i }));
    expect(onRun).toHaveBeenCalledWith("git push origin HEAD");
    expect(onEditBefore).toHaveBeenCalledWith("git push origin HEAD");
  });

  it("copies the generated command", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
    render(<CapabilityCommandCard {...baseProps} />);
    await act(async () => {
      fireEvent.mouseDown(screen.getByRole("button", { name: /Copy/i }));
      await Promise.resolve();
    });
    expect(writeText).toHaveBeenCalledWith("git push origin HEAD");
  });

  it("uses cancel while pending and close after completion", () => {
    const onCancel = vi.fn();
    const onClose = vi.fn();
    const { rerender } = render(
      <CapabilityCommandCard
        {...baseProps}
        status="pending"
        data={null}
        completedAtMs={null}
        onCancel={onCancel}
        onClose={onClose}
      />,
    );
    fireEvent.mouseDown(screen.getByRole("button", { name: /Close/i }));
    expect(onCancel).toHaveBeenCalledTimes(1);

    rerender(<CapabilityCommandCard {...baseProps} onCancel={onCancel} onClose={onClose} />);
    fireEvent.mouseDown(screen.getByRole("button", { name: /Close/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
