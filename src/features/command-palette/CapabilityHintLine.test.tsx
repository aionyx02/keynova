import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { CapabilityHintLine } from "./CapabilityHintLine";

describe("CapabilityHintLine", () => {
  it("renders nothing when visible=false", () => {
    const { container } = render(<CapabilityHintLine visible={false} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders all five prefix hints when visible=true", () => {
    render(<CapabilityHintLine visible={true} />);
    expect(screen.getByText("explain")).not.toBeNull();
    expect(screen.getByText("summarize")).not.toBeNull();
    expect(screen.getByText("cmd")).not.toBeNull();
    expect(screen.getByText("fix")).not.toBeNull();
    expect(screen.getByText("next")).not.toBeNull();
  });

  it("renders args placeholder for prefixes with args", () => {
    render(<CapabilityHintLine visible={true} />);
    expect(screen.getByText("<question>")).not.toBeNull();
    expect(screen.getByText("<intent>")).not.toBeNull();
  });
});
