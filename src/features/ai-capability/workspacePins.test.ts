import { describe, expect, it } from "vitest";

import type { SuggestedNextAction } from "./types";
import { commandKeyOf, mergeProfileWithPins, PIN_MARKER, pinToSuggestion } from "./workspacePins";

const labels = { rationale: "Pinned", subtitle: "pinned command" };

function row(overrides: Partial<SuggestedNextAction> = {}): SuggestedNextAction {
  return {
    title: "/build",
    subtitle: "cmd.run · used 3× · 100% ok",
    route: "cmd.run",
    confidence: 0.7,
    rationale: "frequent in this project",
    last_executed_at: 1_748_000_000,
    workspace_id: 1,
    replay: { route: "cmd.run", payload: { name: "build", args: "" } },
    ...overrides,
  };
}

describe("workspacePins", () => {
  it("reconstructs a stable command key from a cmd.run row", () => {
    expect(commandKeyOf(row())).toBe("/build");
    expect(
      commandKeyOf(row({ replay: { route: "cmd.run", payload: { name: "deploy", args: "prod" } } })),
    ).toBe("/deploy prod");
  });

  it("returns null for non-replayable rows", () => {
    expect(commandKeyOf(row({ replay: null }))).toBeNull();
    expect(commandKeyOf(row({ replay: { route: "action.run", payload: {} } }))).toBeNull();
  });

  it("pinToSuggestion round-trips back to the same key", () => {
    const sug = pinToSuggestion("/deploy prod", labels);
    expect(sug).not.toBeNull();
    expect(sug?.title).toBe(`${PIN_MARKER}/deploy prod`);
    expect(sug?.confidence).toBe(1);
    expect(sug?.rationale).toBe("Pinned");
    expect(commandKeyOf(sug!)).toBe("/deploy prod");
  });

  it("ignores pins that are not /commands", () => {
    expect(pinToSuggestion("build", labels)).toBeNull();
    expect(pinToSuggestion("/", labels)).toBeNull();
  });

  it("prepends pins and drops the duplicate computed row", () => {
    const profile = [
      row({ title: "/build" }),
      row({
        title: "/test",
        replay: { route: "cmd.run", payload: { name: "test", args: "" } },
      }),
    ];
    const merged = mergeProfileWithPins(["/build"], profile, labels);
    // pinned /build first (marked), then only /test remains from the computed tail.
    expect(merged).toHaveLength(2);
    expect(merged[0].title).toBe(`${PIN_MARKER}/build`);
    expect(merged[0].confidence).toBe(1);
    expect(merged[1].title).toBe("/test");
  });

  it("returns the profile unchanged when there are no pins", () => {
    const profile = [row()];
    expect(mergeProfileWithPins([], profile, labels)).toBe(profile);
  });
});
