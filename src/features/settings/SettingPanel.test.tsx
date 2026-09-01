import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { invoke } from "@tauri-apps/api/core";
import { SettingPanel } from "./SettingPanel";
import type { SettingEntry, SettingSchema } from "./settingTypes";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const ENTRIES: SettingEntry[] = [
  // Backend already redacts secrets on list_all, so the panel receives "".
  { key: "ai.api_key", value: "", sensitive: true },
];

const SCHEMA: SettingSchema[] = [
  {
    key: "ai.api_key",
    section: "ai",
    label: "Claude API key",
    value_type: "secret",
    default_value: "",
    sensitive: true,
    options: [],
  },
];

const mockInvoke = vi.mocked(invoke);

beforeEach(() => {
  (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
  mockInvoke.mockReset();
  mockInvoke.mockImplementation(async (_cmd: string, args?: unknown) => {
    const route = (args as { route?: string } | undefined)?.route;
    if (route === "setting.list_all") return ENTRIES as unknown;
    if (route === "setting.schema") return SCHEMA as unknown;
    if (route === "setting.set") return { ok: true } as unknown;
    return null as unknown;
  });
});

afterEach(() => {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
});

describe("SettingPanel secret handling", () => {
  it("does not retain the plaintext secret in the field after saving", async () => {
    render(<SettingPanel />);

    // The panel opens on `hotkeys`; the fixture only defines an `ai` key, so
    // reach the row the way a user does. (`initialArgs` is gone: settings is a
    // window now, and bare `/setting` — the only route to it — carries nothing.)
    fireEvent.click(await screen.findByRole("button", { name: "AI" }));

    const input = (await screen.findByPlaceholderText(
      "Enter a new secret value",
    )) as HTMLInputElement;
    expect(input.type).toBe("password");
    expect(input.value).toBe("");

    fireEvent.change(input, { target: { value: "sk-super-secret" } });
    expect(input.value).toBe("sk-super-secret");

    // Save via Enter; the backend receives the real value...
    fireEvent.keyDown(input, { key: "Enter" });
    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("cmd_dispatch", {
        route: "setting.set",
        payload: { key: "ai.api_key", value: "sk-super-secret" },
      }),
    );

    // ...but the renderer must revert the field to the masked/empty state so
    // no plaintext lingers in component state or the DOM value attribute.
    await waitFor(() => expect(input.value).toBe(""));
  });
});
