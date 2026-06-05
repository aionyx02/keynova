import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { StarterActionsLine } from "./StarterActionsLine";

describe("StarterActionsLine", () => {
  it("renders nothing when hidden", () => {
    const { container } = render(<StarterActionsLine visible={false} onPickQuery={() => {}} />);
    expect(container.firstChild).toBeNull();
  });

  it("renders core starter actions", () => {
    render(<StarterActionsLine visible={true} onPickQuery={() => {}} />);
    expect(screen.getByText("Common entries")).not.toBeNull();
    expect(screen.getByText("Help")).not.toBeNull();
    expect(screen.getByText("Settings")).not.toBeNull();
    expect(screen.getByText("Notes")).not.toBeNull();
    expect(screen.getByText("Model")).not.toBeNull();
  });

  it("fills the selected command query", () => {
    const onPickQuery = vi.fn();
    render(<StarterActionsLine visible={true} onPickQuery={onPickQuery} />);
    fireEvent.click(screen.getByText("Settings"));
    expect(onPickQuery).toHaveBeenCalledWith("/setting");
  });
});
