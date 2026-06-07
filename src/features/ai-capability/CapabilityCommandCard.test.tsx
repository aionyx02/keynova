import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CapabilityCommandCard } from "./CapabilityCommandCard";

const baseProps = {
  status: "complete" as const,
  data: {
    command: "git push origin HEAD",
    confidence: 0.82,
    rationale: "Push the current branch to its origin remote.",
    assumptions: {
      cwd: "C:/work/keynova",
      shell: "powershell",
      os: "windows",
    },
  },
  error: null,
  startedAtMs: 1000,
  completedAtMs: 1400,
  intent: "push current branch to origin",
  riskRequiresConfirmation: true,
  sources: [
    {
      source_id: "workspace:1",
      source_type: "workspace",
      title: "Keynova",
      uri: null,
    },
  ],
  onCancel: vi.fn(),
  onClose: vi.fn(),
  onSubmit: vi.fn(),
};

describe("CapabilityCommandCard", () => {
  it("renders command, rationale, and risk guidance", () => {
    render(<CapabilityCommandCard {...baseProps} />);
    expect(screen.getByText("git push origin HEAD")).not.toBeNull();
    expect(screen.getByText(/Push the current branch/i)).not.toBeNull();
    expect(screen.getByText(/may change local system state/i)).not.toBeNull();
    expect(screen.getByText(/cwd=C:\/work\/keynova/)).not.toBeNull();
    expect(screen.getByTestId("capability-sources").textContent).toContain("Keynova");
  });

  it("shows idle guidance and generate button before submission", () => {
    render(<CapabilityCommandCard {...baseProps} status="idle" data={null} completedAtMs={null} />);
    expect(screen.getByText(/Ready to turn your intent/i)).not.toBeNull();
    expect(screen.getByRole("button", { name: /Generate/i })).not.toBeNull();
  });

  it("offers copy only and no execution controls", () => {
    render(<CapabilityCommandCard {...baseProps} />);
    expect(screen.getByRole("button", { name: /^Copy$/i })).not.toBeNull();
    expect(screen.queryByRole("button", { name: /Run/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /Edit before/i })).toBeNull();
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

  it("shows a low-risk label for read-only commands", () => {
    render(
      <CapabilityCommandCard
        {...baseProps}
        data={{
          command: "git status",
          confidence: 0.9,
          rationale: "Inspect repository state.",
          assumptions: {},
        }}
        riskRequiresConfirmation={false}
        sources={[]}
      />,
    );
    expect(screen.getByText(/Low-risk, read-only/i)).not.toBeNull();
    expect(screen.getByText(/were not provided/i)).not.toBeNull();
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
