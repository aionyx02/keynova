import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CapabilityListCard } from "./CapabilityListCard";

const items = [
  {
    title: "/help",
    subtitle: "cmd.run · most recent · same context",
    route: "cmd.run",
    confidence: 0.92,
    rationale: "recent command usage in the current workspace context",
    last_executed_at: 1_748_500_000,
    workspace_id: 7,
    replay: {
      route: "cmd.run",
      payload: { name: "help", args: "" },
    },
    action_ref: undefined,
  },
  {
    title: "Open main.rs",
    subtitle: "action.run · recent · global history",
    route: "action.run",
    confidence: 0.54,
    rationale: "recent primary action",
    last_executed_at: 1_748_400_000,
    workspace_id: 7,
    replay: null,
    action_ref: undefined,
  },
];

const baseProps = {
  status: "complete" as const,
  items,
  error: null,
  startedAtMs: 1000,
  completedAtMs: 1200,
  selectedIndex: 0,
  onSelectIndex: vi.fn(),
  onRunSelected: vi.fn(),
  onCancel: vi.fn(),
  onClose: vi.fn(),
};

describe("CapabilityListCard", () => {
  it("renders workflow suggestions and replay labels", () => {
    render(<CapabilityListCard {...baseProps} />);
    expect(screen.getByRole("button", { name: /\/help/i })).not.toBeNull();
    expect(screen.getByRole("button", { name: /Open main\.rs/i })).not.toBeNull();
    expect(screen.getByText("Replay")).not.toBeNull();
    expect(screen.getByText("History only")).not.toBeNull();
  });

  it("runs replayable rows on mouse down", () => {
    const onRunSelected = vi.fn();
    const onSelectIndex = vi.fn();
    render(
      <CapabilityListCard
        {...baseProps}
        onRunSelected={onRunSelected}
        onSelectIndex={onSelectIndex}
      />,
    );
    fireEvent.mouseDown(screen.getByRole("button", { name: /\/help/i }));
    expect(onSelectIndex).toHaveBeenCalledWith(0);
    expect(onRunSelected).toHaveBeenCalledWith(0);
  });

  it("does not run history-only rows", () => {
    const onRunSelected = vi.fn();
    const onSelectIndex = vi.fn();
    render(
      <CapabilityListCard
        {...baseProps}
        onRunSelected={onRunSelected}
        onSelectIndex={onSelectIndex}
      />,
    );
    fireEvent.mouseDown(screen.getByRole("button", { name: /Open main.rs/i }));
    expect(onSelectIndex).toHaveBeenCalledWith(1);
    expect(onRunSelected).not.toHaveBeenCalled();
  });

  it("shows empty and pending states", () => {
    const { rerender } = render(<CapabilityListCard {...baseProps} items={[]} />);
    expect(screen.getByText(/No recent workflows yet/i)).not.toBeNull();
    rerender(<CapabilityListCard {...baseProps} status="pending" items={[]} completedAtMs={null} />);
    expect(screen.getByText(/Looking at recent workflows/i)).not.toBeNull();
  });
});
