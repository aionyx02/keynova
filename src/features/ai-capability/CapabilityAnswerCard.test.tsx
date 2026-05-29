import { act, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { DispatchFn } from "../../context/IPCContext";
import { CapabilityAnswerCard } from "./CapabilityAnswerCard";

const NOOP_DISPATCH = vi.fn().mockResolvedValue(undefined) as unknown as DispatchFn;

const baseProps = {
  capabilityLabel: "explain" as const,
  status: "streaming" as const,
  text: "",
  error: null,
  startedAtMs: 1_000_000,
  firstChunkAtMs: null,
  completedAtMs: null,
  args: { text: "rust hashmap remove" },
  dispatch: NOOP_DISPATCH,
  onCancel: vi.fn(),
  onClose: vi.fn(),
};

describe("CapabilityAnswerCard", () => {
  it("renders the capability label and final latency in the header", () => {
    render(
      <CapabilityAnswerCard
        {...baseProps}
        status="complete"
        text="hello"
        startedAtMs={1000}
        firstChunkAtMs={1300}
        completedAtMs={1800}
      />,
    );
    // Final latency (completedAtMs - startedAtMs) = 1800 - 1000 = 800ms → 0.8s
    const header = screen.getByText(/Explain/);
    expect(header.textContent).toContain("0.8s");
  });

  it("renders the Fix label when capabilityLabel is fix", () => {
    render(
      <CapabilityAnswerCard
        {...baseProps}
        capabilityLabel="fix"
        status="complete"
        text="root cause: missing semicolon"
        startedAtMs={1000}
        firstChunkAtMs={1300}
        completedAtMs={1800}
      />,
    );
    expect(screen.getByText(/Fix/)).not.toBeNull();
  });

  it("renders pending placeholder when no chunk has arrived", () => {
    render(<CapabilityAnswerCard {...baseProps} status="pending" text="" />);
    expect(screen.getByText(/Asking model/i)).not.toBeNull();
  });

  it("renders a quiet ready prompt in idle state", () => {
    render(<CapabilityAnswerCard {...baseProps} status="idle" text="" />);
    expect(screen.getByText("Ready to ask.")).not.toBeNull();
    expect(screen.getByText("Ready")).not.toBeNull();
  });

  it("renders streaming text as markdown", async () => {
    render(<CapabilityAnswerCard {...baseProps} status="streaming" text="**bold** text" />);
    expect(await screen.findByText("bold")).not.toBeNull();
  });

  it("renders the cancelled body when status is cancelled", () => {
    render(<CapabilityAnswerCard {...baseProps} status="cancelled" text="partial" />);
    expect(screen.getByText("Cancelled.")).not.toBeNull();
    // Both header suffix and body contain "cancelled"; assert at least one.
    expect(screen.getAllByText(/cancelled/i).length).toBeGreaterThanOrEqual(1);
  });

  it("renders the error string in red when status is error", () => {
    render(<CapabilityAnswerCard {...baseProps} status="error" text="" error="backend offline" />);
    expect(screen.getByText("backend offline")).not.toBeNull();
    // Header suffix shows · error.
    expect(screen.getAllByText(/error/i).length).toBeGreaterThanOrEqual(1);
  });

  it("shows footer chips only when status is complete with text", () => {
    const { rerender } = render(
      <CapabilityAnswerCard {...baseProps} status="streaming" text="partial" />,
    );
    expect(screen.queryByRole("button", { name: /Copy md/i })).toBeNull();
    rerender(
      <CapabilityAnswerCard {...baseProps} status="complete" text="done" completedAtMs={1500} />,
    );
    expect(screen.getByRole("button", { name: /Copy md/i })).not.toBeNull();
    expect(screen.getByRole("button", { name: /Save to note/i })).not.toBeNull();
  });

  it("Copy md writes to clipboard", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText },
      configurable: true,
    });
    render(
      <CapabilityAnswerCard
        {...baseProps}
        status="complete"
        text="markdown body"
        completedAtMs={1500}
      />,
    );
    await act(async () => {
      fireEvent.mouseDown(screen.getByRole("button", { name: /Copy md/i }));
      await Promise.resolve();
    });
    expect(writeText).toHaveBeenCalledWith("markdown body");
  });

  it("Save to note dispatches note.save with auto-name and content", async () => {
    const dispatch = vi.fn().mockResolvedValue(undefined);
    render(
      <CapabilityAnswerCard
        {...baseProps}
        dispatch={dispatch as unknown as DispatchFn}
        status="complete"
        text="answer body"
        completedAtMs={1500}
        args={{ text: "rust hashmap remove returns Option<V>" }}
      />,
    );
    await act(async () => {
      fireEvent.mouseDown(screen.getByRole("button", { name: /Save to note/i }));
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(dispatch).toHaveBeenCalledWith("note.save", {
      name: "Explain: rust hashmap remove returns Option<V>",
      content: "answer body",
    });
  });

  it("truncates auto-name body to 40 chars", async () => {
    const dispatch = vi.fn().mockResolvedValue(undefined);
    const longBody = "a".repeat(60);
    render(
      <CapabilityAnswerCard
        {...baseProps}
        dispatch={dispatch as unknown as DispatchFn}
        capabilityLabel="summarize"
        status="complete"
        text="ok"
        completedAtMs={1500}
        args={{ text: longBody }}
      />,
    );
    await act(async () => {
      fireEvent.mouseDown(screen.getByRole("button", { name: /Save to note/i }));
      await Promise.resolve();
      await Promise.resolve();
    });
    const callArg = dispatch.mock.calls[0]![1] as { name: string };
    expect(callArg.name).toBe(`Summarize: ${"a".repeat(40)}`);
  });

  it("[×] calls onCancel while pending, onClose otherwise", () => {
    const onCancel = vi.fn();
    const onClose = vi.fn();
    const { rerender } = render(
      <CapabilityAnswerCard
        {...baseProps}
        status="pending"
        onCancel={onCancel}
        onClose={onClose}
      />,
    );
    fireEvent.mouseDown(screen.getByRole("button", { name: /Close/i }));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onClose).not.toHaveBeenCalled();

    rerender(
      <CapabilityAnswerCard
        {...baseProps}
        status="complete"
        text="done"
        completedAtMs={1500}
        onCancel={onCancel}
        onClose={onClose}
      />,
    );
    fireEvent.mouseDown(screen.getByRole("button", { name: /Close/i }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
