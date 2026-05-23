// REF.6.A — InlineCapabilityReply unit tests.

import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { InlineCapabilityReply } from "./InlineCapabilityReply";

describe("InlineCapabilityReply", () => {
  it("shows Streaming… placeholder when loading and no text yet", () => {
    render(
      <InlineCapabilityReply text="" isLoading={true} error={null} onCancel={vi.fn()} />,
    );
    expect(screen.getByText("Streaming…")).not.toBeNull();
    expect(screen.getByRole("button", { name: /Cancel/i })).not.toBeNull();
  });

  it("renders accumulated streaming text", () => {
    render(
      <InlineCapabilityReply
        text="Hello there"
        isLoading={true}
        error={null}
        onCancel={vi.fn()}
      />,
    );
    expect(screen.getByText("Hello there")).not.toBeNull();
  });

  it("renders error message and surfaces it via the error label", () => {
    render(
      <InlineCapabilityReply
        text=""
        isLoading={false}
        error="backend offline"
        onCancel={vi.fn()}
      />,
    );
    expect(screen.getByText(/Explain · error/i)).not.toBeNull();
    expect(screen.getByText("backend offline")).not.toBeNull();
  });

  it("Cancel button fires onCancel", () => {
    const onCancel = vi.fn();
    render(
      <InlineCapabilityReply
        text="partial reply"
        isLoading={true}
        error={null}
        onCancel={onCancel}
      />,
    );
    fireEvent.mouseDown(screen.getByRole("button", { name: /Cancel/i }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("Close label replaces Cancel when not loading", () => {
    const onCancel = vi.fn();
    render(
      <InlineCapabilityReply
        text="done"
        isLoading={false}
        error={null}
        onCancel={onCancel}
      />,
    );
    expect(screen.getByRole("button", { name: /Close/i })).not.toBeNull();
  });
});
