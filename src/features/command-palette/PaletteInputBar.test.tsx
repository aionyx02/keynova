import { createRef } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { PaletteInputBar } from "./PaletteInputBar";

function renderInputBar(query = "", mode: "search" | "command" = "search") {
  const onQueryChange = vi.fn();
  const view = render(
    <PaletteInputBar
      mode={mode}
      query={query}
      inputRef={createRef<HTMLInputElement>()}
      onQueryChange={onQueryChange}
      onKeyDown={vi.fn()}
      onFocus={vi.fn()}
      hasContentBelow={false}
    />,
  );
  return { ...view, onQueryChange };
}

describe("PaletteInputBar", () => {
  it("shows actionable command and terminal hints for an empty search", () => {
    const { onQueryChange } = renderInputBar();

    fireEvent.click(screen.getByRole("button", { name: "Type / for commands" }));
    expect(onQueryChange).toHaveBeenCalledWith("/");

    fireEvent.click(screen.getByRole("button", { name: "Type > for terminal" }));
    expect(onQueryChange).toHaveBeenCalledWith(">");
  });

  it("hides mode hints after input begins or command mode is active", () => {
    const { rerender } = renderInputBar("keynova");
    expect(screen.queryByRole("button", { name: "Type / for commands" })).toBeNull();

    rerender(
      <PaletteInputBar
        mode="command"
        query="/"
        inputRef={createRef<HTMLInputElement>()}
        onQueryChange={vi.fn()}
        onKeyDown={vi.fn()}
        onFocus={vi.fn()}
        hasContentBelow={false}
      />,
    );
    expect(screen.queryByRole("button", { name: "Type > for terminal" })).toBeNull();
  });
});
