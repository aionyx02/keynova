import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { invoke } from "@tauri-apps/api/core";
import { SettingPanel } from "./SettingPanel";
import type { SettingEntry, SettingSchema } from "./settingTypes";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const ENTRIES: SettingEntry[] = [
  { key: "hotkeys.app_launcher", value: "Ctrl+K", sensitive: false },
  // Backend already redacts secrets on list_all, so the panel receives "".
  { key: "ai.api_key", value: "", sensitive: true },
];

const SCHEMA: SettingSchema[] = [
  {
    key: "hotkeys.app_launcher",
    section: "hotkeys",
    label: "Launcher hotkey",
    value_type: "hotkey",
    default_value: "Ctrl+K",
    sensitive: false,
    options: [],
  },
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

/** The rail's auto-focus guard runs on a 50 ms timer; outlast it. */
const AFTER_AUTOFOCUS_TIMER = 120;

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

describe("SettingPanel section rail", () => {
  /**
   * Renders and waits for the *schema* to land, then settles the 50 ms
   * auto-focus timer.
   *
   * Waiting on a rail button is not enough: `Hotkeys` and `AI` are both in the
   * built-in `DEFAULT_SECTIONS` fallback too, so a button by that name resolves
   * before the fixture's two-section schema replaces it — and the rail would
   * then be arrowing through a different list. A row label only exists once the
   * real data is in.
   */
  async function renderLoadedPanel() {
    render(<SettingPanel />);
    await screen.findByText("Launcher hotkey");
    await new Promise((resolve) => setTimeout(resolve, AFTER_AUTOFOCUS_TIMER));
    return screen.getByRole("button", { name: "Hotkeys" });
  }

  it("switches section on ArrowDown and keeps focus on the rail", async () => {
    const hotkeys = await renderLoadedPanel();
    hotkeys.focus();
    fireEvent.keyDown(hotkeys, { key: "ArrowDown" });

    const ai = screen.getByRole("button", { name: "AI" });
    await waitFor(() => expect(ai.getAttribute("aria-current")).toBe("true"));
    expect(screen.getByText("Claude API key")).toBeTruthy();

    // The regression this locks in: an effect focuses the first row 50 ms after
    // the section changes, which would throw the caret out of the rail one
    // keypress into arrowing through sections.
    await new Promise((resolve) => setTimeout(resolve, AFTER_AUTOFOCUS_TIMER));
    expect(document.activeElement).toBe(ai);
  });

  it("moves focus into the rows on ArrowRight", async () => {
    const hotkeys = await renderLoadedPanel();
    const row = screen.getByDisplayValue("Ctrl+K");
    hotkeys.focus();
    fireEvent.keyDown(hotkeys, { key: "ArrowRight" });

    await waitFor(() => expect(document.activeElement).toBe(row));
    // Crossing into the rows does not change which section is selected.
    expect(hotkeys.getAttribute("aria-current")).toBe("true");
  });
});
