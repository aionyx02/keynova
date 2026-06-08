import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CapabilityHintLine } from "./CapabilityHintLine";

describe("CapabilityHintLine", () => {
  it("renders nothing when visible=false", () => {
    const { container } = render(<CapabilityHintLine visible={false} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders quick-start prefix hints when visible=true", () => {
    render(<CapabilityHintLine visible={true} />);
    expect(screen.getByText("Quick starts")).not.toBeNull();
    expect(screen.getByText("Next")).not.toBeNull();
    expect(screen.getByText("Profile")).not.toBeNull();
    expect(screen.getByText("Command")).not.toBeNull();
    expect(screen.getByText("Remember")).not.toBeNull();
    expect(screen.getAllByRole("button")).toHaveLength(8);
  });

  it("renders args placeholder for prefixes with args", () => {
    render(<CapabilityHintLine visible={true} />);
    expect(screen.getByText("explain <question>")).not.toBeNull();
    expect(screen.getByText("cmd <intent>")).not.toBeNull();
  });

  it("fills the selected prefix when clicked", () => {
    const onPickPrefix = vi.fn();
    render(<CapabilityHintLine visible={true} onPickPrefix={onPickPrefix} />);
    // [0] next, [1] profile (zero-arg), [2] cmd.
    fireEvent.click(screen.getAllByRole("button")[1]);
    expect(onPickPrefix).toHaveBeenCalledWith("profile");
    fireEvent.click(screen.getAllByRole("button")[2]);
    expect(onPickPrefix).toHaveBeenCalledWith("cmd ");
  });
});
