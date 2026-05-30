import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { SettingRow } from "./SettingRow";
import type { SettingSchema } from "./settingTypes";

function schema(overrides: Partial<SettingSchema>): SettingSchema {
  return {
    key: "ai.provider",
    section: "ai",
    label: "Provider",
    value_type: "string",
    default_value: "claude",
    sensitive: false,
    options: [],
    ...overrides,
  };
}

const handlers = () => ({
  registerRef: vi.fn(),
  onChange: vi.fn(),
  onSave: vi.fn(),
  onReset: vi.fn(),
  onBlur: vi.fn(),
  onKeyDown: vi.fn(),
});

describe("SettingRow", () => {
  it("renders an enum setting as a select with all options", () => {
    render(
      <SettingRow
        entry={{ key: "ai.provider", value: "claude" }}
        fieldSchema={schema({ options: ["claude", "ollama", "openai"] })}
        displayValue="claude"
        rowIdx={0}
        saving={false}
        saved={false}
        showSection={false}
        {...handlers()}
      />,
    );
    const select = screen.getByRole("combobox") as HTMLSelectElement;
    expect(select.value).toBe("claude");
    expect(screen.getByRole("option", { name: "ollama" })).not.toBeNull();
    expect(screen.getByRole("option", { name: "openai" })).not.toBeNull();
  });

  it("saves immediately when the select value changes", () => {
    const h = handlers();
    render(
      <SettingRow
        entry={{ key: "ai.provider", value: "claude" }}
        fieldSchema={schema({ options: ["claude", "ollama", "openai"] })}
        displayValue="claude"
        rowIdx={0}
        saving={false}
        saved={false}
        showSection={false}
        {...h}
      />,
    );
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "ollama" } });
    expect(h.onSave).toHaveBeenCalledWith("ai.provider", "ollama");
  });

  it("shows a reset control only when the value differs from default", () => {
    const h = handlers();
    const { rerender } = render(
      <SettingRow
        entry={{ key: "ai.provider", value: "claude" }}
        fieldSchema={schema({ options: ["claude", "ollama"] })}
        displayValue="claude"
        rowIdx={0}
        saving={false}
        saved={false}
        showSection={false}
        {...h}
      />,
    );
    expect(screen.queryByTitle(/Reset to default/i)).toBeNull();

    rerender(
      <SettingRow
        entry={{ key: "ai.provider", value: "ollama" }}
        fieldSchema={schema({ options: ["claude", "ollama"] })}
        displayValue="ollama"
        rowIdx={0}
        saving={false}
        saved={false}
        showSection={false}
        {...h}
      />,
    );
    const reset = screen.getByTitle(/Reset to default/i);
    fireEvent.click(reset);
    expect(h.onReset).toHaveBeenCalledWith("ai.provider", "claude");
  });

  it("never offers reset for secret fields", () => {
    render(
      <SettingRow
        entry={{ key: "ai.api_key", value: "", sensitive: true }}
        fieldSchema={schema({
          key: "ai.api_key",
          label: "Claude API key",
          value_type: "secret",
          default_value: "",
          sensitive: true,
        })}
        displayValue="sk-something"
        rowIdx={0}
        saving={false}
        saved={false}
        showSection={false}
        {...handlers()}
      />,
    );
    expect(screen.queryByTitle(/Reset to default/i)).toBeNull();
  });

  it("toggles a boolean setting via the switch", () => {
    const h = handlers();
    render(
      <SettingRow
        entry={{ key: "search.preview_enabled", value: "true" }}
        fieldSchema={schema({
          key: "search.preview_enabled",
          label: "Preview pane enabled",
          value_type: "boolean",
          default_value: "true",
          options: ["true", "false"],
        })}
        displayValue="true"
        rowIdx={0}
        saving={false}
        saved={false}
        showSection={false}
        {...h}
      />,
    );
    fireEvent.click(screen.getByRole("switch"));
    expect(h.onSave).toHaveBeenCalledWith("search.preview_enabled", "false");
  });
});
