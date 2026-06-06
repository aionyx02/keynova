import { describe, expect, it } from "vitest";

import { formatUpdateMessage, type UpdateStrings } from "./updater";

const strings: UpdateStrings = {
  available: "Update available: {version} (current {current}).",
  current: "You're on the latest version ({current}).",
  unconfigured: "Updates are not configured for this build yet.",
  error: "Update check failed: {message}",
};

describe("formatUpdateMessage", () => {
  it("renders an available update with version, current, and notes", () => {
    const text = formatUpdateMessage(
      { status: "available", version: "0.7.0", currentVersion: "0.6.0", notes: "Bug fixes" },
      strings,
    );
    expect(text).toContain("Update available: 0.7.0 (current 0.6.0).");
    expect(text).toContain("Bug fixes");
  });

  it("omits the notes block when there are none", () => {
    const text = formatUpdateMessage(
      { status: "available", version: "0.7.0", currentVersion: "0.6.0" },
      strings,
    );
    expect(text).toBe("Update available: 0.7.0 (current 0.6.0).");
  });

  it("reports the current version when up to date", () => {
    expect(formatUpdateMessage({ status: "current", currentVersion: "0.6.0" }, strings)).toBe(
      "You're on the latest version (0.6.0).",
    );
  });

  it("reports a friendly message when the updater is not configured", () => {
    expect(formatUpdateMessage({ status: "unconfigured" }, strings)).toBe(
      "Updates are not configured for this build yet.",
    );
  });

  it("surfaces the error message on failure", () => {
    expect(formatUpdateMessage({ status: "error", message: "network down" }, strings)).toBe(
      "Update check failed: network down",
    );
  });
});
